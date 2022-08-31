use deku::prelude::*;
use crate::model::traits::{Request, Response};

pub mod general;
pub mod lidar;

pub trait Parsable<'a>: Sized + DekuContainerRead<'a> + DekuWrite {
    fn parse(input: &'a [u8]) -> Result<Self, DekuError> {
        let ((_rest, rest_size), val) =
            Self::from_bytes((input, 0))?;
        if rest_size != 0 { tracing::warn!("Some data left not handled by deku!"); }
        Ok(val)
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
pub enum RequestData {
    #[deku(id = "0x00")] General(general::request::Enum),
    #[deku(id = "0x01")] LiDAR(lidar::request::Enum),
}

impl From<general::request::Enum> for RequestData {
    fn from(value: general::request::Enum) -> Self {
        Self::General(value)
    }
}

impl From<lidar::request::Enum> for RequestData {
    fn from(value: lidar::request::Enum) -> Self {
        Self::LiDAR(value)
    }
}

impl Parsable<'_> for RequestData {}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
pub enum ResponseData {
    #[deku(id = "0x00")] General(general::response::Enum),
    #[deku(id = "0x01")] LiDAR(lidar::response::Enum),
}

impl Parsable<'_> for ResponseData {}

impl From<general::response::Enum> for ResponseData {
    fn from(value: general::response::Enum) -> Self {
        Self::General(value)
    }
}

impl From<lidar::response::Enum> for ResponseData {
    fn from(value: lidar::response::Enum) -> Self {
        Self::LiDAR(value)
    }
}

#[derive(Debug)]
pub enum ExtractError<T> {
    WrongCommandSet(ResponseData),
    WrongCommand(T),
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
pub enum MessageData {
    #[deku(id = "0x00")] General(general::message::Enum),
}

impl Parsable<'_> for MessageData {}

impl From<general::message::Enum> for MessageData {
    fn from(value: general::message::Enum) -> Self {
        Self::General(value)
    }
}
