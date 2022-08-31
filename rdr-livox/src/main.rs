use std::error::Error;
use std::io::Cursor;
use std::mem::swap;
use std::net::{Ipv4Addr};
use std::sync::Arc;
use bytes::Bytes;
use image::{ImageBuffer, Luma};
use nalgebra::*;
use tokio::{time};
use tokio_stream::StreamExt;
use tracing::{info, warn};

use rdr_zeromq::prelude::{Message, Timestamp};
use rdr_zeromq::prelude::lidar::{DepthPixel, LiDARDepthPixels, LiDARRawPoints, RawPoint};
use rdr_zeromq::server::{EncodedImgServer, LiDARServer};
use rdr_zeromq::traits::Server;
use livox_rs::Livox;


const COMMAND_SOCKET_PORT: u16 = 1157;
const DATA_LISTEN_PORT: u16 = 7731;

const DEPTH_GRAPH_SERVER_ENDPOINT: &str = "tcp://0.0.0.0:8100";
const DEPTH_PIXELS_SERVER_ENDPOINT: &str = "tcp://0.0.0.0:8200";

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;
    let client = Livox::wait_for_one().await?
        .handshake(Ipv4Addr::new(192, 168, 1, 50), COMMAND_SOCKET_PORT, DATA_LISTEN_PORT).await?;
    client.set_sampling(true).await?;

    let calib_mat = calculate_calib_mat();


    let mut pc_server = LiDARServer::new(DEPTH_PIXELS_SERVER_ENDPOINT).await;


    let img_server = Arc::new(tokio::sync::Mutex::new(EncodedImgServer::new(DEPTH_GRAPH_SERVER_ENDPOINT).await));

    let pc_stream = client.homogeneous_matrix_stream();
    tokio::pin!(pc_stream);

    let mut img = image::GrayImage::new(3072, 2048);
    let mut backup_img = img.clone();

    let mut count = 0;

    let mut saving = Some((time::Instant::now(), LiDARRawPoints::new()));


    while let Some(pc) = pc_stream.next().await {
        match pc {
            Err(err) => warn!("Error happened when parsing data: {}", err),
            Ok(pc) => {
                // Save raw point cloud
                if let Some((start, ref mut msg)) = saving {
                    let point_iter = pc.column_iter().map(|p| RawPoint { x: p.x, y: p.y, z: p.z, ..RawPoint::default() });
                    msg.points.extend(point_iter);

                    if start.elapsed().as_secs() >= 10 {
                        msg.timestamp = Some(Timestamp::now()).into();
                        match std::fs::File::create("point_cloud.buf") {
                            Ok(mut file) => {
                                match msg.write_to_writer(&mut file) {
                                    Ok(_) => info!("Successfully cached 10s of pc."),
                                    Err(err) => warn!("Cache 10s failed! {:?}", err),
                                }
                            }
                            Err(err) => warn!("Failed to open file to cache! {:?}", err),
                        }
                        saving = None;
                    }
                }

                let not_unified_pixel_with_depth = calib_mat * pc;

                let points = not_unified_pixel_with_depth.column_iter().map(|p| ((p.x / p.z) as i32, (p.y / p.z) as i32, p.z))
                    .filter(|(x, y, _)| in_box(3072, 2048)(x, y)).collect::<Vec<_>>();

                // Send points
                {
                    let msg = LiDARDepthPixels {
                        timestamp: Some(Timestamp::now()).into(),
                        pixels: points.iter().map(|(x, y, z)| {
                            DepthPixel {
                                x: *x,
                                y: *y,
                                z: *z,
                                ..DepthPixel::default()
                            }
                        }).collect(),
                        ..LiDARDepthPixels::default()
                    };
                    pc_server.send(&msg).await?;
                }

                // Draw depth graph
                {
                    // let i = points.len();
                    draw_points(&mut img, points.as_ref());
                    draw_points(&mut backup_img, points.as_ref());
                    // info!("{} pixels in range, used time {}ms", i, start_time.elapsed().as_millis());

                    count += 1;
                    if count == 1000 {
                        count = 0;
                        let img_clone = img.clone();
                        let img_server = img_server.clone();

                        tokio::spawn(async move {
                            let start_time = time::Instant::now();
                            let img_bytes = tokio_rayon::spawn(move || {
                                let mut img_bytes: Vec<u8> = Vec::new();
                                img_clone.write_to(&mut Cursor::new(&mut img_bytes), image::ImageOutputFormat::Bmp).map(|()| img_bytes)
                            }).await.unwrap();
                            info!("Bitmap size: {}kb, encoding used {}ms", img_bytes.len() / 1024, start_time.elapsed().as_millis());
                            let start_time = time::Instant::now();
                            img_server.lock().await.send_img(Bytes::copy_from_slice(&img_bytes[..])).await.unwrap();
                            info!("Send image used time {}ms", start_time.elapsed().as_millis());
                        });

                        img.fill(0);
                        swap(&mut img, &mut backup_img);
                    }
                }
            }
        }
    }
    Ok(())
}

fn draw_points(img: &mut ImageBuffer<Luma<u8>, Vec<u8>>, points: &[(i32, i32, f32)]) {
    for (x, y, z) in points {
        let luma = z / 100.0;
        img.put_pixel(*x as u32, *y as u32, Luma([luma as u8]));

        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)].iter() {
            let x1 = x + dx;
            let y1 = y + dy;
            if in_box(3072, 2048)(&x1, &y1) {
                let old_luma = &mut img.get_pixel_mut(x1 as u32, y1 as u32).0;
                old_luma[0] = ((old_luma[0] as f32 + luma) / 2.0) as u8;
            }
        }
    }
}

fn in_box(w: i32, h: i32) -> impl Fn(&i32, &i32) -> bool {
    move |x: &i32, y: &i32| (0..w).contains(x) && (0..h).contains(y)
}

fn calculate_calib_mat() -> Matrix3x4<f32> {
    let ex_mat = Matrix3x4::new(0.0185759, -0.999824, 0.00251985, -0.0904854,
                                0.0174645, -0.00219543, -0.999675, -0.132904,
                                0.999675, 0.018617, 0.0174206, -0.421934);
    let in_mat = Matrix3::new(2580.7380664637653, 0.0, 1535.9830165125002,
                              0.0, 2582.8839945792183, 1008.784910706948,
                              0.0, 0.0, 1.0);
    in_mat * ex_mat
}
