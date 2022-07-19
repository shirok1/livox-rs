# 览沃 Livox 激光雷达适配器

使用 Rust 语言原生编写，直接读写 Livox 激光雷达的数据包。

很大一部分原因是因为 Livox 官方的 SDK 仅提供了 C 风格的接口，过于丑陋，而 ROS 过于庞大且在非 Ubuntu 系统上几乎无法使用。

用 Rust 过程宏（见 `livox-rs-proc`）实现了指令数据包的数据结构，省去大量复制粘贴。

未实现的指令：

- `WriteConfigurationParameters`, `ReadConfigurationParameters`: 涉及到变长消息，宏定义较难实现
- Hub 指令集: 用不上所以没做

[//]: # (未实现的点云数据格式：)

[//]: # ()
[//]: # (- Mid-70 未使用的点云数据格式（即）: 用不上所以没做)

[//]: # (- 双回波点云数据: 用不上所以没做)

另外，点云数据格式仅实现了数据类型 2（Mid-70 使用的单回波直角座标系格式）。

本项目主要使用了以下程序库：

- tokio: Rust 异步编程的核心库
- nalgebra: 线性代数库，用于将点云的坐标转换为相机、像素坐标，用于绘制深度图
- image: Rust 图像处理库，用于绘制深度图

本项目接入了使用 ZeroMQ 和 Protocol Buffers 自行定制的通信协议 rdr，方便跨语言传输。
