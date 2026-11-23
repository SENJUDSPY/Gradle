//! License verification gate for the Mod Porting Assistant.
//!
//! Enforces that source repositories have verified open-source licenses permitting
//! derivative works (MIT, Apache-2.0, LGPL, GPL). Refuses "All Rights Reserved" or
//! ambiguous licenses without explicit human operator confirmation.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Categorization of detected software licenses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseType {
    /// Permissive licenses allowing unlimited derivative works (e.g. MIT, Apache-2.0, BSD)
    Permissive { spdx_id: String },
    /// Copyleft licenses allowing derivative works under compatible terms (e.g. LGPL, GPL, MPL)
    Copyleft { spdx_id: String },
    /// Proprietary or restrictive licenses prohibiting unauthorized derivatives
    AllRightsReserved { notice: String },
    /// Unknown or custom license that cannot be parsed automatically
    Ambiguous { details: String },
    /// No license file or declaration found in repository
    Missing,
}

impl LicenseType {
    pub fn allows_derivatives(&self) -> bool {
        matches!(self, Self::Permissive { .. } | Self::Copyleft { .. })
    }

    pub fn spdx_id(&self) -> Option<&str> {
        match self {
            Self::Permissive { spdx_id } | Self::Copyleft { spdx_id } => Some(spdx_id),
            _ => None,
        }
    }
}

/// Verdict returned by the License Gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseVerdict {
    pub license_type: LicenseType,
    pub license_file_found: Option<String>,
    pub allows_derivatives: bool,
    pub operator_override_applied: bool,
    pub can_proceed: bool,
    pub message: String,
}

/// Known license file candidate filenames in public repositories
const LICENSE_FILENAMES: &[&str] = &[
    "LICENSE",
    "LICENSE.md",
    "LICENSE.txt",
    "LICENCE",
    "LICENCE.md",
    "LICENCE.txt",
    "COPYING",
    "COPYING.LESSER",
    "COPYING.txt",
];

/// Evaluates a source repository root directory for licensing compliance
pub fn evaluate_repository_license(
    repo_root: &Path,
    operator_confirmed_permission: bool,
) -> Result<LicenseVerdict> {
    if !repo_root.is_dir() {
        bail!("Source repository path does not exist or is not a directory: {}", repo_root.display());
    }

    let mut found_file = None;
    let mut license_text = String::new();

    // 1. Search for dedicated license files
    for name in LICENSE_FILENAMES {
        let p = repo_root.join(name);
        if p.is_file()
            && let Ok(content) = fs::read_to_string(&p) {
                found_file = Some(name.to_string());
                license_text = content;
                break;
            }
    }

    // 2. Search root README.md or build.gradle if no dedicated license file was found
    if found_file.is_none() {
        for fallback in &["README.md", "README", "build.gradle", "build.gradle.kts"] {
            let p = repo_root.join(fallback);
            if p.is_file()
                && let Ok(content) = fs::read_to_string(&p) {
                    let lower = content.to_lowercase();
                    if lower.contains("license") || lower.contains("all rights reserved") {
                        found_file = Some(fallback.to_string());
                        license_text = content;
                        break;
                    }
                }
        }
    }

    let Some(license_file) = found_file else {
        if operator_confirmed_permission {
            return Ok(LicenseVerdict {
                license_type: LicenseType::Missing,
                license_file_found: None,
                allows_derivatives: false,
                operator_override_applied: true,
                can_proceed: true,
                message: "No license file found in repository. Operator provided explicit override confirmation of author permission.".to_string(),
            });
        }
        bail!("License Gate Refused: No LICENSE or COPYING file found in source repository. Refusing to port without verified license or operator confirmation.");
    };

    let detected_type = parse_license_text(&license_text);
    let allows_derivatives = detected_type.allows_derivatives();

    if allows_derivatives {
        let spdx = detected_type.spdx_id().unwrap_or("Unknown").to_string();
        Ok(LicenseVerdict {
            license_type: detected_type,
            license_file_found: Some(license_file),
            allows_derivatives: true,
            operator_override_applied: false,
            can_proceed: true,
            message: format!("License Gate Passed: Detected verified open-source license ({spdx}) allowing derivative works."),
        })
    } else if operator_confirmed_permission {
        Ok(LicenseVerdict {
            license_type: detected_type,
            license_file_found: Some(license_file),
            allows_derivatives: false,
            operator_override_applied: true,
            can_proceed: true,
            message: "License Gate Override: Repository license does not permit automatic derivatives, but human operator confirmed explicit author permission.".to_string(),
        })
    } else {
        match detected_type {
            LicenseType::AllRightsReserved { notice } => {
                bail!("License Gate BLOCKED: Repository is marked '{notice}'. Derivative works and porting are prohibited without author permission.");
            }
            LicenseType::Ambiguous { details } => {
                bail!("License Gate BLOCKED: Ambiguous or unrecognized license ({details}). Operator confirmation of author permission is required before proceeding.");
            }
            _ => {
                bail!("License Gate BLOCKED: License does not permit derivative works without explicit author consent.");
            }
        }
    }
}

