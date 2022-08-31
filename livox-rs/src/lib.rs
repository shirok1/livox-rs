use std::time::Duration;
use std::error::Error;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use async_stream::try_stream;
use nalgebra::SMatrix;
use tokio::{select, spawn};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{error, info, info_span, instrument, Instrument, warn};

use crate::LivoxError::{BadResponse, ParseError};
use crate::model::{ControlFrame, FrameData};
use crate::model::deku_data_type::{ExtractError, general, MessageData, RequestData, ResponseData};
use crate::result_util::ToLivoxResult;


pub mod model;

#[cfg(test)]
mod test;

/// Represents a Livox device.
/// See [Livox SDK Communication Protocol](https://github.com/Livox-SDK/Livox-SDK/wiki/Livox-SDK-Communication-Protocol#0x00-broadcast-message) for more information.
#[derive(Debug)]
pub struct Livox {
    /// UDP socket address of the Livox device for commands, port should always be 65000.
    /// (Note: Data transmissions are not from the same socket port as the command transmission.)
    pub lidar_addr: SocketAddr,
    /// Device broadcast code, 15 capital letters or digits with a trailing '\0'
    pub broadcast_code: [u8; 16],
    /// Device type
    pub device_type: DeviceType,
}

/// Livox device type.
#[derive(Debug)]
#[repr(u8)]
pub enum DeviceType {
    /// Livox Mid-70 (0x06)
    Mid70 = 6,
    NotImplemented = 255,
}

/// Error types in [`Livox`] and [`LivoxClient`].
#[derive(Debug)]
pub enum LivoxError {
    IoError(&'static str, std::io::Error),
    ParseError(model::ParseError),
    NoneBroadcastReceived,
    HandshakeFailed(Livox),
    AckFailed(u8),
    AckWrong(ResponseData),
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

mod result_util;

/// A asynchronous Livox command task.
#[derive(Debug)]
pub struct AsyncCommandTask {
    command: RequestData,
    callback: oneshot::Sender<LivoxResult<ResponseData>>,
}

pub struct HandshakeOption {
    user_ip: Ipv4Addr,
    cmd_port: u16,
    data_port: u16,
}

impl Default for HandshakeOption {
    fn default() -> Self {
        HandshakeOption {
            user_ip: Ipv4Addr::new(192, 168, 1, 50),
            cmd_port: 0,
            data_port: 0,
        }
    }
}

impl Livox {
    /// The port host should listen on for broadcast.
    pub const BROADCAST_LISTEN_PORT: u16 = 55000;

    /// Find a Livox device by listening on UDP port 55000.
    /// Follow steps described in
    /// [Livox SDK Communication Protocol](https://github.com/Livox-SDK/Livox-SDK/wiki/Livox-SDK-Communication-Protocol#23-sdk-connection).
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
            if let FrameData::Message(MessageData::General(
                                                 general::message::Enum::BroadcastMessage(
                                                     general::message::BroadcastMessage {
                                                         broadcast_code, dev_type, reserved: _
                                                     }))) = data {
                (broadcast_code, dev_type)
            } else { Err(NoneBroadcastReceived)? }
        };

        match std::str::from_utf8(&broadcast_code[..broadcast_code.len() - 1]) {
            Ok(str_code) => info!("LiDAR broadcast code: {}", str_code),
            Err(err) => warn!("Error parsing broadcast code {:?}: {}", broadcast_code, err),
        }

        Ok(Livox {
            lidar_addr,
            broadcast_code,
            device_type: match dev_type {
                x if x == (DeviceType::Mid70 as u8) => {
                    info!("Yes, it is a Mid-70 (dev_type: 7)");
                    DeviceType::Mid70
                }
                _ => {
                    warn!("Unknown device type ({})!", dev_type);
                    DeviceType::NotImplemented
                }
            },
        })
    }

