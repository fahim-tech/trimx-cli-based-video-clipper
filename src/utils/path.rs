//! Path utilities for Windows-specific handling

use crate::error::{TrimXError, TrimXResult};
use std::path::{Path, PathBuf};

/// Path utilities for Windows-specific operations
pub struct PathUtils;

impl PathUtils {
    /// Create a new path utils instance
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }
}

impl Default for PathUtils {
    fn default() -> Self {
        Self::new()
    }
}

impl PathUtils {
    /// Convert path to Windows long path format if needed
    pub fn to_long_path(&self, path: &str) -> TrimXResult<String> {
        let path = Path::new(path);

        // Check if path is longer than 260 characters
        if path.to_string_lossy().len() > 260 {
            // Convert to long path format
            let long_path = format!("\\\\?\\{}", path.to_string_lossy());
            Ok(long_path)
        } else {
            Ok(path.to_string_lossy().to_string())
        }
    }

    /// Normalize path separators for Windows
    pub fn normalize_path(&self, path: &str) -> String {
        path.replace('/', "\\")
    }

    /// Get file extension from path
    pub fn get_extension(&self, path: &str) -> Option<String> {
        Path::new(path)
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
    }

    /// Get file stem (name without extension) from path
    pub fn get_stem(&self, path: &str) -> Option<String> {
        Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
    }

    /// Check if path is absolute
    pub fn is_absolute(&self, path: &str) -> bool {
        Path::new(path).is_absolute()
    }

    /// Join paths safely
    pub fn join_paths(&self, base: &str, relative: &str) -> TrimXResult<String> {
        let base_path = PathBuf::from(base);
        let joined = base_path.join(relative);

        Ok(joined.to_string_lossy().to_string())
    }

    /// Resolve relative path against base directory
    pub fn resolve_relative_path(&self, base: &str, relative: &str) -> TrimXResult<String> {
        let base_path = PathBuf::from(base);

        if !base_path.is_absolute() {
            return Err(TrimXError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Base path must be absolute",
            )));
        }

        let resolved = base_path
            .join(relative)
            .canonicalize()
            .map_err(TrimXError::IoError)?;

        Ok(resolved.to_string_lossy().to_string())
    }

    /// Validate file path
    pub fn validate_path(&self, path: &str) -> TrimXResult<()> {
        let path = Path::new(path);

        // Check for invalid characters
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        let path_str = path.to_string_lossy();

        for ch in invalid_chars {
            if path_str.contains(ch) {
                return Err(TrimXError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid character '{}' in path", ch),
                )));
            }
        }

        // Check for reserved names
        if let Some(stem) = path.file_stem() {
            let stem_str = stem.to_string_lossy().to_uppercase();
            let reserved_names = [
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ];

            if reserved_names.contains(&stem_str.as_str()) {
                return Err(TrimXError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Reserved name '{}' not allowed", stem_str),
                )));
            }
        }

        Ok(())
    }
}
