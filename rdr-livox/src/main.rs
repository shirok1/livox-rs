use std::error::Error;
use std::io::Cursor;
use std::mem::swap;
use std::net::{Ipv4Addr};
use bytes::Bytes;
use image::{ImageBuffer, Luma};
use nalgebra::*;
use tokio::{time};
use tokio_stream::StreamExt;
use tracing::{info, warn};

use rdr_zeromq::server::EncodedImgServer;
use rdr_zeromq::traits::Server;
use livox_rs::Livox;


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

    let mut img_server = EncodedImgServer::new(DEPTH_GRAPH_SERVER_ENDPOINT).await;

    let pc_stream = client.homogeneous_matrix_stream();
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
                let not_unified_pixel_with_depth = calib_mat * pc;

                let points = not_unified_pixel_with_depth.column_iter().map(|p| Vector3::new(p.x / p.z, p.y / p.z, p.z)).filter(|p| {
                    0.0 < p.x && p.x < 3072.0 && 0.0 < p.y && p.y < 2048.0
                }).collect::<Vec<_>>();

                // let i = points.len();
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

fn calculate_calib_mat() -> Matrix3x4<f32> {
    let ex_mat = Matrix3x4::new(0.0185759, -0.999824, 0.00251985, -0.0904854,
                                0.0174645, -0.00219543, -0.999675, -0.132904,
                                0.999675, 0.018617, 0.0174206, -0.421934);
    let in_mat = Matrix3::new(2580.7380664637653, 0.0, 1535.9830165125002,
                              0.0, 2582.8839945792183, 1008.784910706948,
                              0.0, 0.0, 1.0);
    in_mat * ex_mat
}
