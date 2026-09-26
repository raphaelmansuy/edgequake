//! Test-only crash barriers for PROVIDER-ACCESS-E2E04 (B2/B3).
//!
//! Compiled only with the `provider-access-fault` feature. Release builds omit
//! this module. The child process writes a marker file, then blocks opening a
//! FIFO for read (no writer). The parent polls the marker and SIGKILLs.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pause when `EDGEQUAKE_FAULT_BARRIER` equals `barrier`.
///
/// Expected env (set by the process-kill harness):
/// - `EDGEQUAKE_FAULT_BARRIER` — which barrier this child should stop at
/// - `EDGEQUAKE_FAULT_DIR` — directory for `marker` and `hold` FIFO
pub fn pause_at(barrier: &str) {
    let Ok(wanted) = std::env::var("EDGEQUAKE_FAULT_BARRIER") else {
        return;
    };
    if wanted != barrier {
        return;
    }
    let Ok(dir) = std::env::var("EDGEQUAKE_FAULT_DIR") else {
        tracing::error!("provider-access-fault: EDGEQUAKE_FAULT_DIR unset at {barrier}");
        return;
    };
    signal_and_block(Path::new(&dir), barrier);
}

/// Write the ready marker, then block on the hold FIFO until killed.
pub fn signal_and_block(dir: &Path, barrier: &str) {
    let marker = dir.join("marker");
    let hold = dir.join("hold");
    if let Err(error) = write_marker(&marker, barrier) {
        tracing::error!(error = %error, "provider-access-fault: failed to write marker");
        return;
    }
    // No writer is open. open(O_RDONLY) blocks until a writer appears; the
    // parent never opens write and instead SIGKILLs after seeing the marker.
    match OpenOptions::new().read(true).open(&hold) {
        Ok(_file) => {
            // Unreachable in the kill harness: parent kills while we block in open.
            // If a writer appears (misconfigured harness), park forever.
            loop {
                std::thread::park();
            }
        }
        Err(error) => {
            tracing::error!(
                error = %error,
                path = %hold.display(),
                "provider-access-fault: hold open failed"
            );
        }
    }
}

fn write_marker(path: &Path, barrier: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(barrier.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

/// Create the fault directory and hold FIFO (parent side).
pub fn prepare_fault_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let hold = dir.join("hold");
    if hold.exists() {
        let _ = fs::remove_file(&hold);
    }
    let status = Command::new("mkfifo")
        .arg("-m")
        .arg("0600")
        .arg(&hold)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "mkfifo failed with {status}"
        )));
    }
    Ok(())
}

/// Poll until `marker` exists and contains `barrier`, or deadline elapses.
pub fn wait_for_marker(dir: &Path, barrier: &str, deadline: std::time::Duration) -> bool {
    let marker = dir.join("marker");
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if let Ok(contents) = fs::read_to_string(&marker) {
            if contents.trim() == barrier {
                return true;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    false
}

/// Helper for tests that need a PathBuf from env.
pub fn fault_dir_from_env() -> Option<PathBuf> {
    std::env::var_os("EDGEQUAKE_FAULT_DIR").map(PathBuf::from)
}
