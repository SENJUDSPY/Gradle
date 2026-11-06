use crate::state::{
    ContentSet, Hooks, Instance, InstanceLink, MemorySettings, WindowSize,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchRuntime {
    Horizon,
    Java,
}

impl Default for LaunchRuntime {
    fn default() -> Self {
        Self::Horizon
    }
}

impl std::fmt::Display for LaunchRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Horizon => write!(f, "Horizon (Rust, optimized)"),
            Self::Java => write!(f, "Java (classic)"),
        }
    }
}

impl std::str::FromStr for LaunchRuntime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "horizon" | "rust" => Ok(Self::Horizon),
            "java" | "classic" => Ok(Self::Java),
            other => Err(format!("unknown launch runtime '{other}'")),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct InstanceTabVisibility {
    pub files: bool,
    pub worlds: bool,
    pub screenshots: bool,
}

impl Default for InstanceTabVisibility {
    fn default() -> Self {
        Self {
            files: true,
            worlds: true,
            screenshots: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceLaunchOverrides {
    pub instance_id: String,
    pub java_path: Option<String>,
    pub extra_launch_args: Option<Vec<String>>,
    pub custom_env_vars: Option<Vec<(String, String)>>,
    pub memory: Option<MemorySettings>,
    pub force_fullscreen: Option<bool>,
    pub game_resolution: Option<WindowSize>,
    pub hooks: Hooks,
    #[serde(default)]
    pub visible_tabs: InstanceTabVisibility,
}

impl InstanceLaunchOverrides {
    pub fn empty(instance_id: String) -> Self {
        Self {
            instance_id,
            java_path: None,
            extra_launch_args: None,
            custom_env_vars: None,
            memory: None,
            force_fullscreen: None,
            game_resolution: None,
            hooks: Hooks {
                pre_launch: None,
                wrapper: None,
                post_exit: None,
            },
            visible_tabs: InstanceTabVisibility::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct InstanceLaunchOverridesData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_launch_args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_env_vars: Option<Vec<(String, String)>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemorySettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_fullscreen: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_resolution: Option<WindowSize>,
    #[serde(default)]
    pub hooks: Hooks,
    #[serde(default)]
    pub visible_tabs: InstanceTabVisibility,
}

impl InstanceLaunchOverridesData {
    pub(crate) fn into_launch_overrides(
        self,
        instance_id: String,
    ) -> InstanceLaunchOverrides {
        InstanceLaunchOverrides {
            instance_id,
            java_path: self.java_path,
            extra_launch_args: self.extra_launch_args,
            custom_env_vars: self.custom_env_vars,
            memory: self.memory,
            force_fullscreen: self.force_fullscreen,
            game_resolution: self.game_resolution,
            hooks: self.hooks,
            visible_tabs: self.visible_tabs,
        }
    }
}

impl From<&InstanceLaunchOverrides> for InstanceLaunchOverridesData {
    fn from(overrides: &InstanceLaunchOverrides) -> Self {
        Self {
            java_path: overrides.java_path.clone(),
            extra_launch_args: overrides.extra_launch_args.clone(),
            custom_env_vars: overrides.custom_env_vars.clone(),
            memory: overrides.memory,
            force_fullscreen: overrides.force_fullscreen,
            game_resolution: overrides.game_resolution,
            hooks: overrides.hooks.clone(),
            visible_tabs: overrides.visible_tabs,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceLaunchContext {
    pub instance: Instance,
    pub applied_content_set: ContentSet,
    pub link: InstanceLink,
    pub launch_overrides: InstanceLaunchOverrides,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_runtime_parsing_and_defaults() {
        assert_eq!(LaunchRuntime::default(), LaunchRuntime::Horizon);
        assert_eq!("horizon".parse::<LaunchRuntime>().unwrap(), LaunchRuntime::Horizon);
        assert_eq!("rust".parse::<LaunchRuntime>().unwrap(), LaunchRuntime::Horizon);
        assert_eq!("java".parse::<LaunchRuntime>().unwrap(), LaunchRuntime::Java);
        assert_eq!("classic".parse::<LaunchRuntime>().unwrap(), LaunchRuntime::Java);
        assert!("unknown".parse::<LaunchRuntime>().is_err());

        assert_eq!(LaunchRuntime::Horizon.to_string(), "Horizon (Rust, optimized)");
        assert_eq!(LaunchRuntime::Java.to_string(), "Java (classic)");
    }
}
