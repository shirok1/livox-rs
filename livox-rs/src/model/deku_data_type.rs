mod general;

mod lidar;

use deku::prelude::*;

pub trait CmdType<'a>/*: Eq*/ {
    type General: DekuRead<'a> + DekuWrite;
    type LiDAR: DekuRead<'a> + DekuWrite;
    // type Hub: DekuRead<'a> + DekuWrite;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Request;

#[derive(Debug, PartialEq, Eq)]
pub struct Response;

#[derive(Debug, PartialEq, Eq)]
pub struct Message;

impl CmdType<'_> for Request {
    type General = general::request::Enum;
    type LiDAR = lidar::request::Enum;
    // type Hub = ();
}


impl CmdType<'_> for Response {
    type General = general::response::Enum;
    type LiDAR = lidar::response::Enum;
    // type Hub = ();
}

impl CmdType<'_> for Message {
    type General = general::message::Enum;
    type LiDAR = lidar::message::Enum;
    // type Hub = ();
}

#[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
enum Data<'a, DT: CmdType<'a>> {
    #[deku(id = "0x00")] General(DT::General),
    #[deku(id = "0x01")] LiDAR(DT::LiDAR),
    // #[deku(id = "0x02")] Hub(DT::Hub),
}
