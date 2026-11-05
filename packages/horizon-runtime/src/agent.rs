use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::releases::horizon_data_dir;
use crate::settings::{
    HorizonSettings, load_horizon_settings, save_horizon_settings,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const BUNDLED_CTL: Option<&'static [u8]> =
    Some(include_bytes!("../bin/horizon-ctl"));
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
const BUNDLED_CTL: Option<&'static [u8]> = None;

pub fn horizon_ctl_bin_name() -> &'static str {
    if cfg!(windows) {
        "horizon-ctl.exe"
    } else {
        "horizon-ctl"
    }
}

pub fn horizon_ctl_path() -> PathBuf {
    horizon_data_dir().join("bin").join(horizon_ctl_bin_name())
}

pub fn ensure_horizon_ctl() -> Result<PathBuf> {
    let dest = horizon_ctl_path();
    if dest.is_file() {
        return Ok(dest);
    }

    let parent = dest.parent().context("dest has no parent directory")?;
    fs::create_dir_all(parent)?;

    if let Some(bundled) = BUNDLED_CTL {
        let tmp = parent.join(format!(".tmp_ctl_{}", std::process::id()));
        fs::write(&tmp, bundled)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755));
        }
        let _ = fs::rename(&tmp, &dest);
        return Ok(dest);
    }

    bail!("horizon-ctl binary not available for this platform");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentClientConfig {
    pub id: String,
    pub name: String,
    pub target_file: String,
    pub config_snippet: String,
    pub add_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTestReport {
    pub success: bool,
    pub protocol_version: String,
    pub server_name: String,
    pub server_version: String,
    pub tool_count: usize,
    pub tools: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentControlStatus {
    pub allow_ai_agents: bool,
    pub horizon_ctl_installed: bool,
    pub horizon_ctl_path: String,
    pub configs: Vec<AgentClientConfig>,
}

pub fn get_agent_control_status(
    instance_dir: Option<&Path>,
) -> Result<AgentControlStatus> {
    let settings = load_horizon_settings(instance_dir);
    let ctl = ensure_horizon_ctl().ok();
    let ctl_path = ctl
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| horizon_ctl_path().to_string_lossy().to_string());
    let installed = ctl.is_some();

    let configs = get_agent_configs(&ctl_path);

    Ok(AgentControlStatus {
        allow_ai_agents: settings.allow_ai_agents,
        horizon_ctl_installed: installed,
        horizon_ctl_path: ctl_path,
        configs,
    })
}

pub fn set_agent_control_enabled(
    enabled: bool,
    instance_dir: Option<&Path>,
) -> Result<HorizonSettings> {
    let mut settings = load_horizon_settings(instance_dir);
    settings.allow_ai_agents = enabled;
    save_horizon_settings(instance_dir, &settings)?;
    if enabled {
        let _ = ensure_horizon_ctl();
    }
    Ok(settings)
}

pub fn get_agent_configs(ctl_path: &str) -> Vec<AgentClientConfig> {
    let claude_config = serde_json::json!({
        "mcpServers": {
            "horizon": {
                "command": ctl_path,
                "args": ["mcp"]
            }
        }
    });

    let cursor_config = serde_json::json!({
        "mcpServers": {
            "horizon": {
                "command": ctl_path,
                "args": ["mcp"]
            }
        }
    });

    let escaped_path = ctl_path.replace('\\', "\\\\").replace('"', "\\\"");
    let codex_config = format!(
        "[mcp_servers.horizon]\ncommand = \"{escaped_path}\"\nargs = [\"mcp\"]\n"
    );

    vec![
        AgentClientConfig {
            id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            target_file: "~/.claude.json or .mcp.json".to_string(),
            config_snippet: serde_json::to_string_pretty(&claude_config)
                .unwrap_or_default(),
            add_command: Some(format!(
                "claude mcp add --scope user horizon -- {ctl_path} mcp"
            )),
        },
        AgentClientConfig {
            id: "cursor".to_string(),
            name: "Cursor".to_string(),
            target_file: "~/.cursor/mcp.json".to_string(),
            config_snippet: serde_json::to_string_pretty(&cursor_config)
                .unwrap_or_default(),
            add_command: None,
        },
        AgentClientConfig {
            id: "codex".to_string(),
            name: "Codex".to_string(),
            target_file: "~/.codex/config.toml".to_string(),
            config_snippet: codex_config,
            add_command: Some(format!(
                "codex mcp add horizon -- {ctl_path} mcp"
            )),
        },
        AgentClientConfig {
            id: "generic".to_string(),
            name: "Any MCP Client".to_string(),
            target_file: "client settings (stdio JSON-RPC)".to_string(),
            config_snippet: serde_json::to_string_pretty(&claude_config)
                .unwrap_or_default(),
            add_command: None,
        },
    ]
}

pub fn test_mcp_server() -> Result<McpTestReport> {
    let ctl_path = ensure_horizon_ctl()?;

    let mut child = Command::new(&ctl_path)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn horizon-ctl mcp")?;

    let mut stdin = child.stdin.take().context("Failed to open child stdin")?;
    let stdout = child.stdout.take().context("Failed to open child stdout")?;
    let mut reader = BufReader::new(stdout);

    // 1. Send initialize
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "Gradle Launcher",
                "version": "1.0.0"
            }
        }
    });

    writeln!(stdin, "{}", serde_json::to_string(&init_req)?)?;
    stdin.flush()?;

    let mut init_line = String::new();
    reader.read_line(&mut init_line)?;
    let init_resp: serde_json::Value = serde_json::from_str(&init_line)
        .context("Failed to parse MCP initialize response")?;

    let result = init_resp
        .get("result")
        .context("No result in initialize response")?;
    let proto_ver = result
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let server_info = result.get("serverInfo");
    let s_name = server_info
        .and_then(|s| s.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("horizon-ctl")
        .to_string();
    let s_ver = server_info
        .and_then(|s| s.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();

    // 2. Send tools/list
    let tools_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    writeln!(stdin, "{}", serde_json::to_string(&tools_req)?)?;
    stdin.flush()?;

    let mut tools_line = String::new();
    reader.read_line(&mut tools_line)?;
    let tools_resp: serde_json::Value = serde_json::from_str(&tools_line)
        .context("Failed to parse MCP tools/list response")?;

    let mut tools_vec = Vec::new();
    if let Some(tools_arr) = tools_resp
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
    {
        for t in tools_arr {
            if let Some(t_name) = t.get("name").and_then(|n| n.as_str()) {
                tools_vec.push(t_name.to_string());
            }
        }
    }

    let _ = child.kill();

    let tool_count = tools_vec.len();
    Ok(McpTestReport {
        success: true,
        protocol_version: proto_ver,
        server_name: s_name,
        server_version: s_ver,
        tool_count,
        tools: tools_vec,
        message: format!(
            "MCP server verified successfully with {tool_count} available tools"
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_and_locate_horizon_ctl() {
        let path = ensure_horizon_ctl().expect("Failed to ensure horizon-ctl");
        assert!(path.is_file());
    }

    #[test]
    fn test_agent_configs_generation() {
        let configs = get_agent_configs("/test/bin/horizon-ctl");
        assert_eq!(configs.len(), 4);
        assert!(configs.iter().any(|c| c.id == "claude-code"));
        assert!(configs.iter().any(|c| c.id == "cursor"));
        assert!(configs.iter().any(|c| c.id == "codex"));
    }

    #[test]
    fn test_mcp_test_server_verification() {
        let rep = test_mcp_server().expect("Failed to test MCP server");
        assert!(rep.success);
        assert!(rep.tool_count > 50);
        assert!(rep.tools.contains(&"state".to_string()));
        assert!(rep.tools.contains(&"inventory".to_string()));
        assert!(rep.tools.contains(&"goto".to_string()));
    }
}
