use byte_struct::*;
use nalgebra::{Point3, Vector3};

pub(crate) mod prelude {
    pub use super::*;
}

bitfields!(
    #[derive(PartialEq, Eq, Debug)]
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
    #[derive(PartialEq, Eq, Debug)]
    pub HubStatusCode: u32 {
        sync_status: 2,
        temp_status: 2,
        lidar_status: 1,
        lidar_link_status: 1,
        firmware_status: 1,
        reserved: 23,
        system_status: 2,
    }
);

bitfields!(
    #[derive(PartialEq, Eq, Debug)]
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