    /// Try to send handshake message to this Livox device.
    /// Returns a [`LivoxClient`] if handshake succeeded.
    #[instrument(skip(self, option), fields(lidar = % self.lidar_addr))]
    pub async fn handshake(self, option: HandshakeOption) -> LivoxResult<LivoxClient> {
        use LivoxError::*;
        let command_socket = UdpSocket::bind(
            (Ipv4Addr::UNSPECIFIED, option.cmd_port))
            .await.err_reason("While creating command socket")?;
        let cmd_port = command_socket.local_addr().unwrap().port();
        info!("Command port bind to {}", cmd_port);

        command_socket.connect(self.lidar_addr).await.err_reason("While connecting socket to LiDAR")?;

        let data_socket = UdpSocket::bind(
            (Ipv4Addr::UNSPECIFIED, option.data_port)).await.err_reason("While creating data socket")?;
        let data_port = data_socket.local_addr().unwrap().port();
        info!("Data port bind to {}", data_port);
        // data_socket.connect(self.lidar_addr).await.err_reason("While connecting socket to LiDAR")?;

        let handshake = ControlFrame {
            version: 1,
            data: FrameData::Request(general::request::Handshake {
                user_ip: option.user_ip.octets(),
                data_port,
                cmd_port,
                imu_port: 0,
            }.into()),
            seq_num: 0,
        };

        let sent_size = command_socket.send(handshake.serialize().as_ref())
            .await.err_reason("While sending handshake")?;
        info!("Sent {} bytes of handshake", sent_size);

        let mut buf = [0u8; 1024];
        let size = command_socket.recv(&mut buf)
            .await.err_reason("While receiving handshake")?;

        let handshake_ack = ControlFrame::parse(&buf[..size]).map_err(ParseError)?;

        if let FrameData::Response(response) = handshake_ack.data {
            if let Ok(general::response::Handshake { ret_code: 0 }) = response.try_into() {
                info!("Handshake OK");

                let (task_channel, task_receiver) = mpsc::channel::<AsyncCommandTask>(128);
                let task_thread = LivoxClient::spawn_task_thread(command_socket, task_receiver);

                let (heartbeat_stop, heartbeat_rx) = oneshot::channel();
                let heartbeat_thread = LivoxClient::spawn_heartbeat(task_channel.clone(), heartbeat_rx);

                return Ok(LivoxClient {
                    lidar: self,
                    task_channel,
                    task_thread,
                    heartbeat_stop,
                    heartbeat_thread,
                    data_socket: Arc::new(data_socket),
                });
            }
        }

        Err(HandshakeFailed(self))
    }
}

/// A client of a Livox LiDAR.
/// Should only be created with [`Livox::handshake`].
#[derive(Debug)]
pub struct LivoxClient {
    /// The LiDAR this client is connected to.
    pub lidar: Livox,
    task_channel: mpsc::Sender<AsyncCommandTask>,
    task_thread: JoinHandle<()>,
    heartbeat_stop: oneshot::Sender<()>,
    heartbeat_thread: JoinHandle<()>,
    data_socket: Arc<UdpSocket>,
}

impl LivoxClient {
    const HEARTBEAT_PERIOD: Duration = Duration::from_millis(750);

    async fn send_command_to_channel(channel: &mpsc::Sender<AsyncCommandTask>, command: impl Into<RequestData>) -> LivoxResult<ResponseData> {
        let (callback, task) = oneshot::channel::<LivoxResult<ResponseData>>();
        channel.send(AsyncCommandTask { command: command.into(), callback }).await
            .err_reason("While sending command")?;
        task.await.err_reason("While waiting for command response")?
    }

