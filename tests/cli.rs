use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("blk-{}-{tag}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_blk"))
        .args(args)
        .env("BLK_DIR", dir.path().join("state"))
        .output()
        .unwrap()
}

fn setup(tag: &str, hosts: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new(tag);
    let hosts_path = dir.path().join("hosts");
    fs::write(&hosts_path, hosts).unwrap();
    (dir, hosts_path)
}

fn list(dir: &TempDir, name: &str, body: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, body).unwrap();
    path
}

#[test]
fn applies_two_sources_as_two_sections() {
    let (dir, hosts) = setup("two", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let b = list(&dir, "b.hosts", "0.0.0.0 tracker.example\n");

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = fs::read_to_string(&hosts).unwrap();
    assert_eq!(text.matches("# list: ").count(), 2);
    assert!(text.starts_with("127.0.0.1 localhost"));
    assert!(text.contains("0.0.0.0 ads.example"));
    assert!(text.contains("0.0.0.0 tracker.example"));
}

#[test]
fn reapplying_writes_identical_bytes() {
    let (dir, hosts) = setup("idempotent", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");

    let args = [
        "--hosts",
        hosts.to_str().unwrap(),
        "update",
        a.to_str().unwrap(),
    ];
    assert!(run(&dir, &args).status.success());
    let first = fs::read_to_string(&hosts).unwrap();
    assert!(run(&dir, &args).status.success());
    let second = fs::read_to_string(&hosts).unwrap();
    assert_eq!(first, second);
}

#[test]
fn missing_region_is_appended() {
    let (dir, hosts) = setup("append", "127.0.0.1 localhost");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = fs::read_to_string(&hosts).unwrap();
    assert!(text.starts_with("127.0.0.1 localhost\n\n# BEGIN blk\n"));
    assert!(text.ends_with("# END blk\n"));
}

#[test]
fn unbalanced_markers_leave_the_file_untouched() {
    let (dir, hosts) = setup("unbalanced", "127.0.0.1 localhost\n# BEGIN blk\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let before = fs::read_to_string(&hosts).unwrap();

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap(),
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("matching"));
    assert_eq!(fs::read_to_string(&hosts).unwrap(), before);
}

#[test]
fn off_pauses_every_source_and_resume_restores() {
    let (dir, hosts) = setup("off", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());
    let before = fs::read_to_string(&hosts).unwrap();

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "off"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let paused = fs::read_to_string(&hosts).unwrap();
    assert!(!paused.contains("ads.example"));
    assert!(paused.contains("# BEGIN blk\n# END blk\n"));
    assert!(dir.path().join("state").join("pause.toml").exists());

    assert!(run(&dir, &["resume"]).status.success());
    assert_eq!(fs::read_to_string(&hosts).unwrap(), before);
}

#[test]
fn pause_one_source_keeps_the_other_and_resume_restores_bytes() {
    let (dir, hosts) = setup("pause-one", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let b = list(&dir, "b.hosts", "0.0.0.0 tracker.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap(),
            b.to_str().unwrap()
        ]
    )
    .status
    .success());
    let before = fs::read_to_string(&hosts).unwrap();

    let out = run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "pause", "a.hosts"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let paused = fs::read_to_string(&hosts).unwrap();
    assert!(!paused.contains("ads.example"));
    assert!(paused.contains("tracker.example"));
    assert!(String::from_utf8_lossy(&out.stdout).contains("nothing will restore"));

    assert!(run(&dir, &["resume"]).status.success());
    assert_eq!(fs::read_to_string(&hosts).unwrap(), before);
}

#[test]
fn pause_rejects_unknown_sources() {
    let (dir, hosts) = setup("pause-unknown", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let out = run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "pause", "missing"],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown source"));
}

#[test]
fn status_reports_a_live_pause() {
    let (dir, hosts) = setup("pause-status", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "pause",
            "a.hosts",
            "30m"
        ]
    )
    .status
    .success());

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "status"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("paused:"), "stdout: {stdout}");
}

