//! prism-test-helper：命令执行抽象的跨平台测试助手。
//!
//! 子命令：
//! - `echo <text>`  输出 text 到 stdout，退出码 0
//! - `sleep <ms>`   睡眠 ms 毫秒后退出 0
//! - `fail`         stderr 输出 helper-failed，退出码 3
//! - `spew <bytes>` 输出约 bytes 字节的重复文本（测试大输出/管道背压）

use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("echo") => {
            println!("{}", args.get(1).map(String::as_str).unwrap_or(""));
            0
        }
        Some("sleep") => {
            let ms: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
            sleep(Duration::from_millis(ms));
            0
        }
        Some("fail") => {
            eprintln!("helper-failed");
            3
        }
        Some("spew") => {
            let bytes: usize = args
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(128 * 1024);
            let chunk = "prism-spew-".repeat(64); // ~768 bytes
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            let mut written = 0usize;
            while written < bytes {
                let remaining = bytes - written;
                let take = chunk.len().min(remaining);
                let _ = lock.write_all(&chunk.as_bytes()[..take]);
                written += take;
            }
            let _ = lock.flush();
            0
        }
        _ => {
            eprintln!("usage: prism-test-helper <echo|sleep|fail|spew> [arg]");
            2
        }
    };
    std::process::exit(code);
}
