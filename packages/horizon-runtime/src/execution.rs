use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::releases::{CLIENT_BIN_NAME, SERVER_BIN_NAME, version_dir};

pub fn client_binary(tag: &str) -> PathBuf {
    version_dir(tag).join(CLIENT_BIN_NAME)
}

pub fn server_binary(tag: &str) -> PathBuf {
    let custom_server = version_dir(tag).join(SERVER_BIN_NAME);
    if custom_server.is_file() {
        custom_server
    } else {
        // Horizon headless server can be run via client binary with --dedicated
        version_dir(tag).join(CLIENT_BIN_NAME)
    }
}

pub fn build_client_command(
    tag: &str,
    instance_home: &Path,
    resources_dir: &Path,
    extra_args: &[String],
) -> Result<Command> {
    let bin = client_binary(tag);
    if !bin.is_file() {
        bail!("Horizon client binary not found at {}", bin.display());
    }

    let mut cmd = Command::new(&bin);
    cmd.current_dir(instance_home);
    cmd.arg("--resources").arg(resources_dir);

    if let Ok(exe) = std::env::current_exe() {
        cmd.env("HORIZON_LAUNCHER_EXE", exe);
    }

    // Prevent sound issues on Linux with PipeWire
    if std::env::var_os("PIPEWIRE_PROPS").is_none() {
        cmd.env(
            "PIPEWIRE_PROPS",
            r#"{ application.name = "Gradle (Horizon)" media.role = "Game" }"#,
        );
    }

    for arg in extra_args {
        cmd.arg(arg);
    }

    Ok(cmd)
}

pub fn build_server_command(
    tag: &str,
    server_folder: &Path,
    resources_dir: &Path,
    extra_args: &[String],
) -> Result<Command> {
    let bin = server_binary(tag);
    if !bin.is_file() {
        bail!("Horizon server binary not found at {}", bin.display());
    }

    let mut cmd = Command::new(&bin);
    cmd.current_dir(server_folder);
    cmd.arg("--resources").arg(resources_dir);
    cmd.args(["--dedicated", "--nogui"]);

    for arg in extra_args {
        cmd.arg(arg);
    }

    Ok(cmd)
}
