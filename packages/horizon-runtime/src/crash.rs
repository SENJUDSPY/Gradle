//! Crash detection, analysis, and report generation for Gradle (Horizon / Rust Edition)
//!
//! Copyright (c) 2026 Gradle contributors (MIT License).

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use parking_lot::RwLock;

/// In-memory storage for the latest crash report per instance
static LAST_CRASH_REPORTS: LazyLock<RwLock<HashMap<String, HorizonCrashReport>>> =
	LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HorizonErrorCategory {
	VulkanDriver,
	RustPanic,
	WgpuValidation,
	OutOfMemory,
	AssetMissing,
	SegmentationFault,
	Aborted,
	Unknown,
}

impl HorizonErrorCategory {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::VulkanDriver => "Vulkan Driver / GPU",
			Self::RustPanic => "Rust Runtime Panic",
			Self::WgpuValidation => "wgpu Graphics Validation",
			Self::OutOfMemory => "Out of Memory",
			Self::AssetMissing => "Missing Game Assets",
			Self::SegmentationFault => "Segmentation Fault (SIGSEGV)",
			Self::Aborted => "Process Aborted (SIGABRT)",
			Self::Unknown => "Unknown Application Crash",
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonCrashReport {
	pub timestamp: String,
	pub instance_id: String,
	pub instance_name: String,
	pub exit_code: Option<i32>,
	pub signal: Option<i32>,
	pub is_crash: bool,
	pub error_category: String,
	pub crash_reason: String,
	pub actionable_fix: String,
	pub panic_message: Option<String>,
	pub wgpu_error: Option<String>,
	pub relevant_logs: Vec<String>,
	pub report_text: String,
	pub report_file_path: Option<String>,
}

/// Detects Rust panic details in the given log lines
pub fn detect_panic(log_lines: &[String]) -> Option<String> {
	for (idx, line) in log_lines.iter().enumerate() {
		if line.contains("panicked at") || line.contains("thread 'main' panicked") {
			let mut full_panic = line.trim().to_string();
			// Capture the next 2-3 lines if they look like location or message continuation
			for next_line in log_lines.iter().skip(idx + 1).take(3) {
				let trimmed = next_line.trim();
				if !trimmed.is_empty() && !trimmed.starts_with('[') {
					full_panic.push(' ');
					full_panic.push_str(trimmed);
				} else {
					break;
				}
			}
			return Some(full_panic);
		}
	}
	None
}

/// Detects wgpu or Vulkan graphics errors in log lines
pub fn detect_wgpu_error(log_lines: &[String]) -> Option<String> {
	for line in log_lines.iter().rev() {
		if line.contains("wgpu error:") || line.contains("Validation Error") {
			return Some(line.trim().to_string());
		}
		if line.contains("VkResult") || line.contains("VK_ERROR_") || line.contains("Failed to create Vulkan") {
			return Some(line.trim().to_string());
		}
		if line.contains("No suitable GPU adapters found") || line.contains("RequestAdapter failed") {
			return Some(line.trim().to_string());
		}
	}
	None
}

/// Detects out-of-memory errors
pub fn detect_oom(log_lines: &[String]) -> bool {
	log_lines.iter().any(|line| {
		line.contains("memory allocation of")
			|| line.contains("out of memory")
			|| line.contains("OutOfMemory")
	})
}

/// Detects missing game assets
pub fn detect_missing_assets(log_lines: &[String]) -> bool {
	log_lines.iter().any(|line| {
		line.contains("resources not found")
			|| line.contains("missing asset")
			|| line.contains("horizon-resources.json")
			|| line.contains("Failed to load texture")
	})
}

/// Classifies error into category, short reason, and actionable fix
pub fn categorize_error(
	exit_code: Option<i32>,
	signal: Option<i32>,
	logs: &[String],
) -> (HorizonErrorCategory, String, String) {
	if let Some(sig) = signal {
		if sig == 11 {
			return (
				HorizonErrorCategory::SegmentationFault,
				"Game process terminated by SIGSEGV (Segmentation Fault)".to_string(),
				"A native memory violation occurred. Verify your GPU drivers and Vulkan ICD libraries with the Diagnostics tool.".to_string(),
			);
		}
		if sig == 6 {
			return (
				HorizonErrorCategory::Aborted,
				"Game process aborted by SIGABRT (Assertion Failure)".to_string(),
				"A native assertion or panic triggered an abort. Review the crash report details and update your GPU drivers.".to_string(),
			);
		}
	}

	if let Some(panic_msg) = detect_panic(logs) {
		return (
			HorizonErrorCategory::RustPanic,
			format!("Rust panic: {}", truncate_str(&panic_msg, 120)),
			"A Rust runtime assertion failed. Check the stack trace in the crash report. If the problem persists, report this issue.".to_string(),
		);
	}

	if let Some(wgpu_msg) = detect_wgpu_error(logs) {
		let cat = if wgpu_msg.contains("VkResult") || wgpu_msg.contains("Vulkan") || wgpu_msg.contains("No suitable GPU") {
			HorizonErrorCategory::VulkanDriver
		} else {
			HorizonErrorCategory::WgpuValidation
		};
		return (
			cat,
			format!("Graphics error: {}", truncate_str(&wgpu_msg, 120)),
			"GPU or Vulkan error. Run Gradle Preflight Diagnostics to ensure your graphics driver supports Vulkan 1.3.".to_string(),
		);
	}

	if detect_oom(logs) {
		return (
			HorizonErrorCategory::OutOfMemory,
			"Out of memory: memory allocation failed".to_string(),
			"The system ran out of RAM or video memory. Close heavy background applications and retry.".to_string(),
		);
	}

	if detect_missing_assets(logs) {
		return (
			HorizonErrorCategory::AssetMissing,
			"Missing required game assets".to_string(),
			"Minecraft 26.3 game assets are missing or corrupted. Run 'Prepare Game Resources' from the setup menu.".to_string(),
		);
	}

	let code_str = exit_code
		.map(|c| format!("code {c}"))
		.or_else(|| signal.map(|s| format!("signal {s}")))
		.unwrap_or_else(|| "unknown exit".to_string());

	(
		HorizonErrorCategory::Unknown,
		format!("Process exited unexpectedly with {code_str}"),
		"Review the crash report logs or run Preflight Diagnostics to ensure all system requirements are met.".to_string(),
	)
}

fn truncate_str(s: &str, max: usize) -> String {
	if s.len() > max {
		format!("{}...", &s[..max])
	} else {
		s.to_string()
	}
}

/// Generates a structured crash report and saves it to `<instance_dir>/crash-reports/`
pub fn generate_and_save_crash_report(
	instance_id: &str,
	instance_name: &str,
	exit_code: Option<i32>,
	signal: Option<i32>,
	recent_logs: &[String],
	instance_dir: &Path,
) -> HorizonCrashReport {
	let now = Utc::now();
	let timestamp_str = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();
	let file_timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();

	let (category, reason, fix) = categorize_error(exit_code, signal, recent_logs);
	let panic_msg = detect_panic(recent_logs);
	let wgpu_msg = detect_wgpu_error(recent_logs);

	// Select the last 40 lines of log output
	let tail_len = recent_logs.len().min(40);
	let relevant_logs: Vec<String> = recent_logs
		.iter()
		.rev()
		.take(tail_len)
		.rev()
		.cloned()
		.collect();

	let mut report_text = String::with_capacity(4096);
	report_text.push_str("================================================================================\n");
	report_text.push_str("                   GRADLE RUST EDITION CRASH REPORT                             \n");
	report_text.push_str("================================================================================\n\n");
	report_text.push_str(&format!("Instance Name:    {}\n", instance_name));
	report_text.push_str(&format!("Instance ID:      {}\n", instance_id));
	report_text.push_str(&format!("Timestamp:        {}\n", timestamp_str));
	report_text.push_str(&format!("Exit Status:      {}\n", match (exit_code, signal) {
		(Some(c), _) => format!("Exit Code {}", c),
		(_, Some(s)) => format!("Killed by Signal {}", s),
		_ => "Unknown".to_string(),
	}));
	report_text.push_str(&format!("Category:         {}\n", category.as_str()));
	report_text.push_str(&format!("Summary:          {}\n\n", reason));

	report_text.push_str("--------------------------------------------------------------------------------\n");
	report_text.push_str("RECOMMENDED ACTION:\n");
	report_text.push_str("--------------------------------------------------------------------------------\n");
	report_text.push_str(&format!("{}\n\n", fix));

	if let Some(ref p) = panic_msg {
		report_text.push_str("--------------------------------------------------------------------------------\n");
		report_text.push_str("PANIC DETAILS:\n");
		report_text.push_str("--------------------------------------------------------------------------------\n");
		report_text.push_str(&format!("{}\n\n", p));
	}

	if let Some(ref w) = wgpu_msg {
		report_text.push_str("--------------------------------------------------------------------------------\n");
		report_text.push_str("GRAPHICS / WGPU DETAILS:\n");
		report_text.push_str("--------------------------------------------------------------------------------\n");
		report_text.push_str(&format!("{}\n\n", w));
	}

	report_text.push_str("--------------------------------------------------------------------------------\n");
	report_text.push_str("RECENT CONSOLE LOGS:\n");
	report_text.push_str("--------------------------------------------------------------------------------\n");
	for line in &relevant_logs {
		report_text.push_str(line);
		if !line.ends_with('\n') {
			report_text.push('\n');
		}
	}
	report_text.push_str("\n================================================================================\n");

	// Save to <instance_dir>/crash-reports/crash-<timestamp>.log
	let crash_reports_dir = instance_dir.join("crash-reports");
	let mut report_file_path = None;
	if fs::create_dir_all(&crash_reports_dir).is_ok() {
		let path = crash_reports_dir.join(format!("crash-{file_timestamp}-rust.log"));
		if fs::write(&path, &report_text).is_ok() {
			report_file_path = Some(path.to_string_lossy().to_string());
		}
	}

	let report = HorizonCrashReport {
		timestamp: timestamp_str,
		instance_id: instance_id.to_string(),
		instance_name: instance_name.to_string(),
		exit_code,
		signal,
		is_crash: true,
		error_category: category.as_str().to_string(),
		crash_reason: reason,
		actionable_fix: fix,
		panic_message: panic_msg,
		wgpu_error: wgpu_msg,
		relevant_logs,
		report_text,
		report_file_path,
	};

	// Record in-memory for immediate UI querying
	record_crash(report.clone());

	report
}

/// Stores the crash report in the in-memory cache
pub fn record_crash(report: HorizonCrashReport) {
	let mut lock = LAST_CRASH_REPORTS.write();
	lock.insert(report.instance_id.clone(), report);
}

/// Retrieves the latest crash report for an instance, if any
pub fn get_latest_crash(instance_id: &str) -> Option<HorizonCrashReport> {
	let lock = LAST_CRASH_REPORTS.read();
	lock.get(instance_id).cloned()
}

/// Clears the crash report for an instance
pub fn clear_crash(instance_id: &str) {
	let mut lock = LAST_CRASH_REPORTS.write();
	lock.remove(instance_id);
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::tempdir;

	#[test]
	fn test_detect_panic() {
		let logs = vec![
			"[INFO] Starting Horizon client...".to_string(),
			"thread 'main' panicked at 'assertion failed: index < len', src/renderer.rs:42:9".to_string(),
			"note: run with `RUST_BACKTRACE=1` environment variable".to_string(),
		];
		let panic = detect_panic(&logs);
		assert!(panic.is_some());
		assert!(panic.unwrap().contains("assertion failed"));
	}

	#[test]
	fn test_detect_wgpu_error() {
		let logs = vec![
			"[INFO] Initializing wgpu surface...".to_string(),
			"wgpu error: Validation Error: Buffer size 0 is invalid".to_string(),
		];
		let err = detect_wgpu_error(&logs);
		assert!(err.is_some());
		assert!(err.unwrap().contains("Buffer size 0 is invalid"));
	}

	#[test]
	fn test_categorize_vulkan() {
		let logs = vec![
			"Failed to create Vulkan instance: VK_ERROR_INCOMPATIBLE_DRIVER".to_string(),
		];
		let (cat, reason, fix) = categorize_error(Some(1), None, &logs);
		assert_eq!(cat, HorizonErrorCategory::VulkanDriver);
		assert!(reason.contains("Graphics error"));
		assert!(fix.contains("Preflight Diagnostics"));
	}

	#[test]
	fn test_generate_and_save_report() {
		let dir = tempdir().unwrap();
		let logs = vec![
			"[INFO] Initializing engine...".to_string(),
			"thread 'main' panicked at 'wgpu surface lost'".to_string(),
		];
		let report = generate_and_save_crash_report(
			"test-instance-123",
			"Test World",
			Some(101),
			None,
			&logs,
			dir.path(),
		);

		assert_eq!(report.instance_id, "test-instance-123");
		assert!(report.is_crash);
		assert!(report.report_text.contains("GRADLE RUST EDITION CRASH REPORT"));
		assert!(report.report_file_path.is_some());

		// Verify cached
		let cached = get_latest_crash("test-instance-123");
		assert!(cached.is_some());
		clear_crash("test-instance-123");
		assert!(get_latest_crash("test-instance-123").is_none());
	}
}
