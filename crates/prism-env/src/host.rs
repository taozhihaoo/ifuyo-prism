//! Host / OS / Architecture / Shell / 资源采集（规格 M9a §14–§18/§140–§145/§161–§168）。
//!
//! 全部只读（§176 read-only probes）；采不到的即 None（§307 Unsupported ≠ Failed）。

use crate::model::{Architecture, HostInfo, OsFamily, ResourceFacts, ShellInfo, ShellKind};
use std::path::Path;

/// 采集 host 事实（§14：最小化身份信息——只取 hostname，username 不采集）。
pub fn collect_host() -> HostInfo {
    let os = OsFamily::from_const(std::env::consts::OS);
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|name| !name.trim().is_empty());
    let locale = collect_locale();
    HostInfo {
        os,
        os_name: friendly_os_name(os),
        os_version: os_version(),
        architecture: Architecture::from_const(std::env::consts::ARCH),
        hostname,
        username: None, // §14：v1 默认不采集 username
        locale,
        timezone: None, // §287：非核心，v1 不实现
    }
}

fn friendly_os_name(os: OsFamily) -> String {
    match os {
        OsFamily::Windows => "Windows".to_string(),
        OsFamily::MacOS => "macOS".to_string(),
        OsFamily::Linux => "Linux".to_string(),
        OsFamily::Other => format!("other ({})", std::env::consts::OS),
    }
}

