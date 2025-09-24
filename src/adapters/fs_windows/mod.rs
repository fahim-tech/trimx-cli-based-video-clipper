// Windows filesystem adapter - File system operations for Windows

use crate::domain::model::*;
use crate::domain::errors::*;
use crate::ports::*;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;

/// Windows filesystem adapter
pub struct FsWindowsAdapter {
    temp_dir: PathBuf,
}

impl FsWindowsAdapter {
    /// Create new Windows filesystem adapter
    pub fn new() -> Result<Self, DomainError> {
        let temp_dir = std::env::temp_dir().join("trimx");
        
        // Ensure temp directory exists
        if let Err(e) = fs::create_dir_all(&temp_dir) {
            return Err(DomainError::FsFail(format!("Failed to create temp directory: {}", e)));
        }
        
        Ok(Self { temp_dir })
    }
    
    /// Convert path to Windows long-path format if needed
    fn to_long_path(path: &str) -> String {
        let path = Path::new(path);
        if path.to_string_lossy().len() > 260 {
            // Use Windows long-path prefix
            format!("\\\\?\\{}", path.to_string_lossy())
        } else {
            path.to_string_lossy().to_string()
        }
    }
    
    /// Get file metadata
    fn get_file_metadata_internal(file_path: &str) -> Result<FileMetadata, DomainError> {
        let metadata = fs::metadata(file_path)
            .map_err(|e| DomainError::FsFail(format!("Failed to get file metadata: {}", e)))?;
        
        Ok(FileMetadata {
            size: metadata.len(),
            created: metadata.created().ok(),
            modified: metadata.modified().ok(),
            accessed: metadata.accessed().ok(),
            is_readonly: metadata.permissions().readonly(),
            is_hidden: false, // Would need Windows-specific code to detect hidden files
        })
    }
}

#[async_trait]
impl FsPort for FsWindowsAdapter {
    async fn file_exists(&self, file_path: &str) -> Result<bool, DomainError> {
        Ok(Path::new(file_path).exists())
    }
    
    async fn directory_exists(&self, dir_path: &str) -> Result<bool, DomainError> {
        let path = Path::new(dir_path);
        Ok(path.exists() && path.is_dir())
    }
    
    async fn get_file_size(&self, file_path: &str) -> Result<u64, DomainError> {
        let metadata = fs::metadata(file_path)
            .map_err(|e| DomainError::FsFail(format!("Failed to get file size: {}", e)))?;
        Ok(metadata.len())
    }
    
    async fn get_file_metadata(&self, file_path: &str) -> Result<FileMetadata, DomainError> {
        Self::get_file_metadata_internal(file_path)
    }
    
    async fn create_directory(&self, dir_path: &str) -> Result<(), DomainError> {
        fs::create_dir_all(dir_path)
            .map_err(|e| DomainError::FsFail(format!("Failed to create directory: {}", e)))?;
        Ok(())
    }
    
    async fn create_output_file(&self, file_path: &str) -> Result<(), DomainError> {
        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(file_path).parent() {
            self.create_directory(parent.to_string_lossy().as_ref()).await?;
        }
        
        // Create empty file
        fs::File::create(file_path)
            .map_err(|e| DomainError::FsFail(format!("Failed to create output file: {}", e)))?;
        Ok(())
    }
    
    async fn create_temp_file(&self, prefix: &str, suffix: &str) -> Result<String, DomainError> {
        let temp_file = tempfile::Builder::new()
            .prefix(prefix)
            .suffix(suffix)
            .tempfile_in(&self.temp_dir)
            .map_err(|e| DomainError::FsFail(format!("Failed to create temp file: {}", e)))?;
        
        let temp_path = temp_file.path().to_string_lossy().to_string();
        Ok(temp_path)
    }
    
    async fn delete_file(&self, file_path: &str) -> Result<(), DomainError> {
        fs::remove_file(file_path)
            .map_err(|e| DomainError::FsFail(format!("Failed to delete file: {}", e)))?;
        Ok(())
    }
    
    async fn delete_directory(&self, dir_path: &str) -> Result<(), DomainError> {
        fs::remove_dir_all(dir_path)
            .map_err(|e| DomainError::FsFail(format!("Failed to delete directory: {}", e)))?;
        Ok(())
    }
    
    async fn move_file(&self, from: &str, to: &str) -> Result<(), DomainError> {
        // Ensure destination directory exists
        if let Some(parent) = Path::new(to).parent() {
            self.create_directory(parent.to_string_lossy().as_ref()).await?;
        }
        
        // Move file atomically
        fs::rename(from, to)
            .map_err(|e| DomainError::FsFail(format!("Failed to move file from {} to {}: {}", from, to, e)))?;
        Ok(())
    }
    
    async fn copy_file(&self, from: &str, to: &str) -> Result<(), DomainError> {
        // Ensure destination directory exists
        if let Some(parent) = Path::new(to).parent() {
            self.create_directory(parent.to_string_lossy().as_ref()).await?;
        }
        
        fs::copy(from, to)
            .map_err(|e| DomainError::FsFail(format!("Failed to copy file from {} to {}: {}", from, to, e)))?;
        Ok(())
    }
    
    async fn get_available_space(&self, dir_path: &str) -> Result<u64, DomainError> {
        // For now, return a large number - in real implementation, this would use Windows API
        Ok(1024 * 1024 * 1024) // 1 GB
    }
    
    async fn validate_path(&self, file_path: &str) -> Result<bool, DomainError> {
        let path = Path::new(file_path);
        
        // Check for path traversal attacks
        if path.to_string_lossy().contains("..") {
            return Ok(false);
        }
        
        // Check for invalid characters
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        if path.to_string_lossy().chars().any(|c| invalid_chars.contains(&c)) {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    async fn resolve_path(&self, file_path: &str) -> Result<String, DomainError> {
        let path = Path::new(file_path);
        let canonical_path = path.canonicalize()
            .map_err(|e| DomainError::FsFail(format!("Failed to resolve path: {}", e)))?;
        Ok(canonical_path.to_string_lossy().to_string())
    }
    
    async fn can_write_to_directory(&self, dir_path: &str) -> Result<bool, DomainError> {
        // Check if directory exists and is writable
        if !self.directory_exists(dir_path).await? {
            return Ok(false);
        }
        
        // Try to create a test file
        let test_file = Path::new(dir_path).join(".trimx_test");
        match fs::File::create(&test_file) {
            Ok(_) => {
                // Clean up test file
                let _ = fs::remove_file(&test_file);
                Ok(true)
            },
            Err(_) => Ok(false),
        }
    }
}