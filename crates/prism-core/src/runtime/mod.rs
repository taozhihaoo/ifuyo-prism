//! 运行时基础设施抽象：命令执行与文件系统。
//!
//! 这里只定义**边界**（trait + 数据契约）和一个薄的标准库实现；
//! 业务模块必须依赖 trait 而非具体实现，测试可注入 mock。
//!
//! 注意：`prism-core` 的业务逻辑不调用 Git/Cargo/npm 等具体工具——
//! 本模块只是未来（M4+）经 trait 注入这些能力的中立通道。

pub mod command;
pub mod fs;
