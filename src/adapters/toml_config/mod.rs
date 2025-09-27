// TOML config adapter - Configuration management using TOML files

use crate::domain::errors::*;
use crate::ports::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// TOML configuration adapter
pub struct TomlConfigAdapter {
    config: Arc<RwLock<HashMap<String, String>>>,
    config_file_path: Arc<RwLock<Option<PathBuf>>>,
}

impl TomlConfigAdapter {
    /// Create new TOML config adapter
    pub fn new() -> Result<Self, DomainError> {
        let adapter = Self {
            config: Arc::new(RwLock::new(HashMap::new())),
            config_file_path: Arc::new(RwLock::new(None)),
        };

        // Load default configuration synchronously for now
        // TODO: Make this properly async
        {
            let mut config = adapter.config.write().unwrap();
            config.insert("log_level".to_string(), "info".to_string());
            config.insert("overwrite_policy".to_string(), "prompt".to_string());
            config.insert("thread_count".to_string(), "4".to_string());
            config.insert("buffer_size".to_string(), "4096".to_string());
        } // Drop the lock

        Ok(adapter)
    }

    /// Load default configuration values
    async fn load_default_config(&self) -> Result<(), DomainError> {
        let mut config = self.config.write().unwrap();
        // Set default configuration values
        config.insert("log_level".to_string(), "info".to_string());
        config.insert("output_format".to_string(), "mp4".to_string());
        config.insert("codec_preset".to_string(), "medium".to_string());
        config.insert("crf".to_string(), "18".to_string());
        config.insert("hardware_acceleration".to_string(), "false".to_string());
        config.insert("overwrite_policy".to_string(), "prompt".to_string());

        Ok(())
    }

    /// Get default config file path
    fn get_default_config_path() -> PathBuf {
        // On Windows, use %APPDATA%/TrimX/config.toml
        if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata).join("TrimX").join("config.toml")
        } else {
            // Fallback to current directory
            PathBuf::from("trimx_config.toml")
        }
    }

    /// Serialize config to TOML string
    fn serialize_config(&self) -> Result<String, DomainError> {
        let config = self.config.read().unwrap();
        let mut toml_string = String::new();
        toml_string.push_str("[trimx]\n");

        for (key, value) in config.iter() {
            toml_string.push_str(&format!("{} = \"{}\"\n", key, value));
        }

        Ok(toml_string)
    }

    /// Deserialize config from TOML string
    fn deserialize_config(&self, toml_content: &str) -> Result<(), DomainError> {
        let parsed: toml::Value = toml::from_str(toml_content)
            .map_err(|e| DomainError::BadArgs(format!("Failed to parse TOML config: {}", e)))?;

        let mut config = self.config.write().unwrap();
        if let Some(trimx_section) = parsed.get("trimx") {
            if let Some(table) = trimx_section.as_table() {
                for (key, value) in table {
                    if let Some(str_value) = value.as_str() {
                        config.insert(key.clone(), str_value.to_string());
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ConfigPort for TomlConfigAdapter {
    async fn get_config(&self, key: &str) -> Result<Option<String>, DomainError> {
        let config = self.config.read().unwrap();
        Ok(config.get(key).cloned())
    }

    async fn get_config_or_default(&self, key: &str, default: &str) -> Result<String, DomainError> {
        let config = self.config.read().unwrap();
        Ok(config
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string()))
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<(), DomainError> {
        let mut config = self.config.write().unwrap();
        config.insert(key.to_string(), value.to_string());
        tracing::info!("Set config {} = {}", key, value);
        Ok(())
    }

    async fn load_config(&self, file_path: &str) -> Result<(), DomainError> {
        let path = PathBuf::from(file_path);

        if !path.exists() {
            return Err(DomainError::FsFail(format!(
                "Config file does not exist: {}",
                file_path
            )));
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| DomainError::FsFail(format!("Failed to read config file: {}", e)))?;

        self.deserialize_config(&content)?;
        let mut config_path = self.config_file_path.write().unwrap();
        *config_path = Some(path);

        Ok(())
    }

    async fn save_config(&self, file_path: &str) -> Result<(), DomainError> {
        let path = PathBuf::from(file_path);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DomainError::FsFail(format!("Failed to create config directory: {}", e))
            })?;
        }

        let content = self.serialize_config()?;
        std::fs::write(&path, content)
            .map_err(|e| DomainError::FsFail(format!("Failed to write config file: {}", e)))?;

        let mut config_path = self.config_file_path.write().unwrap();
        *config_path = Some(path);
        Ok(())
    }

    async fn load_default_config(&self) -> Result<(), DomainError> {
        let mut config = self.config.write().unwrap();
        // Set default configuration values
        config.insert("log_level".to_string(), "info".to_string());
        config.insert("output_format".to_string(), "mp4".to_string());
        config.insert("codec_preset".to_string(), "medium".to_string());
        config.insert("crf".to_string(), "18".to_string());
        config.insert("hardware_acceleration".to_string(), "false".to_string());
        config.insert("overwrite_policy".to_string(), "prompt".to_string());

        Ok(())
    }

    async fn validate_config(&self) -> Result<(), DomainError> {
        let config = self.config.read().unwrap();

        // Validate log level
        if let Some(log_level) = config.get("log_level") {
            LogLevel::parse(log_level)?;
        }

        // Validate CRF value
        if let Some(crf) = config.get("crf") {
            let crf_value: u8 = crf
                .parse()
                .map_err(|e| DomainError::BadArgs(format!("Invalid CRF value: {}", e)))?;
            if crf_value > 51 {
                return Err(DomainError::BadArgs(
                    "CRF value cannot exceed 51".to_string(),
                ));
            }
        }

        // Validate boolean values
        for (key, value) in config.iter() {
            if key == "hardware_acceleration" {
                value.parse::<bool>().map_err(|e| {
                    DomainError::BadArgs(format!("Invalid boolean value for {}: {}", key, e))
                })?;
            }
        }

        Ok(())
    }

    async fn get_all_config_keys(&self) -> Result<Vec<String>, DomainError> {
        let config = self.config.read().unwrap();
        Ok(config.keys().cloned().collect())
    }

    async fn clear_config(&self) -> Result<(), DomainError> {
        {
            let mut config = self.config.write().unwrap();
            config.clear();
        } // Drop the lock before await
        self.load_default_config().await?;
        Ok(())
    }

    async fn get_config_file_path(&self) -> Result<String, DomainError> {
        let config_path = self.config_file_path.read().unwrap();
        if let Some(path) = config_path.as_ref() {
            Ok(path.to_string_lossy().to_string())
        } else {
            Ok(Self::get_default_config_path()
                .to_string_lossy()
                .to_string())
        }
    }
}