    fn spawn_task_thread(command_socket: UdpSocket, mut task_receiver: mpsc::Receiver<AsyncCommandTask>) -> JoinHandle<()> {
        spawn(async move {
            let mut seq_num = 0;
            let mut buf = [0u8; 1024];
            while let Some(AsyncCommandTask { command, callback }) = task_receiver.recv().await {
                seq_num += 1;
                let frame = ControlFrame {
                    version: 1,
                    data: FrameData::Request(command),
                    seq_num,
                };

                let callback = |result: LivoxResult<ResponseData>| {
                    if let Err(data) = callback.send(result) {
                        error!("Synchronized sender callback failed! {:?}", data)
                    }
                };

                let _sent_size = match command_socket.send(frame.serialize().as_ref())
                    .await.err_reason("While sending command") {
                    Ok(size) => size,
                    Err(err) => {
                        callback(Err(err));
                        continue;
                    }
                };
                // info!("Sent {} bytes of command", _sent_size);

                let recv_size = match command_socket.recv(&mut buf).await.err_reason("While receiving command") {
                    Ok(size) => size,
                    Err(err) => {
                        callback(Err(err));
                        continue;
                    }
                };
                let ack = match ControlFrame::parse(&buf[..recv_size]).map_err(ParseError) {
                    Ok(ControlFrame { data: FrameData::Response(ack), .. }) => ack,
                    Ok(ControlFrame { .. }) => {
                        callback(Err(BadResponse(frame.data)));
                        continue;
                    }
                    Err(err) => {
                        callback(Err(err));
                        continue;
                    }
                };
                callback(Ok(ack));
            }
            warn!("Task thread exited");
        }.instrument(info_span!("command synchronized sender")))
    }

    // #[instrument]
    fn spawn_heartbeat(channel: mpsc::Sender<AsyncCommandTask>, stop_signal: oneshot::Receiver<()>) -> JoinHandle<()> {
        use general::*;

        spawn(async move {
            let mut interval = interval(LivoxClient::HEARTBEAT_PERIOD);
            let start_time = interval.tick().await;
            // let stop_signal = stop_signal;
            tokio::pin!(stop_signal);
            loop {
                select! {
                _ = interval.tick() => {
                    let ack = LivoxClient::send_command_to_channel(&channel, request::Heartbeat{}).await.unwrap().try_into();
                    if matches!(ack, Ok(response::Heartbeat { ret_code: 0,.. })) {
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

    /// Send a command to the LiDAR.
    /// See [`CmdGeneral`] and [`CmdLiDAR`] for available commands.
    pub async fn send_command(&self, command: impl Into<RequestData>) -> LivoxResult<ResponseData> {
        Self::send_command_to_channel(&self.task_channel, command).await
    }

    /// Start or stop sampling.
    /// [Protocol Definition](https://github.com/Livox-SDK/Livox-SDK/wiki/Livox-SDK-Communication-Protocol#0x04-startstop-sampling)
    #[instrument]
    pub async fn set_sampling(&self, start: bool) -> Result<(), LivoxError> {
        use LivoxError::*;
        use general::*;

        let command = request::StartStopSampling {
            sample_ctrl: if start { 1 } else { 0 }
        };

        let ack = self.send_command(command).await?;
        match ack.try_into() {
            Ok(response::StartStopSampling { ret_code: 0 }) => Ok(()),
            Ok(response::StartStopSampling { ret_code }) => Err(AckFailed(ret_code)),
            // ack if matches!(ack, Acknowledge::General(AckGeneral::StartStopSampling { .. })) => Err(AckFailed(ack)),
            Err(ExtractError::WrongCommand(c)) => Err(AckWrong(c.into())),
            Err(ExtractError::WrongCommandSet(any)) => Err(AckWrong(any)),
        }
    }

    /// Get a async stream of homogeneous matrix of LiDAR data.
    /// Each point is presented by a `Vector4<f32>`, with `1` as its 4th component.
    pub fn homogeneous_matrix_stream(&self) -> impl tokio_stream::Stream<Item=LivoxResult<SMatrix<f32, 4, 96>>> {
        use model::PointCloudFrame;

        let socket = self.data_socket.clone();
        let mut buf = [0u8; 2048];

        try_stream! {
            while let size = socket.recv(&mut buf).await.err_reason("While reading point cloud frame")? {
                yield PointCloudFrame::parse_homogeneous_matrix(&buf[..size]);
            }
        }
    }
}

