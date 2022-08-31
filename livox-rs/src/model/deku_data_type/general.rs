pub mod request {
    use deku::prelude::*;
    use livox_rs_proc::Request;
    use crate::model::traits::Request;

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum Enum {
        #[deku(id = "0x01")]
        Handshake(Handshake),
        #[deku(id = "0x02")]
        QueryDeviceInformation(QueryDeviceInformation),
        #[deku(id = "0x03")]
        Heartbeat(Heartbeat),
        #[deku(id = "0x04")]
        StartStopSampling(StartStopSampling),
        #[deku(id = "0x05")]
        ChangeCoordinateSystem(ChangeCoordinateSystem),
        #[deku(id = "0x06")]
        Disconnect(Disconnect),
        #[deku(id = "0x08")]
        ConfigureStaticDynamicIP(ConfigureStaticDynamicIP),
        #[deku(id = "0x09")]
        GetDeviceIPInformation(GetDeviceIPInformation),
        #[deku(id = "0x0A")]
        RebootDevice(RebootDevice),
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct Handshake {
        pub(crate) user_ip: [u8; 4],
        pub(crate) data_port: u16,
        pub(crate) cmd_port: u16,
        pub(crate) imu_port: u16,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct QueryDeviceInformation {}

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct Heartbeat {}

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct StartStopSampling {
        pub(crate) sample_ctrl: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct ChangeCoordinateSystem {
        pub(crate) coordinate_type: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct Disconnect {}

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct ConfigureStaticDynamicIP {
        pub(crate) ip_mode: u8,
        pub(crate) ip_addr: [u8; 4],
        pub(crate) net_mask: [u8; 4],
        pub(crate) gw_addr: [u8; 4],
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct GetDeviceIPInformation {}

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct RebootDevice {
        pub(crate) timeout: u16,
    }

    // #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    // #[deku(endian = "little")]
    // pub struct WriteConfigurationParameters {
    //     timeout: u16,
    // }
}

pub mod response {
    use deku::prelude::*;
    use crate::model::traits::Response;
    use livox_rs_proc::Response;
    use crate::ResponseData;

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum Enum {
        #[deku(id = "0x01")]
        Handshake(Handshake),
        #[deku(id = "0x02")]
        QueryDeviceInformation(QueryDeviceInformation),
        #[deku(id = "0x03")]
        Heartbeat(Heartbeat),
        #[deku(id = "0x04")]
        StartStopSampling(StartStopSampling),
        #[deku(id = "0x05")]
        ChangeCoordinateSystem(ChangeCoordinateSystem),
        #[deku(id = "0x06")]
        Disconnect(Disconnect),
        #[deku(id = "0x08")]
        ConfigureStaticDynamicIP(ConfigureStaticDynamicIP),
        #[deku(id = "0x09")]
        GetDeviceIPInformation(GetDeviceIPInformation),
        #[deku(id = "0x0A")]
        RebootDevice(RebootDevice),
    }

    impl TryFrom<ResponseData> for Enum {
        type Error = ResponseData;

        fn try_from(value: ResponseData) -> Result<Self, Self::Error> {
            match value {
                ResponseData::General(value) => Ok(value),
                _ => Err(value)
            }
        }
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct Handshake {
        pub(crate) ret_code: u8,
    }

    // impl TryFrom<Enum> for Handshake {
    //     type Error = ();
    //
    //     fn try_from(value: Enum) -> Result<Self, Self::Error> {
    //         match value {
    //             Enum::Handshake(value) => Ok(value),
    //             _ => Err(())
    //         }
    //     }
    // }
    //
    // impl TryFrom<ResponseData> for Handshake {
    //     type Error = ();
    //
    //     fn try_from(value: ResponseData) -> Result<Self, Self::Error> {
    //         match value.try_into() {
    //             Ok(Enum::Handshake(value)) => Ok(value),
    //             _ => Err(())
    //         }
    //     }
    // }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct QueryDeviceInformation {
        pub(crate) ret_code: u8,
        pub(crate) version: [u8; 4],
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct Heartbeat {
        pub(crate) ret_code: u8,
        pub(crate) work_state: u8,
        pub(crate) feature_msg: u8,
        pub(crate) ack_msg: u32,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct StartStopSampling {
        pub(crate) ret_code: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct ChangeCoordinateSystem {
        pub(crate) ret_code: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct Disconnect {
        pub(crate) ret_code: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct ConfigureStaticDynamicIP {
        pub(crate) ret_code: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct GetDeviceIPInformation {
        pub(crate) ret_code: u8,
        pub(crate) ip_mode: u8,
        pub(crate) ip_addr: [u8; 4],
        pub(crate) net_mask: [u8; 4],
        pub(crate) gw_addr: [u8; 4],
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct RebootDevice {
        ret_code: u8,
    }
}

pub mod message {
    use deku::prelude::*;
    use livox_rs_proc::Message;
    use crate::model::traits::Message;


    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum Enum {
        #[deku(id = "0x00")]
        BroadcastMessage(BroadcastMessage),
        #[deku(id = "0x07")]
        PushAbnormalStatusInformation(PushAbnormalStatusInformation),
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Message)]
    #[deku(endian = "little")]
    pub struct BroadcastMessage {
        pub(crate) broadcast_code: [u8; 16],
        pub(crate) dev_type: u8,
        pub(crate) reserved: u16,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Message)]
    #[deku(endian = "little")]
    pub struct PushAbnormalStatusInformation {
        status_code: u32,
    }
}

#[cfg(test)]
mod test {
    use super::super::*;

    #[test]
    fn test_request() {
        use super::request::*;
        let data: Vec<u8> = vec![0x00, 0x01,
                                 127, 0, 0, 1,
                                 0x34, 0x12,
                                 0x78, 0x56,
                                 0x12, 0x90];
        let (_rest, val) = RequestData::from_bytes((data.as_ref(), 0)).unwrap();

        assert_eq!(RequestData::General(Enum::Handshake(Handshake {
            user_ip: [127, 0, 0, 1],
            data_port: 0x1234,
            cmd_port: 0x5678,
            imu_port: 0x9012,
        })), val);

        let data_out = val.to_bytes().unwrap();
        assert_eq!(data, data_out);
    }

    #[test]
    fn test_response() {
        use super::response::*;
        let data: Vec<u8> = vec![0x00, 0x01, 0x01];
        let (_rest, val) = ResponseData::from_bytes((data.as_ref(), 0)).unwrap();

        assert_eq!(ResponseData::General(Enum::Handshake(Handshake {
            ret_code: 0x01
        })), val);

        let data_out = val.to_bytes().unwrap();
        assert_eq!(data, data_out);
    }

    #[test]
    fn test_message() {
        use super::message::*;
        let data: Vec<u8> = vec![0x00, 0x00,
                                 0, 1, 2, 3, 4, 5, 6, 7, 8,
                                 9, 10, 11, 12, 13, 14, 15,
                                 6, 0, 0];
        let (_rest, val) = MessageData::from_bytes((data.as_ref(), 0)).unwrap();

        assert_eq!(MessageData::General(Enum::BroadcastMessage(BroadcastMessage {
            broadcast_code: [0, 1, 2, 3, 4, 5, 6, 7, 8,
                9, 10, 11, 12, 13, 14, 15, ],
            dev_type: 6,
            reserved: 0,
        })), val);

        let data_out = val.to_bytes().unwrap();
        assert_eq!(data, data_out);
    }
}
