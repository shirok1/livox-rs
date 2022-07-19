use std::error::Error;
use std::io::Cursor;
use std::mem::swap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use async_stream::try_stream;
use bytes::Bytes;
use nalgebra::*;
use tokio::net::{UdpSocket};
use tokio::{spawn, time};
use tokio_stream::StreamExt;
use tracing::{error, info, instrument, Instrument, Span, trace, warn};
use image::{ImageBuffer, Luma};
use livox_rs::model::*;
use livox_rs::model::data_type::*;
use livox_rs::model::data_type::AckGeneral::*;
use livox_rs::model::FrameData::{AckMsgFrame, CmdFrame};
use rdr_zeromq::server::EncodedImgServer;
use rdr_zeromq::traits::Server;
use livox_rs::Livox;


#[derive(std::fmt::Debug)]
enum MainError {
    Parse(ParseError),
    FailedHandshake,
    Io(std::io::Error),
    NoneBroadcastReceived,
    ZeroMQ(rdr_zeromq::ZmqError),
}

impl std::fmt::Display for MainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Error for MainError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

const BROADCAST_LISTEN_PORT: u16 = 55000;
const COMMAND_SOCKET_PORT: u16 = 1157;
const DATA_LISTEN_PORT: u16 = 7731;

const DEPTH_GRAPH_SERVER_ENDPOINT: &str = "tcp://0.0.0.0:8100";

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;
    let client = Livox::wait_for_one().await?
        .handshake(Ipv4Addr::new(192, 168, 1, 50), COMMAND_SOCKET_PORT, DATA_LISTEN_PORT).await?;
    client.set_sampling(true).await?;

    let ds = client.get_ds();

    let mut img_server = EncodedImgServer::new(DEPTH_GRAPH_SERVER_ENDPOINT).await;

    let pc_stream = listen_matrix_stream_of_socket(ds.as_ref()).await?;
    tokio::pin!(pc_stream);

    let mut img = image::GrayImage::new(3072, 2048);
    let mut backup_img = img.clone();
    let mut img_bytes: Vec<u8> = Vec::new();

    let mut count = 0;

    let calib_mat = calculate_calib_mat();

    while let Some(pc) = pc_stream.next().await {
        // info!("DOTS!!!!!!!!!");
        match pc {
            Err(err) => warn!("Error happened when parsing data: {}", err),
            Ok(pc) => {
                let start_time = time::Instant::now();

                let not_unified_pixel_with_depth = calib_mat * pc;

                let points = not_unified_pixel_with_depth.column_iter().map(|p| Vector3::new(p.x / p.z, p.y / p.z, p.z)).filter(|p| {
                    0.0 < p.x && p.x < 3072.0 && 0.0 < p.y && p.y < 2048.0
                }).collect::<Vec<_>>();

                let i = points.len();
                draw_points(&mut img, points.as_ref());
                draw_points(&mut backup_img, points.as_ref());
                // info!("{} pixels in range, used time {}ms", i, start_time.elapsed().as_millis());


                count += 1;
                if count == 1000 {
                    count = 0;
                    let start_time = time::Instant::now();
                    img.write_to(&mut Cursor::new(&mut img_bytes), image::ImageOutputFormat::Bmp)?;
                    info!("Bitmap size: {}kb, encoding used {}ms", img_bytes.len() / 1024, start_time.elapsed().as_millis());
                    let start_time = time::Instant::now();
                    img_server.send_img(Bytes::copy_from_slice(&img_bytes[..])).await?;
                    info!("Send image used time {}ms", start_time.elapsed().as_millis());
                    img.fill(0);
                    swap(&mut img, &mut backup_img);
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
#[tracing::instrument]
async fn main_old() -> Result<(), Box<dyn Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let (broadcast_data, lidar) = {
        let broadcast_receiver = UdpSocket::bind(
            (Ipv4Addr::UNSPECIFIED, BROADCAST_LISTEN_PORT)).await?;
        // broadcast_receiver.set_broadcast(true)?;
        let mut buf = [0u8; 4096];
        info!("Waiting for broadcast on {}", BROADCAST_LISTEN_PORT);
        let (size, lidar) = broadcast_receiver.recv_from(&mut buf).await?;
        info!("Received {} bytes from {}...", size, lidar);
        let ControlFrame { data, .. } = ControlFrame::parse(&buf[..size])?;
        (data, lidar)
    };

    let (broadcast_code, dev_type) = {
        if let AckMsgFrame(Acknowledge::General(BroadcastMessage {
                                                    broadcast_code, dev_type, reserved: _
                                                })) = broadcast_data {
            (broadcast_code, dev_type)
        } else { Err(MainError::NoneBroadcastReceived)? }
    };

    match std::str::from_utf8(&broadcast_code[..broadcast_code.len() - 1]) {
        Ok(str_code) => info!("LiDAR broadcast code: {}", str_code),
        Err(err) => warn!("Error parsing broadcast code {:?}: {}", broadcast_data, err),
    }
    match dev_type {
        6 => info!("Yes, it is a Mid-70 (dev_type: 7)"),
        _ => warn!("Unknown device type ({})!", dev_type),
    };

    let command_socket = UdpSocket::bind(
        (Ipv4Addr::UNSPECIFIED, COMMAND_SOCKET_PORT)).await?;

    handshake(lidar, &command_socket).await?;
    start_sampling(lidar, &command_socket).await?;
    spawn_heartbeat(lidar, command_socket);

    let mut img_server = EncodedImgServer::new(DEPTH_GRAPH_SERVER_ENDPOINT).await;

    let pc_stream = listen_matrix_stream(DATA_LISTEN_PORT).await?;
    tokio::pin!(pc_stream);

    let mut img = image::GrayImage::new(3072, 2048);
    let mut backup_img = img.clone();
    let mut img_bytes: Vec<u8> = Vec::new();

    let mut count = 0;

    let calib_mat = calculate_calib_mat();

    while let Some(pc) = pc_stream.next().await {
        match pc {
            Err(err) => warn!("Error happened when parsing data: {}", err),
            Ok(pc) => {
                let start_time = time::Instant::now();

                let not_unified_pixel_with_depth = calib_mat * pc;

                let points = not_unified_pixel_with_depth.column_iter().map(|p| Vector3::new(p.x / p.z, p.y / p.z, p.z)).filter(|p| {
                    0.0 < p.x && p.x < 3072.0 && 0.0 < p.y && p.y < 2048.0
                }).collect::<Vec<_>>();

                let i = points.len();
                draw_points(&mut img, points.as_ref());
                draw_points(&mut backup_img, points.as_ref());
                info!("{} pixels in range, used time {}ms", i, start_time.elapsed().as_millis());


                count += 1;
                if count == 1000 {
                    count = 0;
                    let start_time = time::Instant::now();
                    img.write_to(&mut Cursor::new(&mut img_bytes), image::ImageOutputFormat::Bmp)?;
                    info!("img_bytes size: {}, used time {}ms", img_bytes.len(), start_time.elapsed().as_millis());
                    let start_time = time::Instant::now();
                    img_server.send_img(Bytes::copy_from_slice(&img_bytes[..])).await?;
                    info!("send_img used time {}ms", start_time.elapsed().as_millis());
                    img.fill(0);
                    swap(&mut img, &mut backup_img);
                }
            }
        }
    }

    Ok(())
}

fn draw_points(img: &mut ImageBuffer<Luma<u8>, Vec<u8>>, points: &[Vector3<f32>]) {
    for p in points {
        let x = p.x;
        let y = p.y;
        let luma = p.z / 100.0;
        img.put_pixel(x as u32, y as u32, Luma([luma as u8]));

        for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)].iter() {
            let x1 = (x + dx) as u32;
            let y1 = (y + dy) as u32;
            if x1 >= 3072 || y1 >= 2048 { continue; }
            let old_luma = &mut img.get_pixel_mut(x1, y1).0;
            old_luma[0] = ((old_luma[0] as f32 + luma) / 2.0) as u8;
        }
    }
}

/*fn calibrate() -> (IntrinsicParametersPerspective<f32>, ExtrinsicParameters<f32>) {
    let in_param = IntrinsicParametersPerspective::from(PerspectiveParams {
        fx: 2580.7380664637653,
        fy: 2582.8839945792183,
        skew: 0.0,
        cx: 1535.9830165125002,
        cy: 1008.784910706948,
    });

    // let rot_vec = Vector3::new(1.41418, 1.33785, -1.03742);
    // let rot = UnitQuaternion::from_axis_angle(&Unit::new_normalize(rot_vec), rot_vec.norm());

    let rot_mat = Matrix3::new(0.0185759, -0.999824, 0.00251985,
                               0.0174645, -0.00219543, -0.999845,
                               0.999675, 0.018617, 0.0174206);
    let rot = Rotation3::from_matrix(&rot_mat);
    let rot_q = UnitQuaternion::from_matrix(
        &rot_mat
    );

    let (axis, angle) = rot.axis_angle().unwrap();
    let rot_vec = axis.scale(angle);
    // warn!("axis: {:?}, angle: {}", axis, angle);
    // warn!("rvec = {}", rot_vec);


    let trans_vec = Point3::new(-0.0904854, -0.132904, -0.421934);
    let cam_center = rot_q.inverse().transform_point(&-trans_vec);
    // warn!("cam_center = {}", cam_center);
    // let ex_param = ExtrinsicParameters::from_rotation_and_camcenter(rot, -trans_vec);
    let ex_param = ExtrinsicParameters::from_rotation_and_camcenter(rot_q, cam_center);
    // let ex_param = ExtrinsicParameters::from_rotation_and_camcenter(UnitQuaternion::identity(), Point3::origin());
    // let ex_param = Matrix3x4::new(0.0185759, -0.999824, 0.00251985, -0.0904854,
    //                               0.0174645,-0.00219543, );
    // warn!("ex:{}", ex_param.matrix());
    // Camera::new(in_param, ex_param)
    (in_param, ex_param)
}*/

fn calculate_calib_mat() -> Matrix3x4<f32> {
    let ex_mat = Matrix3x4::new(0.0185759, -0.999824, 0.00251985, -0.0904854,
                                0.0174645, -0.00219543, -0.999675, -0.132904,
                                0.999675, 0.018617, 0.0174206, -0.421934);
    let in_mat = Matrix3::new(2580.7380664637653, 0.0, 1535.9830165125002,
                              0.0, 2582.8839945792183, 1008.784910706948,
                              0.0, 0.0, 1.0);
    in_mat * ex_mat
}

#[instrument(fields(cmd_port = % cmd_port.local_addr().unwrap().port()))]
async fn start_sampling(lidar: SocketAddr, cmd_port: &UdpSocket) -> Result<(), MainError> {
    let start_sampling = ControlFrame {
        version: 1,
        data: CmdFrame(Command::General(CmdGeneral::StartStopSampling {
            sample_ctrl: 1
        })),
        seq_num: 0,
    };
    cmd_port.send_to(start_sampling.serialize().as_ref(), lidar).await.map_err(MainError::Io)?;
    Ok(())
}

#[instrument(fields(cmd_port = % cmd_port.local_addr().unwrap().port()))]
async fn start_sampling2(lidar: SocketAddr, cmd_port: &UdpSocket) -> Result<(), MainError> {
    let start_sampling = ControlFrame {
        version: 1,
        data: CmdFrame(Command::General(CmdGeneral::StartStopSampling {
            sample_ctrl: 1
        })),
        seq_num: 0,
    };
    cmd_port.send_to(start_sampling.serialize().as_ref(), lidar).await.map_err(MainError::Io)?;
    Ok(())
}

#[instrument(fields(cmd_port = % cmd_port.local_addr().unwrap().port()))]
async fn handshake(lidar: SocketAddr, cmd_port: &UdpSocket) -> Result<(), MainError> {
    let handshake = ControlFrame {
        version: 1,
        data: CmdFrame(Command::General(CmdGeneral::Handshake {
            user_ip: [192, 168, 1, 50],
            data_port: DATA_LISTEN_PORT,
            cmd_port: COMMAND_SOCKET_PORT,
            imu_port: 0,
        })),
        seq_num: 69,
    };

    let size = cmd_port.send_to(handshake.serialize().as_ref(), lidar).await.map_err(MainError::Io)?;
    info!("Sent {} bytes of handshake", size);

    let mut buf = [0u8; 4096];

    cmd_port.recv(&mut buf).await.map_err(MainError::Io)?;

    let handshake_ack = ControlFrame::parse(&buf[..size]).map_err(MainError::Parse)?;
    if AckMsgFrame(Acknowledge::General(Handshake { ret_code: 0 })) == handshake_ack.data {
        info!("Handshake OK");
    } else { Err(MainError::FailedHandshake)?; }
    Ok(())
}

#[instrument(skip(cmd_port))]
fn spawn_heartbeat(lidar: SocketAddr, cmd_port: UdpSocket) {
    spawn(async move {
        let mut interval = time::interval(Duration::from_millis(500));
        let start_time = interval.tick().await;
        loop {
            let time = interval.tick().await;
            let heartbeat = ControlFrame {
                version: 1,
                data: CmdFrame(Command::General(CmdGeneral::Heartbeat {})),
                seq_num: (time.duration_since(start_time).as_millis() / 500) as u16,
            }.serialize();
            info!("Heartbeat sending: {}s", time.duration_since(start_time).as_secs_f32());
            if let Ok(size) = cmd_port.send_to(
                heartbeat.as_ref(), lidar).await {
                if size == heartbeat.len() {
                    trace!("Heartbeat sent");
                } else {
                    warn!("Heartbeat sent but size is wrong ({})", size);
                }
            } else {
                error!("Heartbeat sending failed");
            }
        }
    }.instrument(Span::current()));
}

#[instrument(skip(cmd_port))]
fn spawn_heartbeat2(cmd_port: Arc<UdpSocket>) {
    spawn(async move {
        let mut interval = time::interval(Duration::from_millis(500));
        let start_time = interval.tick().await;
        loop {
            let time = interval.tick().await;
            let heartbeat = ControlFrame {
                version: 1,
                data: CmdFrame(Command::General(CmdGeneral::Heartbeat {})),
                seq_num: (time.duration_since(start_time).as_millis() / 500) as u16,
            }.serialize();
            info!("Heartbeat sending: {}s", time.duration_since(start_time).as_secs_f32());
            if let Ok(size) = cmd_port.send(heartbeat.as_ref()).await {
                if size == heartbeat.len() {
                    trace!("Heartbeat sent");
                } else {
                    warn!("Heartbeat sent but size is wrong ({})", size);
                }
            } else {
                error!("Heartbeat sending failed");
            }
        }
    }.instrument(Span::current()));
}

#[instrument]
async fn listen_matrix_stream(port: u16) -> tokio::io::Result<impl tokio_stream::Stream<Item=Result<SMatrix<f32, 4, 96>, Box<dyn Error>>>> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).await?;

    let mut buf = [0u8; 2048];

    let span = Span::current();

    Ok(try_stream! {
        while let size = socket.recv(&mut buf).await? {
            span.in_scope(|| {
                trace!("received data pack of size {}", size);
            });
            let points = PointCloudFrame::parse_augmented_matrix(&buf[..size]);
            yield points;
        }
    })
}

#[instrument]
async fn listen_matrix_stream_of_socket(socket: &UdpSocket) -> tokio::io::Result<impl tokio_stream::Stream<Item=Result<SMatrix<f32, 4, 96>, Box<dyn Error + '_>>>> {
    let mut buf = [0u8; 2048];

    // let span = Span::current();

    Ok(try_stream! {
        while let size = socket.recv(&mut buf).await? {
            // span.in_scope(|| {
            //     info!("received data pack of size {}", size);
            // });
            let points = PointCloudFrame::parse_augmented_matrix(&buf[..size]);
            yield points;
        }
    })
}
