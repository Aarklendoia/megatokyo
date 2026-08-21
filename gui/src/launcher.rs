//! Windowed mode: spawns Qt's own `qml6` runtime against `qml/main.qml`,
//! same pattern as linux-hello's `linux_hello_config` and kio-protondrive's
//! wizard — no Qt/Rust binding crate. QML talks to the daemon directly over
//! its HTTP API (local or remote — see `daemon_link`), so unlike those two
//! launchers this one needs no local control server of its own: there's
//! nothing here for QML to call back into.

use std::path::PathBuf;
use std::process::Command;

use megatokyo_core::local_ctrl::{runtime_dir, write_owner_only_file};

use crate::daemon_link::DaemonLink;

/// Refuses to start a second window if one is already running for this
/// user — same PID-liveness check as `linux_hello_config`'s own lock. Takes
/// the lock file path directly (rather than resolving `$XDG_RUNTIME_DIR`
/// itself) so tests can point it at a tempdir without touching that
/// process-global env var, which parallel `cargo test` runs can't safely
/// mutate per test.
fn acquire_single_instance_lock(path: &std::path::Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            if std::path::Path::new(&format!("/proc/{pid}")).exists() {
                return false;
            }
        }
    }
    let _ = write_owner_only_file(path, &std::process::id().to_string());
    true
}

fn find_qml_path() -> PathBuf {
    let candidates = [
        "/usr/share/megatokyo/qml/main.qml",
        "/usr/share/qt6/qml/Megatokyo/main.qml",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return path;
        }
    }
    // Development fallback: the workspace's qml/ directory, a sibling of
    // this crate's own directory.
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default();
    workspace_root.join("qml").join("main.qml")
}

pub fn run(link: &DaemonLink) -> std::process::ExitCode {
    let uid = crate::daemon_link::current_uid();
    let lock_path = runtime_dir(uid).join("megatokyo-gui.lock");
    if !acquire_single_instance_lock(&lock_path) {
        eprintln!("Megatokyo is already open.");
        return std::process::ExitCode::SUCCESS;
    }

    let qml_path = find_qml_path();
    if !qml_path.exists() {
        eprintln!(
            "Could not find main.qml (looked at {}). Is megatokyo-gui installed correctly?",
            qml_path.display()
        );
        return std::process::ExitCode::FAILURE;
    }

    let qml_import_paths = ["/usr/lib/x86_64-linux-gnu/qt6/qml", "/usr/share/qt6/qml"].join(":");
    let qt_plugin_paths = [
        "/usr/lib/x86_64-linux-gnu/qt6/plugins",
        "/usr/lib/qt6/plugins",
    ]
    .join(":");

    // base_url/token passed positionally after `--`, not as env vars: QML
    // reads `Qt.application.arguments`, but not always the process
    // environment (see linux_hello_config's own note on this).
    let status = Command::new("qml6")
        .arg("-name")
        .arg("megatokyo")
        .arg(&qml_path)
        .arg("--")
        .arg(&link.base_url)
        .arg(&link.token)
        .env("QML_IMPORT_PATH", &qml_import_paths)
        .env("QML2_IMPORT_PATH", &qml_import_paths)
        .env("QT_PLUGIN_PATH", &qt_plugin_paths)
        .env("QT_QPA_PLATFORM", "xcb;wayland;offscreen")
        .env("QT_APPLICATION_DISPLAY_NAME", "Megatokyo")
        .env("QT_QPA_DESKTOPFILENAME", "megatokyo")
        // Qt blocks local-file XHR reads by default; I18n.qml needs one to
        // load qml/i18n/<lang>.json (same requirement as
        // linux_hello_config's own I18n.qml).
        .env("QML_XHR_ALLOW_FILE_READ", "1")
        .status();

    match status {
        Ok(status) if status.success() => std::process::ExitCode::SUCCESS,
        Ok(_) => std::process::ExitCode::FAILURE,
        Err(err) => {
            eprintln!("Could not launch qml6: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stale_lock_from_a_dead_pid_is_reclaimed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("megatokyo-gui.lock");
        // A PID essentially guaranteed not to exist.
        std::fs::write(&path, "999999999").unwrap();
        assert!(acquire_single_instance_lock(&path));
    }

    #[test]
    fn a_lock_held_by_this_very_test_process_is_not_reclaimed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("megatokyo-gui.lock");
        std::fs::write(&path, std::process::id().to_string()).unwrap();
        assert!(!acquire_single_instance_lock(&path));
    }
}
