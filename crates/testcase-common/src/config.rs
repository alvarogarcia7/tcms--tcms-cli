use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JsonEscapingMethod {
    #[serde(rename = "jq")]
    Jq,
    #[serde(rename = "rust_binary")]
    RustBinary,
    #[serde(rename = "shell_fallback")]
    ShellFallback,
    #[default]
    #[serde(rename = "auto")]
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JsonEscapingConfig {
    pub method: JsonEscapingMethod,
    pub enabled: bool,
    pub binary_path: Option<PathBuf>,
    pub jq_path: Option<PathBuf>,
}

impl Default for JsonEscapingConfig {
    fn default() -> Self {
        Self {
            method: JsonEscapingMethod::Auto,
            enabled: true,
            binary_path: None,
            jq_path: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScriptGenerationConfig {
    pub json_escaping: JsonEscapingConfig,
}

#[derive(Debug, Clone)]
pub struct EditorConfig {
    pub editor: Option<String>,
    pub visual: Option<String>,
    pub custom_fallback: Option<String>,
}

impl EditorConfig {
    pub fn load() -> Self {
        let original_vars = Self::save_env_vars();

        if Path::new(".env").exists() {
            let _ = dotenvy::from_filename(".env");
        }

        if Path::new(".env.local").exists() {
            let _ = dotenvy::from_filename(".env.local");
        }

        let editor = Self::get_with_precedence("EDITOR", &original_vars);
        let visual = Self::get_with_precedence("VISUAL", &original_vars);
        let custom_fallback = Self::get_with_precedence("CUSTOM_FALLBACK", &original_vars);

        EditorConfig {
            editor,
            visual,
            custom_fallback,
        }
    }

    fn save_env_vars() -> (Option<String>, Option<String>, Option<String>) {
        (
            env::var("EDITOR").ok(),
            env::var("VISUAL").ok(),
            env::var("CUSTOM_FALLBACK").ok(),
        )
    }

    fn get_with_precedence(
        key: &str,
        original_vars: &(Option<String>, Option<String>, Option<String>),
    ) -> Option<String> {
        let original = match key {
            "EDITOR" => &original_vars.0,
            "VISUAL" => &original_vars.1,
            "CUSTOM_FALLBACK" => &original_vars.2,
            _ => &None,
        };

        if original.is_some() {
            return original.clone();
        }

        env::var(key).ok()
    }

    pub fn get_editor(&self) -> Option<String> {
        self.visual
            .as_ref()
            .or(self.editor.as_ref())
            .or(self.custom_fallback.as_ref())
            .cloned()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitAuthorInfo {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CommitMessageTemplates {
    pub add_testcase: Option<String>,
    pub update_testcase: Option<String>,
    pub delete_testcase: Option<String>,
}

impl Default for CommitMessageTemplates {
    fn default() -> Self {
        Self {
            add_testcase: Some("Add test case: {name}".to_string()),
            update_testcase: Some("Update test case: {name}".to_string()),
            delete_testcase: Some("Delete test case: {name}".to_string()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_database_path: Option<PathBuf>,
    pub default_device_name: Option<String>,
    pub git_author: GitAuthorInfo,
    pub commit_templates: CommitMessageTemplates,
    pub script_generation: ScriptGenerationConfig,
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .context("Could not determine home directory")?;
        Ok(PathBuf::from(home).join(".testcase-manager"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        Self::load_from_path(&config_path)
    }

    /// Load configuration from an explicit TOML file path (does not consult the
    /// default `~/.testcase-manager/config.toml`).
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let contents =
            fs::read_to_string(path).context(format!("Failed to read config file: {:?}", path))?;

        let config: Config = toml::from_str(&contents).context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        fs::create_dir_all(&config_dir).context(format!(
            "Failed to create config directory: {:?}",
            config_dir
        ))?;

        let config_path = Self::config_path()?;
        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, contents)
            .context(format!("Failed to write config file: {:?}", config_path))?;

        Ok(())
    }

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }
}
