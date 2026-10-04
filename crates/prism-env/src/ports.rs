//! 端口 / 进程采集（规格 M9b §81–§100/§134/§194–§195/§211–§213/§312–§313）。
//!
//! 只采集 **listening TCP**（§83）；平台抽象（§84）：Windows `netstat`、
//! Linux `ss`；归属 PID 解析不出 ⇒ None（§85 不猜）。v1 不采集进程
//! command line（§87 隐私红线：命令行可能含 secret）。

use crate::model::{
    ExpectedPort, PortInfo, PortProtocol, PortState, ProcessInfo, ProcessKind, PORT_CAP,
    PROCESS_CAP,
};
use prism_core::runtime::command::{CommandRunner, CommandSpec};
use prism_core::task::CancellationToken;
use std::path::Path;
use std::sync::Arc;

/// 平台探测的统一执行（§84：netstat/ss 是 OS 事实源，同样经 M0 runner、
/// 有界超时 + 取消；不属于工具 allowlist——它们不是"项目工具"）。
fn platform_probe(
    program: &str,
    args: &[&str],
    runner: &Arc<dyn CommandRunner>,
    cancel: &CancellationToken,
) -> Option<String> {
    let mut spec = CommandSpec::new(program.to_string(), args.iter().map(|a| a.to_string()))
        .with_working_directory(Path::new(".").to_path_buf())
        .with_timeout_ms(crate::probe::DEFAULT_PROBE_TIMEOUT_MS);
    spec.max_output_bytes = Some(512 * 1024);
    match runner.run(&spec, Some(cancel)) {
        Ok(output) if output.exit_code == Some(0) => Some(output.stdout),
        _ => None,
    }
}

/// 监听端口采集（§81–§84）。`observed_at` 由调用方统一打点（§211）。
pub fn collect_ports(
    runner: &Arc<dyn CommandRunner>,
    cancel: &CancellationToken,
    observed_at: &str,
) -> Vec<PortInfo> {
    let mut ports = Vec::new();
    if cfg!(windows) {
        if let Some(output) = platform_probe("netstat", &["-ano", "-p", "tcp"], runner, cancel) {
            for (address, port, pid) in parse_netstat(&output) {
                ports.push(PortInfo {
                    port,
                    protocol: PortProtocol::Tcp,
                    address,
                    state: PortState::Listening,
                    pid,
                    process_name: None,
                    source: "netstat".to_string(),
                    observed_at: observed_at.to_string(),
                });
                if ports.len() >= PORT_CAP {
                    break;
                }
            }
        }
    } else if let Some(output) = platform_probe("ss", &["-tln"], runner, cancel) {
        for (address, port) in parse_ss(&output) {
            ports.push(PortInfo {
                port,
                protocol: PortProtocol::Tcp,
                address,
                state: PortState::Listening,
                pid: None,
                process_name: None,
                source: "ss".to_string(),
                observed_at: observed_at.to_string(),
            });
            if ports.len() >= PORT_CAP {
                break;
            }
        }
    }
    ports.sort_by_key(|info| info.port);
    ports.dedup_by(|a, b| a.port == b.port && a.address == b.address);
    ports
}

/// `netstat -ano -p tcp` 解析：`TCP  0.0.0.0:5173  0.0.0.0:0  LISTENING  1234`。
/// 只取 LISTENING 行；IPv6 形如 `[::]:5173`。PID 解析失败 ⇒ None（不猜）。
pub fn parse_netstat(output: &str) -> Vec<(String, u16, Option<u32>)> {
    let mut rows = Vec::new();
    for line in output.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 5 {
            continue;
        }
        if !columns[0].eq_ignore_ascii_case("TCP") {
            continue;
        }
        if !columns[3].eq_ignore_ascii_case("LISTENING") {
            continue;
        }
        let address = columns[1].to_string();
        let Some(port) = local_port(&address) else {
            continue;
        };
        let pid = columns[4].parse::<u32>().ok();
        rows.push((address, port, pid));
    }
    rows
}

/// `ss -tln` 解析：`LISTEN 0 128 0.0.0.0:5173 0.0.0.0:*`。
pub fn parse_ss(output: &str) -> Vec<(String, u16)> {
    let mut rows = Vec::new();
    for line in output.lines().skip(1) {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 5 || columns[0] != "LISTEN" {
            continue;
        }
        let address = columns[3].to_string();
        let Some(port) = local_port(&address) else {
            continue;
        };
        rows.push((address, port));
    }
    rows
}

/// "0.0.0.0:5173" / "[::]:5173" / "127.0.0.1:8080" → 端口；
/// 裸 IPv6（"::1"）没有端口段 ⇒ None（不猜）。
fn local_port(address: &str) -> Option<u16> {
    if let Some(index) = address.find(']') {
        return address[index + 1..]
            .strip_prefix(':')
            .and_then(|port| port.parse::<u16>().ok());
    }
    let (host, port) = address.rsplit_once(':')?;
    if host.contains(':') {
        return None; // 多冒号无括号 ⇒ 裸 IPv6。
    }
    port.parse::<u16>().ok()
}

