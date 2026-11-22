//! Launch and process execution for Gradle Rust Edition (Horizon client / server / ctl)
//!
//! Provides typed, validated argument builders for launching Horizon without raw shell concatenation.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::agent::{ensure_horizon_ctl, horizon_ctl_path};
use crate::releases::{CLIENT_BIN_NAME, SERVER_BIN_NAME, version_dir};

/// Returns the path to the Horizon client executable for the given tag
pub fn client_binary(tag: &str) -> PathBuf {
	version_dir(tag).join(CLIENT_BIN_NAME)
}

/// Returns the path to the Horizon server executable for the given tag
pub fn server_binary(tag: &str) -> PathBuf {
	let custom_server = version_dir(tag).join(SERVER_BIN_NAME);
	if custom_server.is_file() {
		custom_server
	} else {
		// Horizon headless server can be run via client binary with --dedicated
		version_dir(tag).join(CLIENT_BIN_NAME)
	}
}

/// Typed arguments configuration for launching the Horizon client
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizonClientArgs {
	pub tag: String,
	pub instance_home: PathBuf,
	pub resources_dir: PathBuf,
	pub username: Option<String>,
	pub uuid: Option<String>,
	pub server_address: Option<String>,
	pub server_port: Option<u16>,
	pub window_width: Option<u32>,
	pub window_height: Option<u32>,
	pub fullscreen: bool,
	pub allow_ai_agents: bool,
	pub graphics_backend: Option<String>,
	pub env_vars: Vec<(String, String)>,
	pub extra_args: Vec<String>,
}

/// Typed builder for Horizon client arguments
#[derive(Debug, Clone)]
pub struct HorizonClientArgsBuilder {
	tag: String,
	instance_home: PathBuf,
	resources_dir: PathBuf,
	username: Option<String>,
	uuid: Option<String>,
	server_address: Option<String>,
	server_port: Option<u16>,
	window_width: Option<u32>,
	window_height: Option<u32>,
	fullscreen: bool,
	allow_ai_agents: Option<bool>,
	graphics_backend: Option<String>,
	env_vars: Vec<(String, String)>,
	extra_args: Vec<String>,
}

impl HorizonClientArgsBuilder {
	pub fn new(
		tag: impl Into<String>,
		instance_home: impl Into<PathBuf>,
		resources_dir: impl Into<PathBuf>,
	) -> Self {
		Self {
			tag: tag.into(),
			instance_home: instance_home.into(),
			resources_dir: resources_dir.into(),
			username: None,
			uuid: None,
			server_address: None,
			server_port: None,
			window_width: None,
			window_height: None,
			fullscreen: false,
			allow_ai_agents: None,
			graphics_backend: None,
			env_vars: Vec::new(),
			extra_args: Vec::new(),
		}
	}

	pub fn with_auth(
		mut self,
		username: impl Into<String>,
		uuid: Option<impl Into<String>>,
	) -> Self {
		self.username = Some(username.into());
		self.uuid = uuid.map(|u| u.into());
		self
	}

	pub fn with_server(mut self, address: impl Into<String>, port: Option<u16>) -> Self {
		self.server_address = Some(address.into());
		self.server_port = port;
		self
	}

	pub fn with_window_size(mut self, width: u32, height: u32) -> Self {
		self.window_width = Some(width);
		self.window_height = Some(height);
		self
	}

	pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
		self.fullscreen = fullscreen;
		self
	}

	pub fn with_ai_agents(mut self, allow: bool) -> Self {
		self.allow_ai_agents = Some(allow);
		self
	}

	pub fn with_graphics_backend(mut self, backend: impl Into<String>) -> Self {
		self.graphics_backend = Some(backend.into());
		self
	}

	pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.env_vars.push((key.into(), value.into()));
		self
	}

	pub fn with_extra_arg(mut self, arg: impl Into<String>) -> Self {
		self.extra_args.push(arg.into());
		self
	}

	pub fn with_extra_args(
		mut self,
		args: impl IntoIterator<Item = impl Into<String>>,
	) -> Self {
		for arg in args {
			self.extra_args.push(arg.into());
		}
		self
	}

	/// Validates all parameters prior to building
	pub fn validate(&self) -> Result<()> {
		if self.tag.trim().is_empty() {
			bail!("Horizon version tag cannot be empty");
		}
		if let Some(port) = self.server_port {
			if port == 0 {
				bail!("Server port cannot be 0");
			}
		}
		if let Some(w) = self.window_width {
			if w < 100 {
				bail!("Window width ({w}) is invalid; must be >= 100 pixels");
			}
		}
		if let Some(h) = self.window_height {
			if h < 100 {
				bail!("Window height ({h}) is invalid; must be >= 100 pixels");
			}
		}
		Ok(())
	}

	/// Finalizes argument building into typed `HorizonClientArgs`
	pub fn build(self) -> Result<HorizonClientArgs> {
		self.validate()?;

		let allow_ai = match self.allow_ai_agents {
			Some(val) => val,
			None => {
				let settings = crate::settings::load_horizon_settings(Some(&self.instance_home));
				settings.allow_ai_agents
			}
		};

		Ok(HorizonClientArgs {
			tag: self.tag,
			instance_home: self.instance_home,
			resources_dir: self.resources_dir,
			username: self.username,
			uuid: self.uuid,
			server_address: self.server_address,
			server_port: self.server_port,
			window_width: self.window_width,
			window_height: self.window_height,
			fullscreen: self.fullscreen,
			allow_ai_agents: allow_ai,
			graphics_backend: self.graphics_backend,
			env_vars: self.env_vars,
			extra_args: self.extra_args,
		})
	}

	/// Builds the `std::process::Command` ready to execute
	pub fn build_command(&self) -> Result<Command> {
		self.validate()?;
		let bin = client_binary(&self.tag);
		if !bin.is_file() {
			bail!("Horizon client binary not found at {}", bin.display());
		}

		let mut cmd = Command::new(&bin);
		cmd.current_dir(&self.instance_home);
		cmd.arg("--resources").arg(&self.resources_dir);

		if let Some(ref user) = self.username {
			cmd.arg("--username").arg(user);
		}

		if let Some(ref uuid) = self.uuid {
			cmd.arg("--uuid").arg(uuid);
		}

		if let Some(ref server) = self.server_address {
			cmd.arg("--server").arg(server);
			if let Some(port) = self.server_port {
				cmd.arg("--port").arg(port.to_string());
			}
		}

		if let (Some(w), Some(h)) = (self.window_width, self.window_height) {
			cmd.arg("--width").arg(w.to_string());
			cmd.arg("--height").arg(h.to_string());
		}

		if self.fullscreen {
			cmd.arg("--fullscreen");
		}

		let allow_ai = match self.allow_ai_agents {
			Some(val) => val,
			None => {
				let settings = crate::settings::load_horizon_settings(Some(&self.instance_home));
				settings.allow_ai_agents
			}
		};
		if allow_ai {
			cmd.arg("--control");
		}

		for arg in &self.extra_args {
			cmd.arg(arg);
		}

		// Set environment variables safely
		if let Ok(exe) = std::env::current_exe() {
			cmd.env("HORIZON_LAUNCHER_EXE", exe);
		}
		cmd.env("HORIZON_HOME", &self.instance_home);

		if std::env::var_os("PIPEWIRE_PROPS").is_none() {
			cmd.env(
				"PIPEWIRE_PROPS",
				r#"{ application.name = "Gradle (Horizon)" media.role = "Game" }"#,
			);
		}

		if let Some(ref backend) = self.graphics_backend {
			cmd.env("WGPU_BACKEND", backend);
		}

		for (key, val) in &self.env_vars {
			cmd.env(key, val);
		}

		Ok(cmd)
	}
}

/// Typed builder for Horizon server arguments
#[derive(Debug, Clone)]
pub struct HorizonServerArgsBuilder {
	tag: String,
	server_folder: PathBuf,
	resources_dir: PathBuf,
	port: Option<u16>,
	nogui: bool,
	dedicated: bool,
	env_vars: Vec<(String, String)>,
	extra_args: Vec<String>,
}

impl HorizonServerArgsBuilder {
	pub fn new(
		tag: impl Into<String>,
		server_folder: impl Into<PathBuf>,
		resources_dir: impl Into<PathBuf>,
	) -> Self {
		Self {
			tag: tag.into(),
			server_folder: server_folder.into(),
			resources_dir: resources_dir.into(),
			port: None,
			nogui: true,
			dedicated: true,
			env_vars: Vec::new(),
			extra_args: Vec::new(),
		}
	}

	pub fn with_port(mut self, port: u16) -> Self {
		self.port = Some(port);
		self
	}

	pub fn with_nogui(mut self, nogui: bool) -> Self {
		self.nogui = nogui;
		self
	}

	pub fn with_dedicated(mut self, dedicated: bool) -> Self {
		self.dedicated = dedicated;
		self
	}

	pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.env_vars.push((key.into(), value.into()));
		self
	}

	pub fn with_extra_args(
		mut self,
		args: impl IntoIterator<Item = impl Into<String>>,
	) -> Self {
		for arg in args {
			self.extra_args.push(arg.into());
		}
		self
	}

	pub fn build_command(&self) -> Result<Command> {
		let bin = server_binary(&self.tag);
		if !bin.is_file() {
			bail!("Horizon server binary not found at {}", bin.display());
		}

		let mut cmd = Command::new(&bin);
		cmd.current_dir(&self.server_folder);
		cmd.arg("--resources").arg(&self.resources_dir);

		if self.dedicated {
			cmd.arg("--dedicated");
		}
		if self.nogui {
			cmd.arg("--nogui");
		}
		if let Some(port) = self.port {
			cmd.arg("--port").arg(port.to_string());
		}

		for arg in &self.extra_args {
			cmd.arg(arg);
		}

		for (key, val) in &self.env_vars {
			cmd.env(key, val);
		}

		Ok(cmd)
	}
}

/// Helper for executing horizon-ctl CLI commands
#[derive(Debug, Clone, Default)]
pub struct HorizonCtlCommandBuilder {
	subcommand: Vec<String>,
	env_vars: Vec<(String, String)>,
}

impl HorizonCtlCommandBuilder {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn status(mut self) -> Self {
		self.subcommand = vec!["status".to_string()];
		self
	}

	pub fn connect(mut self, host: &str, port: u16) -> Self {
		self.subcommand = vec![
			"connect".to_string(),
			format!("{host}:{port}"),
		];
		self
	}

	pub fn stop(mut self) -> Self {
		self.subcommand = vec!["stop".to_string()];
		self
	}

	pub fn raw(mut self, args: &[&str]) -> Self {
		self.subcommand = args.iter().map(|s| s.to_string()).collect();
		self
	}

	pub fn build_command(&self) -> Result<Command> {
		let ctl_path = match ensure_horizon_ctl() {
			Ok(p) => p,
			Err(_) => horizon_ctl_path(),
		};

		if !ctl_path.is_file() {
			bail!("horizon-ctl binary not available at {}", ctl_path.display());
		}

		let mut cmd = Command::new(&ctl_path);
		for arg in &self.subcommand {
			cmd.arg(arg);
		}
		for (key, val) in &self.env_vars {
			cmd.env(key, val);
		}
		Ok(cmd)
	}
}

/// Backwards-compatible convenience wrapper for client command execution
pub fn build_client_command(
	tag: &str,
	instance_home: &Path,
	resources_dir: &Path,
	extra_args: &[String],
) -> Result<Command> {
	HorizonClientArgsBuilder::new(tag, instance_home, resources_dir)
		.with_extra_args(extra_args.iter().cloned())
		.build_command()
}

/// Backwards-compatible convenience wrapper for server command execution
pub fn build_server_command(
	tag: &str,
	server_folder: &Path,
	resources_dir: &Path,
	extra_args: &[String],
) -> Result<Command> {
	HorizonServerArgsBuilder::new(tag, server_folder, resources_dir)
		.with_extra_args(extra_args.iter().cloned())
		.build_command()
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::tempdir;

	#[test]
	fn test_client_args_builder_validation() {
		let dir = tempdir().unwrap();
		let res = dir.path().join("resources");

		// Empty tag fails
		let b1 = HorizonClientArgsBuilder::new("", dir.path(), &res);
		assert!(b1.validate().is_err());

		// Port 0 fails
		let b2 = HorizonClientArgsBuilder::new("v1.0", dir.path(), &res).with_server("localhost", Some(0));
		assert!(b2.validate().is_err());

		// Window dimension < 100 fails
		let b3 = HorizonClientArgsBuilder::new("v1.0", dir.path(), &res).with_window_size(50, 600);
		assert!(b3.validate().is_err());

		// Valid builder succeeds
		let b4 = HorizonClientArgsBuilder::new("v1.0", dir.path(), &res)
			.with_auth("Player1", Some("00000000-0000-0000-0000-000000000000"))
			.with_server("127.0.0.1", Some(25565))
			.with_window_size(1280, 720)
			.with_fullscreen(true)
			.with_graphics_backend("vulkan");
		assert!(b4.validate().is_ok());

		let built = b4.build().unwrap();
		assert_eq!(built.username.as_deref(), Some("Player1"));
		assert_eq!(built.server_port, Some(25565));
		assert_eq!(built.window_width, Some(1280));
		assert!(built.fullscreen);
		assert_eq!(built.graphics_backend.as_deref(), Some("vulkan"));
	}

	#[test]
	fn test_server_args_builder() {
		let dir = tempdir().unwrap();
		let res = dir.path().join("resources");

		let s = HorizonServerArgsBuilder::new("v1.0", dir.path(), &res)
			.with_port(25565)
			.with_nogui(true)
			.with_dedicated(true)
			.with_extra_args(["--custom-flag"]);

		assert_eq!(s.port, Some(25565));
		assert!(s.nogui);
		assert!(s.dedicated);
		assert_eq!(s.extra_args, vec!["--custom-flag"]);
	}
}
