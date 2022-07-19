use std::error::Error;
use std::net::Ipv4Addr;
use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use std::time::Duration;
use tokio::{select, spawn};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{error, info, info_span, instrument, Instrument, Span, trace, warn};
use crate::LivoxError::{BadResponse, ParseError};
use crate::model::{Acknowledge, Command, ControlFrame, FrameData};
use crate::model::data_type::*;
// use crate::model::data_type::prelude::*;
use crate::model::FrameData::{AckMsgFrame, CmdFrame};
use crate::result_util::ToLivoxResult;

pub mod model;
#[cfg(test)]
mod test;

#[derive(Debug)]
pub struct Livox {
    lidar_addr: SocketAddr,
    broadcast_code: [u8; 16],
    device_type: DeviceType,
}

#[derive(Debug)]
#[repr(u8)]
pub enum DeviceType {
    Mid70 = 6,
    NotImplemented = 255,
}

#[derive(Debug)]
pub enum LivoxError {
    IoError(&'static str, std::io::Error),
    ParseError(model::ParseError),
    NoneBroadcastReceived,
    HandshakeFailed(Livox),
    AckFailed(Acknowledge),
    AckWrong(Acknowledge),
    BadResponse(FrameData),
    AsyncChannelError(&'static str, mpsc::error::SendError<AsyncCommandTask>),
    AsyncCallbackError(&'static str, oneshot::error::RecvError),
}

impl std::fmt::Display for LivoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Error for LivoxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub type LivoxResult<T> = Result<T, LivoxError>;

#[derive(Debug)]
pub struct AsyncCommandTask {
    command: Command,
    callback: oneshot::Sender<LivoxResult<Acknowledge>>,
}

mod result_util;

impl Livox {
    pub const BROADCAST_LISTEN_PORT: u16 = 55000;

    #[instrument]
    pub async fn wait_for_one() -> LivoxResult<Self> {
        use LivoxError::*;
        let broadcast_receiver = UdpSocket::bind(
            (Ipv4Addr::UNSPECIFIED, Livox::BROADCAST_LISTEN_PORT))
            .await.err_reason("While creating broadcast socket")?;
        let mut buf = [0u8; 1024];

        info!("Waiting for broadcast on {}", Livox::BROADCAST_LISTEN_PORT);
        let (size, lidar_addr) = broadcast_receiver.recv_from(&mut buf)
            .await.err_reason("While receiving broadcast")?;
        info!("Received {} bytes from {}...", size, lidar_addr);

        let ControlFrame { data, .. } = ControlFrame::parse(&buf[..size]).map_err(ParseError)?;

        let (broadcast_code, dev_type) = {
            if let AckMsgFrame(
                Acknowledge::General(
                    AckGeneral::BroadcastMessage {
                        broadcast_code, dev_type, reserved: _
                    })) = data {
                (broadcast_code, dev_type)
            } else { Err(NoneBroadcastReceived)? }
        };

        Ok(Livox {
            lidar_addr,
            broadcast_code,
            device_type: match dev_type {
                x if x == (DeviceType::Mid70 as u8) => DeviceType::Mid70,
                _ => DeviceType::NotImplemented,
            },
        })
    }

    #[instrument(skip(self, user_ip, cmd_port, data_port), fields(lidar = % self.lidar_addr))]
    pub async fn handshake(self, user_ip: Ipv4Addr, cmd_port: u16, data_port: u16) -> LivoxResult<LivoxClient> {
        use LivoxError::*;
        let command_socket = UdpSocket::bind(
            (Ipv4Addr::UNSPECIFIED, cmd_port))
            .await.err_reason("While creating command socket")?;
        command_socket.connect(self.lidar_addr).await.err_reason("While connecting socket to LiDAR")?;

        let data_socket = UdpSocket::bind(
            (Ipv4Addr::UNSPECIFIED, data_port)).await.err_reason("While creating data socket")?;
        // data_socket.connect(self.lidar_addr).await.err_reason("While connecting socket to LiDAR")?;

        let handshake = ControlFrame {
            version: 1,
            data: CmdFrame(Command::General(CmdGeneral::Handshake {
                user_ip: user_ip.octets(),
                data_port,
                cmd_port,
                imu_port: 0,
            })),
            seq_num: 0,
        };

        let sent_size = command_socket.send(handshake.serialize().as_ref())
            .await.err_reason("While sending handshake")?;
        info!("Sent {} bytes of handshake", sent_size);

        let mut buf = [0u8; 1024];
        let size = command_socket.recv(&mut buf)
            .await.err_reason("While receiving handshake")?;

        let handshake_ack = ControlFrame::parse(&buf[..size]).map_err(ParseError)?;

        let (task_channel, task_receiver) = mpsc::channel::<AsyncCommandTask>(128);

        let task_thread = LivoxClient::spawn_task_thread(command_socket, task_receiver);

        let (heartbeat_stop, heartbeat_rx) = oneshot::channel();

        let heartbeat_thread = LivoxClient::spawn_heartbeat(task_channel.clone(), heartbeat_rx);

        if AckMsgFrame(Acknowledge::General(AckGeneral::Handshake { ret_code: 0 })) == handshake_ack.data {
            info!("Handshake OK");
            Ok(LivoxClient {
                lidar: self,
                task_channel,
                task_thread,
                heartbeat_stop,
                heartbeat_thread,
                data_socket: Arc::new(data_socket),
            })
        } else { Err(HandshakeFailed(self)) }
    }
}

#[derive(Debug)]
pub struct LivoxClient {
    lidar: Livox,
    task_channel: mpsc::Sender<AsyncCommandTask>,
    task_thread: JoinHandle<()>,
    heartbeat_stop: oneshot::Sender<()>,
    heartbeat_thread: JoinHandle<()>,
    data_socket: Arc<UdpSocket>,
}

impl LivoxClient {
    const HEARTBEAT_PERIOD: Duration = Duration::from_millis(750);

    pub fn get_ds(&self) -> Arc<UdpSocket> {
        self.data_socket.clone()
    }

    async fn send_command_to_channel(channel: &mpsc::Sender<AsyncCommandTask>, command: Command) -> LivoxResult<Acknowledge> {
        let (callback, task) = oneshot::channel::<LivoxResult<Acknowledge>>();
        channel.send(AsyncCommandTask { command, callback }).await
            .err_reason("While sending command")?;
        task.await.err_reason("While waiting for command response")?
    }
    fn spawn_task_thread(command_socket: UdpSocket, mut task_receiver: Receiver<AsyncCommandTask>) -> JoinHandle<()> {
        spawn(async move {
            let mut seq_num = 0;
            let mut buf = [0u8; 1024];
            while let Some(AsyncCommandTask { command, callback }) = task_receiver.recv().await {
                seq_num += 1;
                let frame = ControlFrame {
                    version: 1,
                    data: CmdFrame(command),
                    seq_num,
                };
                let sent_size = match command_socket.send(frame.serialize().as_ref())
                    .await.err_reason("While sending command") {
                    Ok(size) => size,
                    Err(err) => {
                        match callback.send(Err(err)) {
                            Err(data) => error!("Synchronized sender callback failed! {:?}", data),
                            _ => {}
                        }
                        continue;
                    }
                };
                // info!("Sent {} bytes of command", sent_size);
                let recv_size = match command_socket.recv(&mut buf).await.err_reason("While receiving command") {
                    Ok(size) => size,
                    Err(err) => {
                        match callback.send(Err(err)) {
                            Err(data) => error!("Synchronized sender callback failed! {:?}", data),
                            _ => {}
                        }
                        continue;
                    }
                };
                // info!("Received {} bytes of acknowledge", recv_size);
                // let frame = match ControlFrame::parse(&buf[..recv_size]).map_err(ParseError) {
                //     Ok(frame) => frame,
                //     Err(err) => {
                //         match callback.send(Err(err)) {
                //             Err(data) => error!("Synchronized sender callback failed! {:?}", data),
                //             _ => {}
                //         }
                //         continue;
                //     }
                // };
                // let ack = match frame.data {
                //     CmdFrame(_) => {
                //         match callback.send(Err(BadResponse(frame.data))) {
                //             Err(data) => error!("Synchronized sender callback failed! {:?}", data),
                //             _ => {}
                //         }
                //         continue;
                //     }
                //     AckMsgFrame(ack) => ack,
                // };
                let ack = match ControlFrame::parse(&buf[..recv_size]).map_err(ParseError) {
                    Ok(ControlFrame { data: AckMsgFrame(ack), .. }) => ack,
                    Ok(ControlFrame { data: CmdFrame(_), .. }) => {
                        match callback.send(Err(BadResponse(frame.data))) {
                            Err(data) => error!("Synchronized sender callback failed! {:?}", data),
                            _ => {}
                        }
                        continue;
                    }
                    Err(err) => {
                        match callback.send(Err(err)) {
                            Err(data) => error!("Synchronized sender callback failed! {:?}", data),
                            _ => {}
                        }
                        continue;
                    }
                };
                callback.send(Ok(ack)).expect("Callback should be safe");
            }
            warn!("Task thread exited");
        }.instrument(info_span!("command synchronized sender")))
    }

    // #[instrument]
    fn spawn_heartbeat(channel: mpsc::Sender<AsyncCommandTask>, stop_signal: oneshot::Receiver<()>) -> JoinHandle<()> {
        spawn(async move {
            let mut interval = interval(LivoxClient::HEARTBEAT_PERIOD);
            let start_time = interval.tick().await;
            // let stop_signal = stop_signal;
            tokio::pin!(stop_signal);
            loop {
                select! {
                _ = interval.tick() => {
                    let ack = LivoxClient::send_command_to_channel(&channel, Command::General(CmdGeneral::Heartbeat {})).await.unwrap();
                    if matches!(ack, Acknowledge::General(AckGeneral::Heartbeat { ret_code: 0, .. })) {
                        info!("Heartbeat OK @ {}ms", start_time.elapsed().as_millis());
                    } else {
                        error!("Heartbeat failed @ {}ms: {:?}", start_time.elapsed().as_millis(), ack);
                    }
                }
                _ = &mut stop_signal => { break; }
            }
            }
        }.instrument(info_span!("heartbeat")))
    }

    pub async fn send_command(&self, command: Command) -> LivoxResult<Acknowledge> {
        Self::send_command_to_channel(&self.task_channel, command).await
    }

    #[instrument]
    pub async fn set_sampling(&self, start: bool) -> Result<(), LivoxError> {
        use LivoxError::*;
        let command = Command::General(CmdGeneral::StartStopSampling {
            sample_ctrl: if start { 1 } else { 0 }
        });

        let ack = self.send_command(command).await?;
        match ack {
            Acknowledge::General(AckGeneral::StartStopSampling { ret_code: 0 }) => Ok(()),
            ack if matches!(ack, Acknowledge::General(AckGeneral::StartStopSampling { .. })) => Err(AckFailed(ack)),
            any => Err(AckWrong(any)),
        }
    }
}