/// 进程采集（§86–§89）：端口归属进程 + 生态相关进程名。
/// v1 不采集 command line（§87）；cwd/可执行路径不可得 ⇒ None（§92）。
pub fn collect_processes(
    ports: &[PortInfo],
    runner: &Arc<dyn CommandRunner>,
    cancel: &CancellationToken,
    observed_at: &str,
) -> Vec<ProcessInfo> {
    // 端口归属的 PID（§85）。
    let unique_pids: std::collections::BTreeSet<u32> =
        ports.iter().filter_map(|info| info.pid).collect();
    let pids: Vec<u32> = unique_pids.into_iter().collect();
    if pids.is_empty() {
        return Vec::new();
    }
    let name_by_pid = process_names(runner, cancel);
    let mut processes: Vec<ProcessInfo> = Vec::new();
    for pid in pids {
        let name = name_by_pid.get(&pid).cloned();
        let mut listening: Vec<u16> = ports
            .iter()
            .filter(|info| info.pid == Some(pid))
            .map(|info| info.port)
            .collect();
        listening.sort_unstable();
        listening.dedup();
        processes.push(ProcessInfo {
            pid,
            name: name.clone().unwrap_or_else(|| format!("pid-{pid}")),
            executable: None, // §92：v1 不请求高权限路径信息
            ports: listening,
            classification: name
                .as_deref()
                .map(classify)
                .unwrap_or(ProcessKind::Unknown),
            project_association: None, // §91：无 cwd/cmdline 证据 ⇒ 不猜
            observed_at: observed_at.to_string(),
        });
        if processes.len() >= PROCESS_CAP {
            break;
        }
    }
    processes.sort_by_key(|info| info.pid);
    processes
}

/// PID → 进程名（§85 归属解析）。Windows `tasklist`，Linux `ps`。
fn process_names(
    runner: &Arc<dyn CommandRunner>,
    cancel: &CancellationToken,
) -> std::collections::BTreeMap<u32, String> {
    let mut map = std::collections::BTreeMap::new();
    if cfg!(windows) {
        if let Some(output) = platform_probe("tasklist", &["/FO", "CSV", "/NH"], runner, cancel) {
            for (name, pid) in parse_tasklist(&output) {
                map.insert(pid, name);
            }
        }
    } else if let Some(output) = platform_probe("ps", &["-eo", "pid,comm"], runner, cancel) {
        for (name, pid) in parse_ps(&output) {
            map.insert(pid, name);
        }
    }
    map
}

/// `tasklist /FO CSV /NH`：`"node.exe","1234","Console",...`。
pub fn parse_tasklist(output: &str) -> Vec<(String, u32)> {
    let mut rows = Vec::new();
    for line in output.lines() {
        // CSV：`"name","pid",...` → split('"') = ["", "name", ",", "pid", ...]。
        let mut parts = line.split('"');
        if parts.next() != Some("") {
            continue;
        }
        let Some(name) = parts.next() else {
            continue;
        };
        if parts.next() != Some(",") {
            continue;
        }
        let Some(pid_text) = parts.next() else {
            continue;
        };
        let Ok(pid) = pid_text.parse::<u32>() else {
            continue;
        };
        rows.push((name.to_string(), pid));
    }
    rows
}

/// `ps -eo pid,comm`：`  1234 node`。
pub fn parse_ps(output: &str) -> Vec<(String, u32)> {
    let mut rows = Vec::new();
    for line in output.lines().skip(1) {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 2 {
            continue;
        }
        if let Ok(pid) = columns[0].parse::<u32>() {
            rows.push((columns[1].to_string(), pid));
        }
    }
    rows
}

/// 进程分类（§89：基于 executable/name evidence，不猜）。
pub fn classify(name: &str) -> ProcessKind {
    let lower = name.to_ascii_lowercase();
    let base = lower.strip_suffix(".exe").unwrap_or(&lower);
    match base {
        "node" => ProcessKind::Node,
        "python" | "python3" => ProcessKind::Python,
        "dotnet" => ProcessKind::Dotnet,
        "java" | "javaw" => ProcessKind::Java,
        "cargo" | "rustc" | "rustup" => ProcessKind::Rust,
        "docker" | "com.docker.backend" => ProcessKind::Docker,
        "postgres" | "mysqld" | "sqlservr" | "redis-server" => ProcessKind::Database,
        "chrome" | "msedge" | "firefox" => ProcessKind::Browser,
        _ => ProcessKind::Unknown,
    }
}

