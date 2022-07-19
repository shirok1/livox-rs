use livox_rs_proc::{generate_full};

use byte_struct::*;
use nalgebra::{Point3, Vector3};

pub(crate) mod prelude {
    pub use super::*;
    pub use super::CmdGeneral::*;
    pub use super::CmdLiDAR::*;
    pub use super::AckGeneral::*;
    pub use super::AckLiDAR::*;
}

bitfields!(
    #[derive(PartialEq, Debug)]
    pub LiDARStatusCode: u32 {
        temp_status: 2,
        volt_status: 2,
        motor_status: 2,
        dirty_warn: 2,
        firmware_status: 1,
        pps_status: 1,
        device_status: 1,
        fan_status: 1,
        self_heating: 1,
        ptp_status: 1,
        time_sync_status: 2,
        reserved: 13,
        system_status: 2,
    }
);

bitfields!(
    #[derive(PartialEq, Debug)]
    pub HubStatusCode: u32 {
        sync_status: 2,
        temp_status: 2,
        LiDAR_status: 1,
        LiDAR_link_status: 1,
        firmware_status: 1,
        RSVD: 23,
        system_status: 2,
    }
);

bitfields!(
    #[derive(PartialEq, Debug)]
    pub TagInfo: u8 {
        space: 2,
        strength: 2,
        return_count: 2,
        near_distortion: 2,
    }
);

#[derive(ByteStruct, PartialEq, Debug)]
#[byte_struct_le]
pub struct DT2 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub reflectivity: u8,
    pub tag: TagInfo,
}

impl DT2 {
    pub fn to_vector(&self) -> Vector3<i32> {
        Vector3::new(self.x, self.y, self.z)
    }
    pub fn to_point(&self) -> Point3<i32> {
        Point3::new(self.x, self.y, self.z)
    }
}

#[derive(ByteStruct, PartialEq, Debug)]
#[byte_struct_le]
pub struct DT3 {
    pub depth: u32,
    pub theta: u16,
    pub phi: u16,
    pub reflectivity: u8,
    pub tag: TagInfo,
}

const fn const_max(a: usize, b: usize) -> usize {
    [a, b][(a < b) as usize]
}

generate_full! {
    enum General {
        BroadcastMessage { } {
            broadcast_code: [u8;16],
            dev_type: u8,
            reserved: u16
        },
        Handshake {
            user_ip: [u8;4],
            data_port: u16,
            cmd_port: u16,
            imu_port: u16,
        } { ret_code: u8 },
        QueryDeviceInformation { } {
            ret_code: u8,
            version: [u8;4],
        },
        Heartbeat { } {
            ret_code: u8,
            work_state: u8,
            feature_msg: u8,
            ack_msg: u32,
        },
        StartStopSampling {
            sample_ctrl: u8,
        } { ret_code: u8 },
        ChangeCoordinateSystem {
            coordinate_type: u8,
        } { ret_code: u8 },
        Disconnect { } { ret_code: u8 },
        PushAbnormalStatusInformation {
        } { status_code: u32 },
        ConfigureStaticDynamicIP {
            ip_mode: u8,
            ip_addr: [u8;4],
            net_mask: [u8;4],
            gw_addr: [u8;4],
        } { ret_code: u8 },
        GetDeviceIPInformation { } {
            ret_code: u8,
            ip_mode: u8,
            ip_addr: [u8;4],
            net_mask: [u8;4],
            gw_addr: [u8;4]
        },
        RebootDevice {
            timeout: u16
        } { ret_code: u8 },
        // WriteConfigurationParameters,
        // ReadConfigurationParameters
    }
}

generate_full! {
    enum LiDAR {
        SetMode {
            lidar_mode: u8,
        } { ret_code: u8 },
        WriteLiDARExtrinsicParameters {
            roll:f32,
            pitch:f32,
            yaw:f32,
            x:i32,
            y:i32,
            z:i32,
        } { ret_code: u8 },
        ReadLiDARExtrinsicParameters { } {
            ret_code: u8,
            roll:f32,
            pitch:f32,
            yaw:f32,
            x:i32,
            y:i32,
            z:i32,
        },
        TurnOnOffRainFogSuppression {
            state: u8
        } { ret_code: u8 },
        SetTurnOnOffFan {
            state: u8
        } { ret_code: u8 },
        GetTurnOnOffFanState{} {
            ret_code: u8,
            state: u8
        },
        SetLiDARReturnMode {
            mode: u8
        } { ret_code: u8 },
        GetLiDARReturnMode {
        } { ret_code: u32, mode: u8 },
        SetIMUDataPushFrequency {
            frequency: u8
        } { ret_code: u8 },
        GetIMUDataPushFrequency { } {
            ret_code: u8,
            frequency: u8
        },
        UpdateUTCSynchronizeTime {
            year: u8,
            month: u8,
            day: u8,
            hour: u8,
            microsecond: u32,
        } { ret_code: u8 }
    }
}

