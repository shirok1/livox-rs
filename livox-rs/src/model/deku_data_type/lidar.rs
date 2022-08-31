pub mod request {
    use deku::prelude::*;
    use livox_rs_proc::Request;
    use crate::model::traits::Request;

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum Enum {
        #[deku(id = "0x00")]
        SetMode(SetMode),
        #[deku(id = "0x01")]
        WriteLiDARExtrinsicParameters(WriteLiDARExtrinsicParameters),
        #[deku(id = "0x02")]
        ReadLiDARExtrinsicParameters(ReadLiDARExtrinsicParameters),
        #[deku(id = "0x03")]
        TurnOnOffRainFogSuppression(TurnOnOffRainFogSuppression),
        #[deku(id = "0x04")]
        SetTurnOnOffFan(SetTurnOnOffFan),
        #[deku(id = "0x05")]
        GetTurnOnOffFanState(GetTurnOnOffFanState),
        #[deku(id = "0x06")]
        SetLiDARReturnMode(SetLiDARReturnMode),
        #[deku(id = "0x07")]
        GetLiDARReturnMode(GetLiDARReturnMode),
        #[deku(id = "0x08")]
        SetIMUDataPushFrequency(SetIMUDataPushFrequency),
        #[deku(id = "0x09")]
        GetIMUDataPushFrequency(GetIMUDataPushFrequency),
        #[deku(id = "0x0A")]
        UpdateUTCSynchronizeTime(UpdateUTCSynchronizeTime),
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct SetMode {
        pub(crate) lidar_mode: u8,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct WriteLiDARExtrinsicParameters {
        roll: f32,
        pitch: f32,
        yaw: f32,
        x: i32,
        y: i32,
        z: i32,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct ReadLiDARExtrinsicParameters {}

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct TurnOnOffRainFogSuppression {
        state: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct SetTurnOnOffFan {
        state: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct GetTurnOnOffFanState {}

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct SetLiDARReturnMode {
        mode: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct GetLiDARReturnMode {}

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct SetIMUDataPushFrequency {
        frequency: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct GetIMUDataPushFrequency {}

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Request)]
    #[deku(endian = "little")]
    pub struct UpdateUTCSynchronizeTime {
        year: u8,
        month: u8,
        day: u8,
        hour: u8,
        microsecond: u32,
    }
}

pub mod response {
    use deku::prelude::*;
    use livox_rs_proc::Response;
    use crate::model::traits::Response;

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum Enum {
        #[deku(id = "0x00")]
        SetMode(SetMode),
        #[deku(id = "0x01")]
        WriteLiDARExtrinsicParameters(WriteLiDARExtrinsicParameters),
        #[deku(id = "0x02")]
        ReadLiDARExtrinsicParameters(ReadLiDARExtrinsicParameters),
        #[deku(id = "0x03")]
        TurnOnOffRainFogSuppression(TurnOnOffRainFogSuppression),
        #[deku(id = "0x04")]
        SetTurnOnOffFan(SetTurnOnOffFan),
        #[deku(id = "0x05")]
        GetTurnOnOffFanState(GetTurnOnOffFanState),
        #[deku(id = "0x06")]
        SetLiDARReturnMode(SetLiDARReturnMode),
        #[deku(id = "0x07")]
        GetLiDARReturnMode(GetLiDARReturnMode),
        #[deku(id = "0x08")]
        SetIMUDataPushFrequency(SetIMUDataPushFrequency),
        #[deku(id = "0x09")]
        GetIMUDataPushFrequency(GetIMUDataPushFrequency),
        #[deku(id = "0x0A")]
        UpdateUTCSynchronizeTime(UpdateUTCSynchronizeTime),
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct SetMode {
        pub(crate) ret_code: u8,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct WriteLiDARExtrinsicParameters {
        ret_code: u8,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct ReadLiDARExtrinsicParameters {
        ret_code: u8,
        roll: f32,
        pitch: f32,
        yaw: f32,
        x: i32,
        y: i32,
        z: i32,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct TurnOnOffRainFogSuppression {
        ret_code: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct SetTurnOnOffFan {
        ret_code: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct GetTurnOnOffFanState {
        ret_code: u8,
        state: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct SetLiDARReturnMode {
        ret_code: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct GetLiDARReturnMode {
        ret_code: u32,
        mode: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct SetIMUDataPushFrequency {
        ret_code: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct GetIMUDataPushFrequency {
        ret_code: u8,
        frequency: u8,
    }

    #[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite, Response)]
    #[deku(endian = "little")]
    pub struct UpdateUTCSynchronizeTime {
        ret_code: u8,
    }
}

pub mod message {
    // use deku::prelude::*;
    // use livox_rs_proc::Message;
    // use crate::model::traits::Message;

    pub type Enum = u8;
}

#[cfg(test)]
mod test {
    use super::super::*;

    #[test]
    fn test_request() {
        use super::request::*;
        let data: Vec<u8> = vec![0x01, 0x00, 0x01];
        let (_rest, val) = Data::<Request>::from_bytes((data.as_ref(), 0)).unwrap();

        assert_eq!(Data::<Request>::LiDAR(Enum::SetMode(SetMode {
            lidar_mode: 0x01
        })), val);

        let data_out = val.to_bytes().unwrap();
        assert_eq!(data, data_out);
    }

    #[test]
    fn test_response() {
        use super::response::*;
        let data: Vec<u8> = vec![0x01, 0x00, 0x00];
        let (_rest, val) = Data::<Response>::from_bytes((data.as_ref(), 0)).unwrap();

        assert_eq!(Data::<Response>::LiDAR(Enum::SetMode(SetMode {
            ret_code: 0x00
        })), val);

        let data_out = val.to_bytes().unwrap();
        assert_eq!(data, data_out);
    }
}
