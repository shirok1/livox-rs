use crate::FrameData;
use crate::model::{ControlFrame};
use crate::model::deku_data_type::*;
use crate::model::deku_data_type::general::request::ConfigureStaticDynamicIP;
use crate::model::deku_data_type::lidar::request::SetLiDARReturnMode;

#[test]
fn test_control_frame() {
    // let data = ControlFrame {
    //     version: 1,
    //     data: CmdFrame(Command::LiDAR(CmdLiDAR::SetLiDARReturnMode {
    //         mode: 2,
    //     })),
    //     seq_num: 3,
    // };
    let data = ControlFrame {
        version: 1,
        data: FrameData::Request(RequestData::LiDAR(lidar::request::Enum::SetLiDARReturnMode(SetLiDARReturnMode {
            mode: 2,
        }))),
        seq_num: 3,
    };
    let buf = data.serialize();
    let neo_data = ControlFrame::parse(&buf).unwrap();
    assert_eq!(data, neo_data);


    let data = ControlFrame {
        version: 1,
        data: FrameData::Request(RequestData::General(general::request::Enum::ConfigureStaticDynamicIP(ConfigureStaticDynamicIP {
            ip_mode: 1,
            ip_addr: [1, 2, 3, 4],
            net_mask: [5, 7, 9, 1],
            gw_addr: [255, 255, 255, 0],
        }))),
        seq_num: 3,
    };
    let buf = data.serialize();
    let neo_data = ControlFrame::parse(&buf).unwrap();
    assert_eq!(data, neo_data);
}