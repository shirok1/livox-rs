use std::cmp::max;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use byte_struct::*;
use bytes::{Buf, BufMut, BytesMut};
use crc::{Algorithm, Crc};
use nalgebra::{Matrix, OMatrix, Point3, RowVector3, SMatrix, Vector3, Vector4};

use tracing::{debug, warn};
use crate::model::ParseError::{InvalidCommandType, InvalidCrc16, InvalidCrc32, InvalidData, InvalidLength, InvalidSOF, InvalidVersion, WrongPointCloudSize};

use data_type::*;

const HEADER_CHECKSUM_ALGORITHM: Algorithm<u16> = Algorithm { init: 0x4c49u16.reverse_bits(), ..crc::CRC_16_MCRF4XX };
const FRAME_CHECKSUM_ALGORITHM: Algorithm<u32> = Algorithm { init: !0x564f580au32.reverse_bits(), ..crc::CRC_32_ISO_HDLC };
const CRC16: Crc<u16> = Crc::<u16>::new(&HEADER_CHECKSUM_ALGORITHM);
const CRC32: Crc<u32> = Crc::<u32>::new(&FRAME_CHECKSUM_ALGORITHM);

#[derive(PartialEq, Debug)]
pub struct ControlFrame {
    //	Protocol Version, 1 for The Current Version
    pub version: u8,

    // cmd_type & data
    pub data: FrameData,

    // Frame Sequence Number
    pub seq_num: u16,
}

