# ADR-005: Configuration Management Strategy

**Status:** Accepted (Target Architecture)  
**Implementation Status:** Single-Phase Implementation  
**Target Implementation:** Weeks 1-16  
**Current Implementation:** Hexagonal architecture from start  
**Date:** 2024-01-01  
**Context:** Need centralized configuration management for user preferences, system settings, and runtime parameters.

## Decision

### Configuration Hierarchy
- **CLI Arguments** (highest priority): Override all other sources
- **Environment Variables**: System-wide settings with `TRIMX_*` prefix
- **Config File**: User preferences in `%APPDATA%/TrimX/config.toml`
- **Defaults**: Sensible defaults for all settings

### Configuration Sources
- **Config File**: TOML format for user preferences
- **Environment Variables**: `TRIMX_LOG_LEVEL`, `TRIMX_OUTPUT_FORMAT`, etc.
- **CLI Arguments**: Immediate override for all settings
- **Validation**: All configuration validated at startup

## Implementation Status

### Current Implementation (v0.1.0)
- **Status**: No configuration system implemented
- **Missing**: TOML config file support
- **Missing**: Environment variable handling
- **Missing**: Configuration validation
- **Missing**: User preference management

### Target Implementation (Phase 2)
- **Weeks 9-10**: Basic configuration system and TOML support
- **Weeks 11-12**: Environment variable handling and validation
- **Weeks 13-14**: Advanced configuration features and user preferences
- **Weeks 15-16**: Configuration testing and documentation

## Consequences

### Positive
- Flexible configuration hierarchy
- User-friendly defaults with override capability
- Clear validation and error reporting
- Environment-specific settings support
- Persistent user preferences

### Negative
- Multiple configuration sources to manage
- Potential for conflicting settings
- Need for clear precedence rules
- Configuration validation complexity

## Implementation Strategy

### Configuration Types

#### User Preferences (config.toml)
```toml
[logging]
level = "info"  # error, warn, info, debug, trace

[output]
default_format = "mp4"  # mp4, mkv, mov, same
default_codec = "h264"  # h264, hevc, copy
crf = 18               # 0-51, null for copy mode
preset = "medium"      # ultrafast, fast, medium, slow

[performance]
hardware_acceleration = true
max_memory_mb = 512
thread_count = 0       # 0 = auto-detect

[filesystem]
overwrite_policy = "prompt"  # prompt, always, never
temp_dir = ""               # empty = system default
long_path_support = true

[ui]
show_progress = true
json_output = false
verbose_errors = true
```

#### Environment Variables (TRIMX_*)
- `TRIMX_LOG_LEVEL`: Override logging level
- `TRIMX_OUTPUT_FORMAT`: Default output container format
- `TRIMX_DEFAULT_CODEC`: Default video codec
- `TRIMX_HARDWARE_ACCEL`: Enable/disable hardware acceleration
- `TRIMX_TEMP_DIR`: Custom temporary directory
- `TRIMX_MAX_MEMORY_MB`: Memory limit in MB
- `TRIMX_OVERWRITE_POLICY`: Default overwrite behavior

#### CLI Arguments (Highest Priority)
All CLI arguments override configuration file and environment variables:
- `--log-level`: Override logging level
- `--mode`: Override clipping strategy
- `--codec`: Override video codec
- `--crf`: Override quality setting
- `--preset`: Override encoding preset
- `--overwrite`: Override overwrite policy
- `--json`: Override output format

### Configuration Schema and Validation

#### Validation Rules
```rust
pub struct ConfigValidation {
    // Log level validation
    log_level: LogLevel,  // Must be valid enum value
    
    // Output format validation
    output_format: String,  // Must be supported container format
    codec: String,         // Must be supported video codec
    crf: Option<u8>,      // Must be 0-51 if specified
    preset: String,       // Must be valid preset name
    
    // Performance validation
    hardware_acceleration: bool,
    max_memory_mb: u32,   // Must be > 0 and < system memory
    thread_count: u32,    // Must be > 0, 0 means auto-detect
    
    // Filesystem validation
    overwrite_policy: OverwritePolicy,  // Must be valid enum
    temp_dir: Option<PathBuf>,         // Must exist and be writable
    long_path_support: bool,
    
    // UI validation
    show_progress: bool,
    json_output: bool,
    verbose_errors: bool,
}
```

#### Error Messages and Recovery
- **Invalid log level**: `Invalid log level 'xyz'. Valid options: error, warn, info, debug, trace`
- **Invalid output format**: `Unsupported output format 'xyz'. Supported formats: mp4, mkv, mov, same`
- **Invalid CRF value**: `CRF value 52 is out of range. Valid range: 0-51`
- **Invalid memory limit**: `Memory limit 999999MB exceeds system memory. Maximum: 4096MB`
- **Invalid temp directory**: `Temp directory '/invalid/path' does not exist or is not writable`

### Configuration Loading Process

1. **Load Defaults**: Set sensible defaults for all configuration options
2. **Load Config File**: Read `%APPDATA%/TrimX/config.toml` if it exists
3. **Load Environment**: Apply environment variables with `TRIMX_` prefix
4. **Load CLI Args**: Apply command-line arguments (highest priority)
5. **Validate**: Validate all configuration values
6. **Report Errors**: Show clear error messages for invalid settings

### Configuration Migration

#### Version Compatibility
- Configuration files include version field for compatibility
- Automatic migration for breaking changes
- Graceful handling of unknown configuration options
- Backup of original configuration before migration

#### Migration Examples
```toml
# v1.0.0 to v1.1.0: Add new hardware acceleration option
[performance]
hardware_acceleration = true  # New option with default value

# v1.1.0 to v1.2.0: Rename preset option
[output]
encoding_preset = "medium"    # Renamed from 'preset' to 'encoding_preset'
```

### Configuration Documentation

#### Auto-generated Documentation
- Configuration options documented in code comments
- CLI help shows configuration file location
- Environment variable documentation in README
- Configuration examples in user guide

#### User Guidance
- Clear examples for common use cases
- Performance tuning recommendations
- Troubleshooting configuration issues
- Best practices for different scenarios

## Related ADRs
- ADR-001: Architecture Style
- ADR-006: Logging & Observability Strategy