#[test]
fn an_expired_pause_is_restored_by_the_next_command() {
    let (dir, hosts) = setup("pause-expiry", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());
    let before = fs::read_to_string(&hosts).unwrap();

    assert!(run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "pause", "a.hosts", "1s"]
    )
    .status
    .success());
    std::thread::sleep(Duration::from_secs(2));

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "status"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("pause expired"));
    assert_eq!(fs::read_to_string(&hosts).unwrap(), before);
}

#[test]
fn update_clears_a_pause_record() {
    let (dir, hosts) = setup("pause-update", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());
    assert!(run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "pause", "a.hosts"]
    )
    .status
    .success());

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!dir.path().join("state").join("pause.toml").exists());
    assert!(fs::read_to_string(&hosts).unwrap().contains("ads.example"));
}

#[test]
fn status_reports_sections() {
    let (dir, hosts) = setup("status", "127.0.0.1 localhost\n");
    let a = list(
        &dir,
        "a.hosts",
        "0.0.0.0 ads.example\n0.0.0.0 tracker.example\n",
    );
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "status"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("2 entries in 1 sources"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn dry_run_writes_nothing() {
    let (dir, hosts) = setup("dry", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let before = fs::read_to_string(&hosts).unwrap();

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "--dry-run",
            "update",
            a.to_str().unwrap(),
        ],
    );
    assert!(out.status.success());
    assert_eq!(fs::read_to_string(&hosts).unwrap(), before);
    assert!(!dir.path().join("state").exists());
}

#[test]
fn backup_is_written_before_the_first_change() {
    let (dir, hosts) = setup("backup", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");

    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());
    let backup = dir.path().join("state").join("hosts.bak");
    assert!(backup.exists());
    assert_eq!(fs::read_to_string(backup).unwrap(), "127.0.0.1 localhost\n");
}

#[test]
fn update_without_arguments_uses_the_saved_config() {
    let (dir, hosts) = setup("config-reuse", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let config = fs::read_to_string(dir.path().join("state").join("config.toml")).unwrap();
    assert!(config.contains("a.hosts"), "config: {config}");

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "update"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(fs::read_to_string(&hosts).unwrap().contains("ads.example"));
}

#[test]
fn update_without_sources_or_config_errors() {
    let (dir, hosts) = setup("no-sources", "127.0.0.1 localhost\n");
    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "update"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no sources configured"));
}

#[test]
fn offline_url_source_reads_the_cache() {
    let (dir, hosts) = setup("offline-cache", "127.0.0.1 localhost\n");
    let cache = dir.path().join("state").join("cache");
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("x.hosts"), "0.0.0.0 cached.example\n").unwrap();

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "--offline",
            "update",
            "https://example.test/x.hosts",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = fs::read_to_string(&hosts).unwrap();
    assert!(text.contains("# list: x.hosts"));
    assert!(text.contains("0.0.0.0 cached.example"));
}

#[test]
fn dry_run_writes_no_cache_or_config() {
    let (dir, hosts) = setup("dry-cache", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "--dry-run",
            "update",
            a.to_str().unwrap(),
        ],
    );
    assert!(out.status.success());
    assert!(!dir.path().join("state").exists());
}