/// 期望端口候选（§94）：v1 只解析 compose 的 `ports:`（静态、来源明确）。
/// `"8080:80"` / `"127.0.0.1:5173:5173"` → 宿主端口。
pub fn expected_ports(project_root: &Path) -> Vec<ExpectedPort> {
    let mut out = Vec::new();
    for file in ["docker-compose.yml", "docker-compose.yaml", "compose.yaml"] {
        let Ok(content) = std::fs::read_to_string(project_root.join(file)) else {
            continue;
        };
        for line in content.lines() {
            let trimmed = line.trim();
            let Some(mapping) = trimmed
                .strip_prefix("- \"")
                .and_then(|rest| rest.strip_suffix('"'))
                .or_else(|| trimmed.strip_prefix("- "))
            else {
                continue;
            };
            // 取冒号左侧为宿主端口（"8080:80" / "127.0.0.1:5173:5173"）。
            let segments: Vec<&str> = mapping.split(':').collect();
            let host = if segments.len() >= 3 {
                segments[1]
            } else if segments.len() == 2 {
                segments[0]
            } else {
                continue;
            };
            if let Ok(port) = host.parse::<u16>() {
                out.push(ExpectedPort {
                    port,
                    source: format!("{file} ports"),
                    confidence: prism_core::Confidence::High,
                });
            }
        }
    }
    out.sort_by_key(|expected| expected.port);
    out.dedup_by(|a, b| a.port == b.port && a.source == b.source);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // §195 port tests：free/occupied 由 matcher 判定；这里覆盖解析。
    #[test]
    fn netstat_listening_rows_parsed_ipv4_ipv6() {
        let output = "\n  Proto  Local Address          Foreign Address        State           PID\n  TCP    0.0.0.0:5173           0.0.0.0:0              LISTENING       1234\n  TCP    [::]:8000              [::]:0                 LISTENING       5678\n  TCP    10.0.0.2:52275         93.184.216.34:443      ESTABLISHED     9999\n  TCP    127.0.0.1:5432         0.0.0.0:0              LISTENING       not-a-pid\n";
        let rows = parse_netstat(output);
        assert_eq!(rows.len(), 3, "LISTENING only: {rows:?}");
        assert_eq!(rows[0], ("0.0.0.0:5173".to_string(), 5173, Some(1234)));
        assert_eq!(rows[1], ("[::]:8000".to_string(), 8000, Some(5678)));
        assert_eq!(rows[2].2, None, "PID 解析失败 ⇒ None（§85 不猜）");
    }

    #[test]
    fn ss_listening_rows_parsed() {
        let output = "State  Recv-Q Send-Q Local Address:Port Peer Address:Address\nLISTEN 0      128        0.0.0.0:5173      0.0.0.0:*\nLISTEN 0      128           [::]:22           [::]:*\n";
        let rows = parse_ss(output);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1, 5173);
        assert_eq!(rows[1].1, 22);
    }

    #[test]
    fn tasklist_csv_and_ps_comm_parsed() {
        let csv = "\"node.exe\",\"1234\",\"Console\",\"1\",\"88,000 K\"\n\"chrome.exe\",\"4321\",\"Console\",\"1\",\"200,000 K\"\n";
        let rows = parse_tasklist(csv);
        assert_eq!(rows[0], ("node.exe".to_string(), 1234));
        assert_eq!(rows[1], ("chrome.exe".to_string(), 4321));

        let ps = "  PID COMMAND\n 1234 node\n 5678 python3\n";
        let rows = parse_ps(ps);
        assert_eq!(rows[0], ("node".to_string(), 1234));
        assert_eq!(rows[1], ("python3".to_string(), 5678));
    }

    // §89 process classification。
    #[test]
    fn process_classification_is_evidence_based() {
        assert_eq!(classify("node.exe"), ProcessKind::Node);
        assert_eq!(classify("PYTHON"), ProcessKind::Python);
        assert_eq!(classify("dotnet"), ProcessKind::Dotnet);
        assert_eq!(classify("postgres"), ProcessKind::Database);
        assert_eq!(classify("explorer.exe"), ProcessKind::Unknown);
    }

    // §94 expected ports from compose。
    #[test]
    fn compose_ports_parsed_with_provenance() {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-compose-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"5173:5173\"\n      - 8080:80\n  db:\n    ports:\n      - \"127.0.0.1:5432:5432\"\n",
        )
        .unwrap();
        let expected = expected_ports(&dir);
        assert_eq!(expected.len(), 3, "{expected:?}");
        assert_eq!(expected[0].port, 5173);
        assert_eq!(expected[0].source, "docker-compose.yml ports");
        assert_eq!(expected[1].port, 5432);
        assert_eq!(expected[2].port, 8080);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn local_port_rejects_address_without_port() {
        assert_eq!(local_port("0.0.0.0:5173"), Some(5173));
        assert_eq!(local_port("[::]:8000"), Some(8000));
        assert_eq!(local_port("::1"), None);
    }
}
