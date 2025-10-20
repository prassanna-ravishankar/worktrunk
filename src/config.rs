use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for worktree path formatting and LLM integration.
///
/// The `worktree-path` template is relative to the repository root and supports:
/// - `{repo}` - Repository name
/// - `{branch}` - Branch name (slashes replaced with dashes)
///
/// # Examples
///
/// ```toml
/// # Default - parent directory siblings
/// worktree-path = "../{repo}.{branch}"
///
/// # Inside repo (bare repository style)
/// worktree-path = "{branch}"
///
/// # Organized in .worktrees subdirectory
/// worktree-path = ".worktrees/{branch}"
///
/// # Repository-namespaced shared directory (avoids conflicts)
/// worktree-path = "../worktrees/{repo}/{branch}"
///
/// # LLM configuration for commit message generation
/// [llm]
/// command = "llm"  # Command to invoke LLM (e.g., "llm", "claude")
/// args = ["-s"]    # Arguments to pass to the command
/// ```
///
/// Config file location:
/// - Linux: `~/.config/worktrunk/config.toml`
/// - macOS: `~/Library/Application Support/worktrunk/config.toml`
/// - Windows: `%APPDATA%\worktrunk\config.toml`
///
/// Environment variable: `WORKTRUNK_WORKTREE_PATH`
#[derive(Debug, Serialize, Deserialize)]
pub struct WorktrunkConfig {
    #[serde(rename = "worktree-path")]
    pub worktree_path: String,

    #[serde(default)]
    pub llm: LlmConfig,
}

/// Configuration for LLM integration
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LlmConfig {
    /// Command to invoke LLM (e.g., "llm", "claude")
    #[serde(default)]
    pub command: Option<String>,

    /// Arguments to pass to the LLM command
    #[serde(default)]
    pub args: Vec<String>,
}

impl Default for WorktrunkConfig {
    fn default() -> Self {
        Self {
            worktree_path: "../{repo}.{branch}".to_string(),
            llm: LlmConfig::default(),
        }
    }
}

impl WorktrunkConfig {
    /// Load configuration from config file and environment variables.
    ///
    /// Configuration is loaded in the following order (later sources override earlier ones):
    /// 1. Default values
    /// 2. Config file (~/.config/worktrunk/config.toml on Linux/macOS)
    /// 3. Environment variables (WORKTRUNK_*)
    pub fn load() -> Result<Self, ConfigError> {
        let defaults = Self::default();

        let mut builder = Config::builder()
            .set_default("worktree-path", defaults.worktree_path)?
            .set_default("llm.command", defaults.llm.command.unwrap_or_default())?
            .set_default("llm.args", defaults.llm.args)?;

        // Add config file if it exists
        if let Some(config_path) = get_config_path()
            && config_path.exists()
        {
            builder = builder.add_source(File::from(config_path));
        }

        // Add environment variables with WORKTRUNK prefix
        builder = builder.add_source(config::Environment::with_prefix("WORKTRUNK").separator("_"));

        let config: Self = builder.build()?.try_deserialize()?;
        validate_worktree_path(&config.worktree_path)?;
        Ok(config)
    }

    /// Format a worktree path using this configuration's template.
    ///
    /// # Arguments
    /// * `repo` - Repository name (replaces {repo} in template)
    /// * `branch` - Branch name (replaces {branch} in template, slashes sanitized to dashes)
    ///
    /// # Examples
    /// ```
    /// use worktrunk::config::WorktrunkConfig;
    ///
    /// let config = WorktrunkConfig::default();
    /// let path = config.format_path("myproject", "feature/foo");
    /// assert_eq!(path, "../myproject.feature-foo");
    /// ```
    pub fn format_path(&self, repo: &str, branch: &str) -> String {
        // Sanitize branch name by replacing path separators to prevent directory traversal
        let safe_branch = branch.replace(['/', '\\'], "-");
        self.worktree_path
            .replace("{repo}", repo)
            .replace("{branch}", &safe_branch)
    }
}

fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "worktrunk").map(|dirs| dirs.config_dir().join("config.toml"))
}

fn validate_worktree_path(template: &str) -> Result<(), ConfigError> {
    if template.is_empty() {
        return Err(ConfigError::Message(
            "worktree-path cannot be empty".to_string(),
        ));
    }

    // Reject absolute paths
    let path = std::path::Path::new(template);
    if path.is_absolute() {
        return Err(ConfigError::Message(
            "worktree-path must be relative, not absolute".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorktrunkConfig::default();
        assert_eq!(config.worktree_path, "../{repo}.{branch}");
    }

    #[test]
    fn test_config_serialization() {
        let config = WorktrunkConfig::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("worktree-path"));
        assert!(toml.contains("../{repo}.{branch}"));
    }

    #[test]
    fn test_load_config_defaults() {
        // Without a config file or env vars, should return defaults
        let config = WorktrunkConfig::load().unwrap();
        assert_eq!(config.worktree_path, "../{repo}.{branch}");
    }

    #[test]
    fn test_format_worktree_path() {
        let config = WorktrunkConfig {
            worktree_path: "{repo}.{branch}".to_string(),
            llm: LlmConfig::default(),
        };
        assert_eq!(
            config.format_path("myproject", "feature-x"),
            "myproject.feature-x"
        );
    }

    #[test]
    fn test_format_worktree_path_custom_template() {
        let config = WorktrunkConfig {
            worktree_path: "{repo}-{branch}".to_string(),
            llm: LlmConfig::default(),
        };
        assert_eq!(
            config.format_path("myproject", "feature-x"),
            "myproject-feature-x"
        );
    }

    #[test]
    fn test_format_worktree_path_only_branch() {
        let config = WorktrunkConfig {
            worktree_path: "{branch}".to_string(),
            llm: LlmConfig::default(),
        };
        assert_eq!(config.format_path("myproject", "feature-x"), "feature-x");
    }

    #[test]
    fn test_format_worktree_path_with_slashes() {
        // Slashes should be replaced with dashes to prevent directory traversal
        let config = WorktrunkConfig {
            worktree_path: "{repo}.{branch}".to_string(),
            llm: LlmConfig::default(),
        };
        assert_eq!(
            config.format_path("myproject", "feature/foo"),
            "myproject.feature-foo"
        );
    }

    #[test]
    fn test_format_worktree_path_with_multiple_slashes() {
        let config = WorktrunkConfig {
            worktree_path: "{branch}".to_string(),
            llm: LlmConfig::default(),
        };
        assert_eq!(
            config.format_path("myproject", "feature/sub/task"),
            "feature-sub-task"
        );
    }

    #[test]
    fn test_format_worktree_path_with_backslashes() {
        // Windows-style path separators should also be sanitized
        let config = WorktrunkConfig {
            worktree_path: "{branch}".to_string(),
            llm: LlmConfig::default(),
        };
        assert_eq!(
            config.format_path("myproject", "feature\\foo"),
            "feature-foo"
        );
    }

    #[test]
    fn test_validate_rejects_empty_path() {
        let result = validate_worktree_path("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_rejects_absolute_path_unix() {
        let result = validate_worktree_path("/absolute/path/{branch}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be relative"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_validate_rejects_absolute_path_windows() {
        let result = validate_worktree_path("C:\\absolute\\path\\{branch}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be relative"));
    }

    #[test]
    fn test_validate_accepts_relative_path() {
        assert!(validate_worktree_path(".worktrees/{branch}").is_ok());
        assert!(validate_worktree_path("../{repo}.{branch}").is_ok());
        assert!(validate_worktree_path("../../shared/{branch}").is_ok());
    }
}