/// OS 版本：v1 仅 Windows 提供（FFI RtlGetVersion）；其他平台 None。
fn os_version() -> Option<String> {
    #[cfg(windows)]
    {
        windows_version()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// locale 名（§286：只记名字，不展示完整变量内容）。
fn collect_locale() -> Option<String> {
    for key in ["LC_ALL", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// 当前 shell 探测（§17：记录事实；§161：不启动用户 shell profile）。
pub fn collect_shell() -> ShellInfo {
    let (kind, executable) = if cfg!(windows) {
        // Windows 启发式：PSModulePath 存在 ⇒ PowerShell 家族（含 Core 路径
        // 则 pwsh）；否则 ComSpec ⇒ cmd（§191 Windows 重点）。
        let ps_module_path = std::env::var("PSModulePath").unwrap_or_default();
        if !ps_module_path.is_empty() {
            if ps_module_path.contains("PowerShellCore") || which_pwsh().is_some() {
                (ShellKind::Pwsh, which_pwsh())
            } else {
                (ShellKind::PowerShell, which_powershell())
            }
        } else {
            let comspec = std::env::var("ComSpec").ok().filter(|s| !s.is_empty());
            (ShellKind::Cmd, comspec)
        }
    } else {
        let shell = std::env::var("SHELL").ok().filter(|s| !s.is_empty());
        let kind = shell.as_deref().map(basename).unwrap_or_default();
        let shell_kind = match kind {
            "bash" => ShellKind::Bash,
            "zsh" => ShellKind::Zsh,
            "fish" => ShellKind::Fish,
            "pwsh" => ShellKind::Pwsh,
            "powershell" => ShellKind::PowerShell,
            _ => ShellKind::Unknown,
        };
        (shell_kind, shell)
    };
    ShellInfo {
        kind,
        executable,
        version: None, // §18：v1 不为 shell 发起版本 probe
    }
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn which_pwsh() -> Option<String> {
    prism_core::runtime::lookup::resolve_executable("pwsh", Path::new("."))
        .map(|path| path.to_string_lossy().to_string())
}

fn which_powershell() -> Option<String> {
    prism_core::runtime::lookup::resolve_executable("powershell", Path::new("."))
        .map(|path| path.to_string_lossy().to_string())
}

/// 资源事实（§140/§142：时点快照；§144 记录逻辑核数与架构而非负载）。
pub fn collect_resources(project_root: &Path) -> ResourceFacts {
    let logical_cpus = std::thread::available_parallelism()
        .ok()
        .map(|count| count.get() as u64);
    let mut facts = ResourceFacts {
        logical_cpus,
        ..Default::default()
    };
    #[cfg(windows)]
    {
        let (memory, disk) = windows_resources(project_root);
        facts.memory_total_bytes = memory;
        facts.disk_free_bytes = disk.0;
        facts.disk_path = disk.1;
    }
    #[cfg(not(windows))]
    {
        let _ = project_root;
    }
    facts
}

// ---------- Windows FFI（§190 平台适配器；只读 API） ----------

#[cfg(windows)]
fn windows_resources(project_root: &Path) -> (Option<u64>, (Option<u64>, Option<String>)) {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    // 物理/虚拟内存总量（§140 memory total）。
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    let memory = unsafe { GlobalMemoryStatusEx(&mut status) };
    let memory_total = (memory != 0).then_some(status.ullTotalPhys);

    // 项目根所在卷的可用空间（§143 disk context）。
    let mut free_to_caller: u64 = 0;
    let mut total: u64 = 0;
    let mut free: u64 = 0;
    let mut wide: Vec<u16> = project_root
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .collect();
    wide.push(0);
    let disk_ok =
        unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free_to_caller, &mut total, &mut free) };
    let disk_free = (disk_ok != 0).then_some(free);
    let disk_path = Some(project_root.to_string_lossy().to_string());
    (memory_total, (disk_free, disk_path))
}

/// OS 版本（Windows）：读注册表 CurrentVersion（GetVersionExW 在现代
/// Windows 上返回兼容 shim 值，不可信）。失败 ⇒ None（不猜）。
#[cfg(windows)]
fn windows_version() -> Option<String> {
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    };

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let subkey = wide("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");

    let read_dword = |name: &str| -> Option<u32> {
        let name = wide(name);
        let mut value: u32 = 0;
        let mut size: u32 = 4;
        let ok = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                subkey.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut value as *mut u32 as *mut core::ffi::c_void,
                &mut size,
            )
        };
        (ok == 0).then_some(value)
    };

    let read_sz = |name: &str| -> Option<String> {
        let name = wide(name);
        let mut buffer = [0u16; 64];
        let mut size: u32 = (buffer.len() as u32) * 2;
        let ok = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                subkey.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                &mut size,
            )
        };
        (ok == 0).then(|| {
            let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
            String::from_utf16_lossy(&buffer[..len])
        })
    };

    let major = read_dword("CurrentMajorVersionNumber")?;
    let minor = read_dword("CurrentMinorVersionNumber")?;
    let build = read_sz("CurrentBuildNumber")?;
    Some(format!("{major}.{minor}.{build}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_facts_are_normalized_and_minimal() {
        let host = collect_host();
        assert!(matches!(
            host.os,
            OsFamily::Windows | OsFamily::MacOS | OsFamily::Linux | OsFamily::Other
        ));
        assert!(!host.architecture.as_str().is_empty());
        assert!(host.username.is_none(), "§14: v1 does not collect username");
        #[cfg(windows)]
        assert!(
            host.os_version.is_some(),
            "windows registry version available"
        );
    }

    #[test]
    fn shell_kind_is_detected_without_spawning() {
        let shell = collect_shell();
        if cfg!(windows) {
            assert!(
                matches!(
                    shell.kind,
                    ShellKind::PowerShell | ShellKind::Pwsh | ShellKind::Cmd
                ),
                "windows shell detected: {:?}",
                shell.kind
            );
        }
    }

    #[test]
    fn resources_are_collected_or_unknown() {
        let facts = collect_resources(Path::new("."));
        assert!(
            facts.logical_cpus.unwrap_or(0) > 0,
            "logical cpus always available"
        );
        #[cfg(windows)]
        {
            assert!(facts.memory_total_bytes.unwrap_or(0) > 0);
            assert!(facts.disk_free_bytes.unwrap_or(0) > 0);
            assert!(facts.disk_path.is_some());
        }
    }
}