#[test]
fn add_appends_a_source_and_applies() {
    let (dir, hosts) = setup("add", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let b = list(&dir, "b.hosts", "0.0.0.0 tracker.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "add",
            b.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = fs::read_to_string(&hosts).unwrap();
    assert_eq!(text.matches("# list: ").count(), 2);
    assert!(text.contains("tracker.example"));
    let config = fs::read_to_string(dir.path().join("state").join("config.toml")).unwrap();
    assert!(config.contains("b.hosts"), "config: {config}");
}

#[test]
fn rm_removes_a_source_and_rebuilds() {
    let (dir, hosts) = setup("rm", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let b = list(&dir, "b.hosts", "0.0.0.0 tracker.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap(),
            b.to_str().unwrap()
        ]
    )
    .status
    .success());

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "rm",
            b.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = fs::read_to_string(&hosts).unwrap();
    assert_eq!(text.matches("# list: ").count(), 1);
    assert!(!text.contains("tracker.example"));
    assert!(text.contains("ads.example"));
    let config = fs::read_to_string(dir.path().join("state").join("config.toml")).unwrap();
    assert!(!config.contains("b.hosts"), "config: {config}");
}

#[test]
fn rm_all_removes_the_region() {
    let (dir, hosts) = setup("rm-all", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "rm",
            a.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = fs::read_to_string(&hosts).unwrap();
    assert!(!text.contains("# BEGIN blk"), "{text}");
    assert!(!dir.path().join("state").join("state.toml").exists());
}

#[test]
fn add_while_paused_only_saves_config() {
    let (dir, hosts) = setup("add-paused", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let b = list(&dir, "b.hosts", "0.0.0.0 tracker.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());
    assert!(run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "pause", "a.hosts"]
    )
    .status
    .success());

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "add",
            b.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!fs::read_to_string(&hosts)
        .unwrap()
        .contains("tracker.example"));
    assert!(dir.path().join("state").join("pause.toml").exists());
    let config = fs::read_to_string(dir.path().join("state").join("config.toml")).unwrap();
    assert!(config.contains("b.hosts"), "config: {config}");
}

#[test]
fn add_rejects_unknown_sources() {
    let (dir, hosts) = setup("add-unknown", "127.0.0.1 localhost\n");
    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "add", "nope"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown source"));
}

#[test]
fn sources_lists_current_sources_without_a_tty() {
    let (dir, hosts) = setup("sources-list", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "sources"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("a.hosts"), "stdout: {stdout}");
    assert!(stdout.contains("blk catalog"), "stdout: {stdout}");
}

#[test]
fn status_json_reports_sources_and_warnings() {
    let (dir, hosts) = setup(
        "json",
        "127.0.0.1 localhost\n# Block List\n0.0.0.0 legacy.example\n",
    );
    let a = list(
        &dir,
        "a.hosts",
        "0.0.0.0 ads.example\n0.0.0.0 tracker.example\n",
    );
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let out = run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "status", "--json"],
    );
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with('{'), "stdout: {stdout}");
    assert!(stdout.contains("\"active\":true"), "stdout: {stdout}");
    assert!(stdout.contains("\"total_entries\":2"), "stdout: {stdout}");
    assert!(stdout.contains("\"name\":\"a.hosts\""), "stdout: {stdout}");
    assert!(stdout.contains("legacy-block-list"), "stdout: {stdout}");
}

#[test]
fn test_reports_region_and_other_lines() {
    let (dir, hosts) = setup(
        "test-domain",
        "127.0.0.1 localhost\n0.0.0.0 outside.example\n",
    );
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let blocked = run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "test", "ads.example"],
    );
    assert!(String::from_utf8_lossy(&blocked.stdout).contains("blocked by blk"));

    let other = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "test",
            "outside.example",
        ],
    );
    assert!(String::from_utf8_lossy(&other.stdout).contains("blocked by another line"));

    let free = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "test",
            "nothing.example",
        ],
    );
    assert!(String::from_utf8_lossy(&free.stdout).contains("not blocked"));
}

#[test]
fn doctor_passes_after_a_clean_update() {
    let (dir, hosts) = setup("doctor-ok", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "doctor"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("doctor: ok"));
}

#[test]
fn doctor_fails_on_drift() {
    let (dir, hosts) = setup("doctor-drift", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let text = fs::read_to_string(&hosts).unwrap();
    fs::write(
        &hosts,
        text.replace(
            "0.0.0.0 ads.example",
            "0.0.0.0 ads.example\n0.0.0.0 hand.edit",
        ),
    )
    .unwrap();

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "doctor"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("drift"));
}

