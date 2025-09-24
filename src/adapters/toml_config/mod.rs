// TOML config adapter - Configuration management using TOML files

use crate::domain::model::*;
use crate::domain::errors::*;
use crate::ports::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

/// TOML configuration adapter
pub struct TomlConfigAdapter {
    config: HashMap<String, String>,
    config_file_path: Option<PathBuf>,
}

impl TomlConfigAdapter {
    /// Create new TOML config adapter
    pub fn new() -> Result<Self, DomainError> {
        let mut adapter = Self {
            config: HashMap::new(),
            config_file_path: None,
        };
        
        // Load default configuration
        adapter.load_default_config().await?;
        
        Ok(adapter)
    }
    
    /// Load default configuration values
    async fn load_default_config(&mut self) -> Result<(), DomainError> {
        // Set default configuration values
        self.config.insert("log_level".to_string(), "info".to_string());
        self.config.insert("output_format".to_string(), "mp4".to_string());
        self.config.insert("codec_preset".to_string(), "medium".to_string());
        self.config.insert("crf".to_string(), "18".to_string());
        self.config.insert("hardware_acceleration".to_string(), "false".to_string());
        self.config.insert("overwrite_policy".to_string(), "prompt".to_string());
        
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
        let mut toml_string = String::new();
        toml_string.push_str("[trimx]\n");
        
        for (key, value) in &self.config {
            toml_string.push_str(&format!("{} = \"{}\"\n", key, value));
        }
        
        Ok(toml_string)
    }
    
    /// Deserialize config from TOML string
    fn deserialize_config(&mut self, toml_content: &str) -> Result<(), DomainError> {
        let parsed: toml::Value = toml::from_str(toml_content)
            .map_err(|e| DomainError::BadArgs(format!("Failed to parse TOML config: {}", e)))?;
        
        if let Some(trimx_section) = parsed.get("trimx") {
            if let Some(table) = trimx_section.as_table() {
                for (key, value) in table {
                    if let Some(str_value) = value.as_str() {
                        self.config.insert(key.clone(), str_value.to_string());
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
        Ok(self.config.get(key).cloned())
    }
    
    async fn get_config_or_default(&self, key: &str, default: &str) -> Result<String, DomainError> {
        Ok(self.config.get(key).cloned().unwrap_or_else(|| default.to_string()))
    }
    
    async fn set_config(&mut self, key: &str, value: &str) -> Result<(), DomainError> {
        self.config.insert(key.to_string(), value.to_string());
        Ok(())
    }
    
    async fn load_config(&mut self, file_path: &str) -> Result<(), DomainError> {
        let path = PathBuf::from(file_path);
        
        if !path.exists() {
            return Err(DomainError::FsFail(format!("Config file does not exist: {}", file_path)));
        }
        
        let content = std::fs::read_to_string(&path)
            .map_err(|e| DomainError::FsFail(format!("Failed to read config file: {}", e)))?;
        
        self.deserialize_config(&content)?;
        self.config_file_path = Some(path);
        
        Ok(())
    }
    
    async fn save_config(&mut self, file_path: &str) -> Result<(), DomainError> {
        let path = PathBuf::from(file_path);
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DomainError::FsFail(format!("Failed to create config directory: {}", e)))?;
        }
        
        let content = self.serialize_config()?;
        std::fs::write(&path, content)
            .map_err(|e| DomainError::FsFail(format!("Failed to write config file: {}", e)))?;
        
        self.config_file_path = Some(path);
        Ok(())
    }
    
    async fn load_default_config(&mut self) -> Result<(), DomainError> {
        self.load_default_config().await
    }
    
    async fn validate_config(&self) -> Result<(), DomainError> {
        // Validate log level
        if let Some(log_level) = self.config.get("log_level") {
            LogLevel::parse(log_level)?;
        }
        
        // Validate CRF value
        if let Some(crf) = self.config.get("crf") {
            let crf_value: u8 = crf.parse()
                .map_err(|e| DomainError::BadArgs(format!("Invalid CRF value: {}", e)))?;
            if crf_value > 51 {
                return Err(DomainError::BadArgs("CRF value cannot exceed 51".to_string()));
            }
        }
        
        // Validate boolean values
        for (key, value) in &self.config {
            if key == "hardware_acceleration" {
                value.parse::<bool>()
                    .map_err(|e| DomainError::BadArgs(format!("Invalid boolean value for {}: {}", key, e)))?;
            }
        }
        
        Ok(())
    }
    
    async fn get_all_config_keys(&self) -> Result<Vec<String>, DomainError> {
        Ok(self.config.keys().cloned().collect())
    }
    
    async fn clear_config(&mut self) -> Result<(), DomainError> {
        self.config.clear();
        self.load_default_config().await?;
        Ok(())
    }
    
    async fn get_config_file_path(&self) -> Result<String, DomainError> {
        if let Some(path) = &self.config_file_path {
            Ok(path.to_string_lossy().to_string())
        } else {
            Ok(Self::get_default_config_path().to_string_lossy().to_string())
        }
    }
}