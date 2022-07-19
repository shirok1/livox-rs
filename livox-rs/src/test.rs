use crate::model::{Command, ControlFrame};
use crate::model::data_type::*;
use crate::model::FrameData::CmdFrame;

#[test]
fn test_data_type() {
    let data = AckGeneral::Heartbeat {
        ret_code: 6,
        work_state: 2,
        feature_msg: 1,
        ack_msg: 2362,
    };
    let mut buf = [0u8; AckGeneral::MAX_BYTE_LEN];
    data.write_bytes(&mut buf);

    let neo_data = AckGeneral::read_bytes(&buf);

    assert_eq!(data, neo_data);
}

#[test]
fn test_control_frame() {
    let data = ControlFrame {
        version: 1,
        data: CmdFrame(Command::LiDAR(CmdLiDAR::SetLiDARReturnMode {
            mode: 2,
        })),
        seq_num: 3,
    };
    let buf = data.serialize();
    let neo_data = ControlFrame::parse(&buf).unwrap();
    assert_eq!(data, neo_data);


    let data = ControlFrame {
        version: 1,
        data: CmdFrame(Command::General(CmdGeneral::ConfigureStaticDynamicIP {
            ip_mode: 1,
            ip_addr: [1,2,3,4],
            net_mask: [5,7,9,1],
            gw_addr: [255,255,255,0],
        })),
        seq_num: 3,
    };
    let buf = data.serialize();
    let neo_data = ControlFrame::parse(&buf).unwrap();
    assert_eq!(data, neo_data);
}