#[derive(PartialEq, Eq, Debug)]
pub enum ParseError {
    InvalidSOF,
    InvalidVersion,
    InvalidLength,
    InvalidCrc16 { frame: u16, calculated: u16 },
    InvalidCrc32,
    InvalidCommandType,
    InvalidData,
    WrongPointCloudSize,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl ControlFrame {
    const SOF: u8 = 0xAA;
    #[tracing::instrument]
    pub fn parse(frame: &[u8]) -> Result<ControlFrame, ParseError> {
        if frame[0] != ControlFrame::SOF { return Err(InvalidSOF); }

        // if frame[1] != VERSION { return Err(InvalidVersion); }

        let len = frame[2] as usize;
        // if frame[2] != len as u8 { return Err(InvalidLength); }


        let frame_crc16 = u16::from_le_bytes([frame[7], frame[8]]);
        let calculated_crc16 = CRC16.checksum(&frame[..7]);
        if frame_crc16 != calculated_crc16 {
            warn!("Invalid CRC16 checksum! In frame: {:04x} Calculated: {:04x}", frame_crc16, calculated_crc16);
            return Err(InvalidCrc16 { frame: frame_crc16, calculated: calculated_crc16 });
        } else { debug!("CRC16 checksum: {:04x}", calculated_crc16); }

        let calculated_crc32 = CRC32.checksum(&frame[..len - 4]);
        let frame_crc32 = u32::from_le_bytes([frame[len - 4], frame[len - 3], frame[len - 2], frame[len - 1]]);
        if frame_crc32 != calculated_crc32 {
            warn!("Invalid CRC32 checksum! In frame: {:08x} Calculated: {:08x}", frame_crc32, calculated_crc32);
            return Err(InvalidCrc32);
        } else { debug!("CRC32 checksum: {:08x}", calculated_crc32); }

        Ok(ControlFrame {
            version: frame[1],
            data: match frame[4] {
                0x00 /*CMD*/ => FrameData::CmdFrame(Command::parse(&frame[9..len - 4])?),
                0x01 /*ACK*/ => FrameData::AckMsgFrame(Acknowledge::parse(&frame[9..len - 4])?),
                0x02 /*MSG*/ => FrameData::AckMsgFrame(Acknowledge::parse(&frame[9..len - 4])?),
                _ => Err(InvalidCommandType)?
            },
            seq_num: u16::from_le_bytes([frame[5], frame[6]]),
        })
    }
    pub fn serialize(&self) -> BytesMut {
        let data: Box<[u8]> = match &self.data {
            FrameData::CmdFrame(Command::General(cmd_general)) => {
                let mut dataframe = Box::new([0u8; CmdGeneral::MAX_BYTE_LEN]);
                cmd_general.write_bytes(dataframe.as_mut_slice());
                dataframe
            }
            FrameData::CmdFrame(Command::LiDAR(cmd_lidar)) => {
                let mut dataframe = Box::new([0u8; CmdLiDAR::MAX_BYTE_LEN]);
                cmd_lidar.write_bytes(dataframe.as_mut_slice());
                dataframe
            }
            FrameData::AckMsgFrame(Acknowledge::General(ack_general)) => {
                let mut dataframe = Box::new([0u8; AckGeneral::MAX_BYTE_LEN]);
                ack_general.write_bytes(dataframe.as_mut_slice());
                dataframe
            }
            FrameData::AckMsgFrame(Acknowledge::LiDAR(ack_lidar)) => {
                let mut dataframe = Box::new([0u8; AckLiDAR::MAX_BYTE_LEN]);
                ack_lidar.write_bytes(dataframe.as_mut_slice());
                dataframe
            }
            FrameData::AckMsgFrame(Acknowledge::Hub()) | FrameData::CmdFrame(Command::Hub()) => todo!("no hub")
        };


        let mut frame = BytesMut::with_capacity(data.len() + 14);

        frame.put_u8(ControlFrame::SOF);
        frame.put_u8(self.version);
        frame.put_u16_le(data.len() as u16 + 14);
        frame.put_u8(match &self.data {
            FrameData::CmdFrame(_) => 0x00,
            FrameData::AckMsgFrame(_) => 0x01
        });
        frame.put_u16_le(self.seq_num);
        frame.put_u16_le(CRC16.checksum(&frame[..frame.remaining()]));
        frame.put_u8(
            match &self.data {
                FrameData::CmdFrame(Command::General(_)) | FrameData::AckMsgFrame(Acknowledge::General(_)) => 0x00,
                _ => 0x01,
                /*hub*/
            }
        );
        frame.put(&data[..]);
        frame.put_u32_le(CRC32.checksum(&frame[..frame.remaining()]));
        frame
    }
}

#[derive(PartialEq, Debug)]
pub enum FrameData {
    CmdFrame(Command),
    AckMsgFrame(Acknowledge),
    // MSG,
}


mod parsing;

mod command;
pub mod data_type;

#[derive(PartialEq, Debug)]
pub enum Command {
    General(CmdGeneral),
    LiDAR(CmdLiDAR),
    Hub(),
}

impl Command {
    pub fn parse(data: &[u8]) -> Result<Command, ParseError> {
        match data[0] {
            0x00 => Ok(Command::General(CmdGeneral::read_bytes(&data[1..]))),
            0x01 => Ok(Command::LiDAR(CmdLiDAR::read_bytes(&data[1..]))),
            0x02 => Ok(Command::Hub()),
            _ => Err(InvalidCommandType),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Acknowledge {
    General(AckGeneral),
    LiDAR(AckLiDAR),
    Hub(),
}

impl Acknowledge {
    pub fn parse(data: &[u8]) -> Result<Acknowledge, ParseError> {
        match data[0] {
            0x00 => Ok(Acknowledge::General(AckGeneral::read_bytes(&data[1..]))),
            0x01 => Ok(Acknowledge::LiDAR(AckLiDAR::read_bytes(&data[1..]))),
            0x02 => Ok(Acknowledge::Hub()),
            _ => Err(InvalidCommandType),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct PointCloudFrame {
    pub version: u8,

    pub slot_id: u8,

    pub lidar_id: u8,

    pub status_code: LiDARStatusCode,

    pub timestamp_type: u8,

    pub timestamp: u64,

    pub data: PointCloudFrameData,
}

#[derive(PartialEq, Debug)]
pub enum PointCloudFrameData {
    DT2(Box<[DT2; 96]>),
    DT3(Box<[DT3; 96]>),
}

impl PointCloudFrameData {
    pub fn extract_points(&self) -> Vec<Point3<i32>> {
        match self {
            PointCloudFrameData::DT2(data) => data.iter()
                .map(DT2::to_point).collect::<Vec<_>>(),
            _ => todo!("not implemented"),
            // PointCloudFrameData::DT3(data) => data.iter().flat_map(|dt3| dt3.points()).collect(),
        }
    }
}

impl PointCloudFrame {
    pub fn parse(frame: &[u8]) -> Result<PointCloudFrame, ParseError> {
        Ok(PointCloudFrame {
            version: frame[0],
            slot_id: frame[1],
            lidar_id: frame[2],
            status_code: LiDARStatusCode::read_bytes_default_le(&frame[4..8]),
            // timestamp_type: frame[4],
            timestamp_type: 0,
            timestamp: 0,
            // timestamp: u64::from_le_bytes([frame[5], frame[6], frame[7], frame[8]]),
            data: match frame[9] {
                0x02 => PointCloudFrameData::DT2(<Box<[DT2; 96]>>::try_from(frame[18..].chunks(DT2::BYTE_LEN).map(DT2::read_bytes_default_le).collect::<Vec<DT2>>().into_boxed_slice()).map_err(|_| WrongPointCloudSize)?),
                0x03 => PointCloudFrameData::DT3(<Box<[DT3; 96]>>::try_from(frame[18..].chunks(DT3::BYTE_LEN).map(DT3::read_bytes_default_le).collect::<Vec<DT3>>().into_boxed_slice()).map_err(|_| WrongPointCloudSize)?),
                _ => return Err(InvalidData),
            },
        })
    }
    /*pub fn parse_row_matrix(frame: &[u8]) -> SMatrix<f32, 96, 3> {
        assert_eq!(frame[9], 0x02);
        let vec = frame[18..].chunks(DT2::BYTE_LEN).map(|d| RowVector3::new(
            i32::read_bytes_default_le(&d[0..4]) as f32,
            i32::read_bytes_default_le(&d[4..8]) as f32,
            i32::read_bytes_default_le(&d[8..12]) as f32)).collect::<Vec<_>>();
        SMatrix::<f32, 96, 3>::from_rows(vec.as_slice())
    }
    pub fn parse_matrix(frame: &[u8]) -> SMatrix::<f32, 3, 96> {
        assert_eq!(frame[9], 0x02);
        let vec = frame[18..].chunks(DT2::BYTE_LEN).map(|d| Vector3::new(
            i32::read_bytes_default_le(&d[0..4]) as f32,
            i32::read_bytes_default_le(&d[4..8]) as f32,
            i32::read_bytes_default_le(&d[8..12]) as f32)).collect::<Vec<_>>();
        SMatrix::<f32, 3, 96>::from_columns(vec.as_slice())
    }*/
    pub fn parse_augmented_matrix(frame: &[u8]) -> SMatrix::<f32, 4, 96> {
        assert_eq!(frame[9], 0x02);
        let vec = frame[18..].chunks(DT2::BYTE_LEN).map(|d| Vector4::new(
            i32::read_bytes_default_le(&d[0..4]) as f32,
            i32::read_bytes_default_le(&d[4..8]) as f32,
            i32::read_bytes_default_le(&d[8..12]) as f32, 1.0)).collect::<Vec<_>>();
        SMatrix::<f32, 4, 96>::from_columns(vec.as_slice())
    }
}
