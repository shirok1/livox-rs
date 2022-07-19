// use crate::model::ParseError;




// enum GeneralCommand {
//     BroadcastMessage,
//     Handshake,
//     QueryDeviceInformation,
//     Heartbeat,
//     StartStopSampling,
//     ChangeCoordinateSystem,
//     Disconnect,
//     PushAbnormalStatusInformation,
//     ConfigureStaticDynamicIP,
//     GetDeviceIPInformation,
//     RebootDevice,
//     WriteConfigurationParameters,
//     ReadConfigurationParameters,
// }

enum LiDARCommand {
    SetMode,
    WriteLiDARExtrinsicParameters,
    ReadLiDARExtrinsicParameters,
    TurnOnOffRainFogSuppression,
    SetTurnOnOffFan,
    GetTurnOnOffFanState,
    SetLiDARReturnMode,
    GetLiDARReturnMode,
    SetIMUDataPushFrequency,
    GetIMUDataPushFrequency,
    UpdateUTCSynchronizationTime,
}
