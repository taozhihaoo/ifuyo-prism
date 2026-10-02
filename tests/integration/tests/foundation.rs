//! M0 验收要求的集成测试：
//! Core can be initialized / Config can load / AppInfo can be generated /
//! Command abstraction can execute（含超时、取消、大输出、非零退出码）。

mod support;

use prism_core::application::AppInfoService;
use prism_core::config::PrismConfig;
use prism_core::project::Project;
use prism_core::runtime::command::{CommandOutput, CommandRunner, StdCommandRunner};
use prism_core::runtime::fs::{FileSystem, StdFileSystem};
use prism_core::task::{CancellationToken, TaskState, TaskTracker};
use support::{helper_command, temp_dir};

#[test]
fn core_can_be_initialized_and_config_can_load() {
    // 默认配置合法。
    PrismConfig::default().validate().unwrap();

    // 不存在的配置文件 → 默认值，且“未从磁盘加载”。
    let path = temp_dir("config").join("prism.config.json");
    let (config, loaded) = PrismConfig::load(&path).unwrap();
    assert!(!loaded);
    assert_eq!(config, PrismConfig::default());

    // 保存后可加载回读。
    config.save(&path).unwrap();
    let (reloaded, loaded) = PrismConfig::load(&path).unwrap();
    assert!(loaded);
    assert_eq!(reloaded, config);
}

#[test]
fn app_info_can_be_generated() {
    let info = AppInfoService::new().app_info();
    assert_eq!(info.name, "Prism");
    assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    assert!(!info.os.is_empty());
    assert!(!info.arch.is_empty());

    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"build_profile\""));
}

#[test]
fn command_runner_executes_and_captures_output() {
    let runner = StdCommandRunner::new();
    let output: CommandOutput = runner
        .run(&helper_command(&["echo", "integration-ok"]), None)
        .unwrap();
    assert_eq!(output.exit_code, Some(0));
    assert!(output.stdout.contains("integration-ok"));
}

#[test]
fn command_runner_reports_nonzero_exit_and_stderr() {
    let runner = StdCommandRunner::new();
    let output = runner.run(&helper_command(&["fail"]), None).unwrap();
    assert_eq!(output.exit_code, Some(3));
    assert!(output.stderr.contains("helper-failed"));
}

#[test]
fn command_runner_handles_large_output_without_deadlock() {
    let runner = StdCommandRunner::new();
    // 1MB 输出：远超管道缓冲区，验证读取线程不会死锁。
    let output = runner
        .run(&helper_command(&["spew", "1048576"]), None)
        .unwrap();
    assert_eq!(output.exit_code, Some(0));
    assert!(output.stdout.len() >= 1_000_000);
}

#[test]
fn command_runner_enforces_timeout() {
    let runner = StdCommandRunner::new();
    let mut spec = helper_command(&["sleep", "30_000"]);
    spec.timeout_ms = 500;
    let started = std::time::Instant::now();
    let err = runner.run(&spec, None).unwrap_err();
    assert_eq!(err.code(), prism_core::PrismErrorCode::Command);
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
}

#[test]
fn command_runner_supports_cancellation() {
    let runner = StdCommandRunner::new();
    let token = CancellationToken::new();
    token.cancel();
    let err = runner
        .run(&helper_command(&["sleep", "30_000"]), Some(&token))
        .unwrap_err();
    assert_eq!(err.code(), prism_core::PrismErrorCode::Cancelled);
}

#[test]
fn filesystem_abstraction_roundtrip() {
    let fs = StdFileSystem::new();
    let dir = temp_dir("fs");
    fs.write(&dir.join("a.txt"), "alpha").unwrap();
    fs.write(&dir.join("b.txt"), "beta").unwrap();

    let entries = fs.list_dir(&dir).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].ends_with("a.txt"), "list must be sorted");

    let meta = fs.metadata(&dir.join("a.txt")).unwrap();
    assert!(meta.is_file);
    assert_eq!(meta.size_bytes, 5);

    assert_eq!(fs.read_to_string(&dir.join("b.txt")).unwrap(), "beta");
    fs.remove_dir_all(&dir).unwrap();
    assert!(!fs.exists(&dir));
}

#[test]
fn project_can_be_created_from_directory() {
    let dir = temp_dir("project");
    let project = Project::from_root(&dir).unwrap();
    assert_eq!(project.name, dir.file_name().unwrap().to_string_lossy());
    assert!(project.root_path.is_absolute());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn task_tracker_tracks_full_lifecycle() {
    let tracker = TaskTracker::new();
    let record = tracker.register("integration");
    tracker.mark_running(&record.id);
    tracker.update_progress(&record.id, 1, Some(4));
    tracker.set_message(&record.id, "in flight");
    tracker.mark_completed(&record.id);

    let done = tracker.get(&record.id).unwrap();
    assert_eq!(done.state, TaskState::Completed);
    assert!(done.started_at.is_some());
    assert!(done.finished_at.is_some());
}