#[test]
fn doctor_fails_on_unbalanced_markers() {
    let (dir, hosts) = setup("doctor-broken", "127.0.0.1 localhost\n# BEGIN blk\n");
    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "doctor"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("region:"));
}

#[test]
fn bare_without_a_tty_prints_status_and_exits_two() {
    let (dir, hosts) = setup("bare", "127.0.0.1 localhost\n");
    let out = run(&dir, &["--hosts", hosts.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stdout).contains("no block region"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage"));
}

#[test]
fn atomic_write_keeps_the_file_mode() {
    use std::os::unix::fs::PermissionsExt;

    let (dir, hosts) = setup("mode", "127.0.0.1 localhost\n");
    fs::set_permissions(&hosts, fs::Permissions::from_mode(0o600)).unwrap();
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");

    let out = run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mode = fs::metadata(&hosts).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn resume_dry_run_does_not_write() {
    let (dir, hosts) = setup("resume-dry", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());
    assert!(run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "pause", "a.hosts"]
    )
    .status
    .success());
    let paused = fs::read_to_string(&hosts).unwrap();

    let out = run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "--dry-run", "resume"],
    );
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("would restore"));
    assert_eq!(fs::read_to_string(&hosts).unwrap(), paused);
    assert!(dir.path().join("state").join("pause.toml").exists());
}

#[test]
fn an_expired_pause_is_restored_by_the_test_command() {
    let (dir, hosts) = setup("expired-test", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());
    assert!(run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "pause", "a.hosts", "1s"]
    )
    .status
    .success());
    std::thread::sleep(Duration::from_secs(2));

    let out = run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "test", "ads.example"],
    );
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("blocked by blk"));
    assert!(!dir.path().join("state").join("pause.toml").exists());
}

#[test]
fn status_prints_a_drift_warning() {
    let (dir, hosts) = setup("drift-text", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap()
        ]
    )
    .status
    .success());

    let text = fs::read_to_string(&hosts).unwrap();
    fs::write(
        &hosts,
        text.replace(
            "0.0.0.0 ads.example",
            "0.0.0.0 ads.example\n0.0.0.0 hand.edit",
        ),
    )
    .unwrap();

    let out = run(&dir, &["--hosts", hosts.to_str().unwrap(), "status"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("warning: region-drift"));
}

#[test]
fn drift_is_skipped_while_paused() {
    let (dir, hosts) = setup("drift-paused", "127.0.0.1 localhost\n");
    let a = list(&dir, "a.hosts", "0.0.0.0 ads.example\n");
    let b = list(&dir, "b.hosts", "0.0.0.0 tracker.example\n");
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "update",
            a.to_str().unwrap(),
            b.to_str().unwrap()
        ]
    )
    .status
    .success());
    assert!(run(
        &dir,
        &[
            "--hosts",
            hosts.to_str().unwrap(),
            "pause",
            "a.hosts",
            "30m"
        ]
    )
    .status
    .success());

    let json = run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "status", "--json"],
    );
    assert!(!String::from_utf8_lossy(&json.stdout).contains("region-drift"));

    let doctor = run(&dir, &["--hosts", hosts.to_str().unwrap(), "doctor"]);
    assert!(
        doctor.status.success(),
        "{}",
        String::from_utf8_lossy(&doctor.stdout)
    );
    assert!(String::from_utf8_lossy(&doctor.stdout).contains("skipped while paused"));
}

#[test]
fn update_unknown_source_names_candidates() {
    let (dir, hosts) = setup("unknown-update", "127.0.0.1 localhost\n");
    let out = run(
        &dir,
        &["--hosts", hosts.to_str().unwrap(), "update", "nope"],
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown source nope"), "stderr: {stderr}");
    assert!(stderr.contains("brave/default"), "stderr: {stderr}");
}
