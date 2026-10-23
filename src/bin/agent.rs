//! Background agent: the same as `lumen-lite watch`, but with no console
//! window, logging to `%LOCALAPPDATA%\lumen-lite\agent.log`. Meant to start
//! at logon.

#![windows_subsystem = "windows"]

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use lumen_lite::{timestamped, watch};

/// Start a fresh log once it passes this size.
const MAX_LOG_BYTES: u64 = 1024 * 1024;

fn main() {
    let mut file = open_log();
    let mut log = |msg: &str| {
        if let Some(file) = file.as_mut() {
            let _ = writeln!(file, "{}", timestamped(msg));
        }
    };
    if let Err(err) = watch::run(&mut log) {
        log(&format!("stopped: {err:#}"));
    }
}

fn open_log() -> Option<File> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("lumen-lite");
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join("agent.log");
    let too_big = fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_LOG_BYTES);
    OpenOptions::new()
        .create(true)
        .append(!too_big)
        .write(true)
        .truncate(too_big)
        .open(path)
        .ok()
}