/// Identifies the license from file contents
pub fn parse_license_text(text: &str) -> LicenseType {
    let lower = text.to_lowercase();

    // Check for All Rights Reserved
    if lower.contains("all rights reserved")
        && !lower.contains("mit license")
        && !lower.contains("apache license")
        && !lower.contains("gnu general public license")
    {
        return LicenseType::AllRightsReserved {
            notice: "All Rights Reserved (ARR)".to_string(),
        };
    }

    // Permissive
    if lower.contains("mit license") || (lower.contains("permission is hereby granted") && lower.contains("free of charge")) {
        return LicenseType::Permissive { spdx_id: "MIT".to_string() };
    }
    if lower.contains("apache license") && lower.contains("version 2.0") {
        return LicenseType::Permissive { spdx_id: "Apache-2.0".to_string() };
    }
    if lower.contains("bsd 2-clause") || (lower.contains("redistribution and use in source and binary forms") && lower.contains("2-clause")) {
        return LicenseType::Permissive { spdx_id: "BSD-2-Clause".to_string() };
    }
    if lower.contains("bsd 3-clause") || (lower.contains("redistribution and use in source and binary forms") && lower.contains("neither the name")) {
        return LicenseType::Permissive { spdx_id: "BSD-3-Clause".to_string() };
    }
    if lower.contains("isc license") || lower.contains("permission to use, copy, modify, and/or distribute this software for any purpose") {
        return LicenseType::Permissive { spdx_id: "ISC".to_string() };
    }
    if lower.contains("creative commons zero") || lower.contains("cc0 1.0") {
        return LicenseType::Permissive { spdx_id: "CC0-1.0".to_string() };
    }

    // Copyleft
    if lower.contains("lesser general public license") || lower.contains("lgpl") {
        if lower.contains("version 3") || lower.contains("v3") {
            return LicenseType::Copyleft { spdx_id: "LGPL-3.0-or-later".to_string() };
        }
        return LicenseType::Copyleft { spdx_id: "LGPL-2.1-or-later".to_string() };
    }
    if lower.contains("gnu general public license") || lower.contains("gpl") {
        if lower.contains("version 3") || lower.contains("v3") {
            return LicenseType::Copyleft { spdx_id: "GPL-3.0-or-later".to_string() };
        }
        return LicenseType::Copyleft { spdx_id: "GPL-2.0-or-later".to_string() };
    }
    if lower.contains("mozilla public license") || lower.contains("mpl") {
        return LicenseType::Copyleft { spdx_id: "MPL-2.0".to_string() };
    }

    LicenseType::Ambiguous {
        details: "Unrecognized license header. Please inspect LICENSE file.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mit_license() {
        let text = "MIT License\n\nCopyright (c) 2026 Author\n\nPermission is hereby granted...";
        let parsed = parse_license_text(text);
        assert_eq!(parsed, LicenseType::Permissive { spdx_id: "MIT".to_string() });
        assert!(parsed.allows_derivatives());
    }

    #[test]
    fn test_parse_apache_license() {
        let text = "Apache License\nVersion 2.0, January 2004\nhttp://www.apache.org/licenses/";
        let parsed = parse_license_text(text);
        assert_eq!(parsed, LicenseType::Permissive { spdx_id: "Apache-2.0".to_string() });
        assert!(parsed.allows_derivatives());
    }

    #[test]
    fn test_parse_lgpl_license() {
        let text = "GNU LESSER GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007";
        let parsed = parse_license_text(text);
        assert_eq!(parsed, LicenseType::Copyleft { spdx_id: "LGPL-3.0-or-later".to_string() });
        assert!(parsed.allows_derivatives());
    }

    #[test]
    fn test_block_all_rights_reserved_without_override() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("LICENSE"), "Copyright (c) 2026 Closed Source. All Rights Reserved.").unwrap();

        let res = evaluate_repository_license(temp.path(), false);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("All Rights Reserved"));
    }

    #[test]
    fn test_allow_all_rights_reserved_with_operator_override() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("LICENSE"), "Copyright (c) 2026 Closed Source. All Rights Reserved.").unwrap();

        let verdict = evaluate_repository_license(temp.path(), true).unwrap();
        assert!(verdict.can_proceed);
        assert!(verdict.operator_override_applied);
    }
}
