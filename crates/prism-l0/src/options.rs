//! L0 扫描选项（2.0-M22 §35/§36/§72）。
//!
//! 只定义当前真实需要的旋钮；Fast/Standard/Deep profile 留待有真实需求再分档，
//! 但字段化选项为未来扩展保留空间（§36）。

/// L0 结构分析选项（资源上限 = 对恶意/超大项目的风险评估结论，§72）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct L0Options {
    /// 文件/目录实体上限；超出 ⇒ Partial + 诊断（§55 不伪装完整）。
    pub max_entities: usize,
    /// 单个 manifest 的依赖声明抽取上限（§28）。
    pub max_dependencies_per_manifest: usize,
    /// manifest 读取大小上限（字节）——超过跳过解析只记录存在（§59 不读一切）。
    pub max_manifest_bytes: u64,
    /// git 状态逐文件事实上限（§48 防关系噪声）。
    pub max_git_status_facts: usize,
    /// 是否记录 os/arch 常量事实（零探测成本；工具探测归 prism-env/M9）。
    pub include_platform_facts: bool,
}

impl Default for L0Options {
    fn default() -> Self {
        L0Options {
            max_entities: 20_000,
            max_dependencies_per_manifest: 200,
            max_manifest_bytes: 1_048_576,
            max_git_status_facts: 100,
            include_platform_facts: true,
        }
    }
}
