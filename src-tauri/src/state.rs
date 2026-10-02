//! 应用共享状态。

use prism_core::application::AppInfoService;
use prism_core::config::PrismConfig;

/// Tauri managed state。只保存配置与服务实例，不缓存业务结果。
pub struct AppState {
    /// M0 的 IPC 命令尚未消费配置；字段保留为 M1+ 命令（scan 等读取
    /// analysis/logging 配置）的入口，删除会让后续里程碑重复接线。
    #[allow(dead_code)]
    pub config: PrismConfig,
    pub app_info: AppInfoService,
}

impl AppState {
    pub fn new(config: PrismConfig) -> Self {
        AppState {
            config,
            app_info: AppInfoService::new(),
        }
    }
}
