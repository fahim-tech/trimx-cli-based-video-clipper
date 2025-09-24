# ADR-006: Logging & Observability Strategy

**Status:** Accepted (Target Architecture)  
**Implementation Status:** Single-Phase Implementation  
**Target Implementation:** Weeks 1-16  
**Current Implementation:** Hexagonal architecture from start  
**Date:** 2024-01-01  
**Context:** Need comprehensive logging for debugging, monitoring, and user feedback.

## Decision

### Structured Logging
- **Framework**: Use `tracing` crate for structured logging
- **Log Levels**: `error`, `warn`, `info`, `debug`, `trace`
- **Output Formats**: Human-readable (default), JSON (`--json`), structured events
- **Performance**: Async logging to avoid blocking main thread

### Logging Context
- **Phase Information**: Current operation phase (probe, plan, execute, verify)
- **Timing Data**: Operation duration and performance metrics
- **Resource Usage**: Memory, CPU, disk I/O statistics
- **Error Context**: Detailed error information with recovery hints

## Implementation Status

### Current Implementation (v0.1.0)
- **Status**: Basic tracing setup in main.rs
- **Missing**: Structured logging events
- **Missing**: Performance metrics
- **Missing**: JSON output format
- **Missing**: Async logging

### Target Implementation (Phase 2)
- **Weeks 9-10**: Basic structured logging and event system
- **Weeks 11-12**: Performance metrics and resource monitoring
- **Weeks 13-14**: JSON output format and async logging
- **Weeks 15-16**: Advanced logging features and documentation

## Consequences

### Positive
- Rich debugging information for developers
- Automation-friendly structured output
- Performance monitoring capabilities
- User-friendly error messages
- Comprehensive audit trail

### Negative
- Increased complexity in logging setup
- Performance overhead for detailed logging
- Log file size management
- Privacy considerations for sensitive data

## Implementation Strategy

### Log Categories and Events

#### Application Events
```rust
// User actions
#[derive(Debug, Serialize)]
struct UserActionEvent {
    action: String,           // "clip", "inspect", "verify"
    input_file: String,
    output_file: Option<String>,
    parameters: HashMap<String, String>,
    timestamp: DateTime<Utc>,
}

// Configuration changes
#[derive(Debug, Serialize)]
struct ConfigChangeEvent {
    source: String,           // "cli", "env", "file", "default"
    changed_keys: Vec<String>,
    old_values: HashMap<String, String>,
    new_values: HashMap<String, String>,
    timestamp: DateTime<Utc>,
}
```

#### System Events
```rust
// File operations
#[derive(Debug, Serialize)]
struct FileOperationEvent {
    operation: String,        // "read", "write", "delete", "move"
    path: String,
    size_bytes: Option<u64>,
    duration_ms: u64,
    success: bool,
    error: Option<String>,
}

// Resource allocation
#[derive(Debug, Serialize)]
struct ResourceEvent {
    resource_type: String,    // "memory", "cpu", "disk", "gpu"
    action: String,           // "allocate", "deallocate", "usage"
    amount: u64,              // bytes, cpu cores, etc.
    peak_usage: Option<u64>,
    timestamp: DateTime<Utc>,
}
```

#### Performance Events
```rust
// Operation timing
#[derive(Debug, Serialize)]
struct PerformanceEvent {
    operation: String,        // "probe", "plan", "execute", "verify"
    duration_ms: u64,
    input_size_mb: f64,
    output_size_mb: Option<f64>,
    throughput_mbps: Option<f64>,
    memory_peak_mb: u64,
    cpu_usage_percent: f32,
}

// libav-specific events
#[derive(Debug, Serialize)]
struct LibavEvent {
    function: String,         // libav function name
    duration_ms: u64,
    packets_processed: u64,
    frames_processed: u64,
    codec: String,
    hardware_accel: bool,
    error_code: Option<i32>,
}
```

#### Error Events
```rust
// Error with context
#[derive(Debug, Serialize)]
struct ErrorEvent {
    error_type: String,       // "BadArgs", "ProbeFail", "ExecFail", etc.
    error_code: u8,           // Exit code
    message: String,
    hint: Option<String>,
    phase: String,            // "validation", "probe", "execute", "verify"
    context: HashMap<String, String>,
    stack_trace: Option<String>,
    recovery_attempted: bool,
    timestamp: DateTime<Utc>,
}
```

### Log Event Examples

#### Successful Clip Operation
```json
{
  "level": "info",
  "timestamp": "2024-01-15T10:30:45Z",
  "event": "operation_complete",
  "operation": "clip",
  "duration_ms": 1250,
  "input_file": "video.mp4",
  "output_file": "clip.mp4",
  "mode": "hybrid",
  "input_size_mb": 150.5,
  "output_size_mb": 45.2,
  "throughput_mbps": 120.4,
  "memory_peak_mb": 256,
  "cpu_usage_percent": 45.2
}
```

#### Error with Recovery
```json
{
  "level": "error",
  "timestamp": "2024-01-15T10:35:22Z",
  "event": "operation_failed",
  "error_type": "ExecFail",
  "error_code": 4,
  "message": "Hardware acceleration failed, falling back to software encoding",
  "hint": "Update graphics drivers or use --preset medium",
  "phase": "execute",
  "context": {
    "input_file": "video.mp4",
    "codec": "hevc",
    "hardware_accel": true,
    "fallback_successful": true
  },
  "recovery_attempted": true
}
```

#### Performance Monitoring
```json
{
  "level": "debug",
  "timestamp": "2024-01-15T10:30:45Z",
  "event": "performance_metrics",
  "operation": "execute",
  "libav_events": [
    {
      "function": "av_read_frame",
      "duration_ms": 15,
      "packets_processed": 1000,
      "throughput_mbps": 95.2
    },
    {
      "function": "av_interleaved_write_frame",
      "duration_ms": 8,
      "packets_processed": 1000,
      "throughput_mbps": 110.5
    }
  ],
  "memory_usage": {
    "current_mb": 180,
    "peak_mb": 256,
    "allocations": 45,
    "deallocations": 42
  }
}
```

### Output Management
- **Console Output**: Human-readable format for interactive use
- **JSON Output**: Structured format for automation and analysis
- **File Logging**: Optional persistent logging for debugging
- **Log Rotation**: Manage log file sizes and retention

### Performance Considerations

#### Async Logging Implementation
```rust
// Non-blocking log operations
use tracing::{info, warn, error, debug};
use tokio::sync::mpsc;

// Log channel for async processing
let (log_tx, mut log_rx) = mpsc::channel::<LogEvent>(1000);

// Spawn log processing task
tokio::spawn(async move {
    while let Some(event) = log_rx.recv().await {
        // Process log event asynchronously
        process_log_event(event).await;
    }
});
```

#### Performance Impact Measurements
- **Logging Overhead**: <1% CPU impact for info level
- **Memory Usage**: <10MB for typical operations
- **I/O Impact**: Minimal with async logging
- **Throughput**: <5% reduction with debug logging enabled

#### Configurable Log Levels and Performance
```rust
// Performance-optimized log levels
pub enum LogLevel {
    Error,   // Minimal overhead, critical issues only
    Warn,    // Low overhead, important warnings
    Info,    // Standard overhead, normal operations
    Debug,   // Higher overhead, detailed debugging
    Trace,   // Maximum overhead, verbose tracing
}

// Performance budgets per log level
const LOG_OVERHEAD_BUDGET: &[f32] = &[
    0.1,  // Error: 0.1% CPU overhead
    0.3,  // Warn: 0.3% CPU overhead  
    0.8,  // Info: 0.8% CPU overhead
    2.0,  // Debug: 2.0% CPU overhead
    5.0,  // Trace: 5.0% CPU overhead
];
```

#### Sampling and Filtering Strategies
```rust
// High-frequency event sampling
#[derive(Debug)]
struct SamplingConfig {
    sample_rate: f32,        // 0.0-1.0, percentage of events to log
    burst_threshold: u32,    // Events per second before sampling kicks in
    sample_window_ms: u64,   // Time window for sampling decisions
}

// Context-based filtering
#[derive(Debug)]
struct LogFilter {
    include_phases: Vec<String>,     // Only log specific phases
    exclude_operations: Vec<String>, // Skip certain operations
    min_duration_ms: u64,            // Only log operations above threshold
    include_error_context: bool,     // Include error context
}
```

#### Memory Management for Logging
```rust
// Bounded log buffer to prevent memory leaks
const MAX_LOG_BUFFER_SIZE: usize = 10000;
const MAX_LOG_MESSAGE_SIZE: usize = 1024;

// Log rotation and cleanup
#[derive(Debug)]
struct LogRotation {
    max_file_size_mb: u64,    // Rotate when file exceeds size
    max_files: u32,           // Keep maximum number of log files
    retention_days: u32,      // Delete logs older than N days
    compression: bool,        // Compress old log files
}
```

#### Performance Monitoring Integration
```rust
// Log performance metrics
#[derive(Debug, Serialize)]
struct LoggingPerformanceMetrics {
    events_logged: u64,
    events_dropped: u64,
    average_latency_ms: f64,
    memory_usage_mb: f64,
    cpu_overhead_percent: f32,
    io_operations: u64,
    errors: u64,
}

// Performance alerting thresholds
const PERFORMANCE_THRESHOLDS: LoggingPerformanceMetrics = LoggingPerformanceMetrics {
    events_logged: 0,
    events_dropped: 100,           // Alert if >100 events dropped
    average_latency_ms: 10.0,      // Alert if latency >10ms
    memory_usage_mb: 50.0,         // Alert if memory >50MB
    cpu_overhead_percent: 5.0,     // Alert if CPU >5%
    io_operations: 0,
    errors: 10,                    // Alert if >10 logging errors
};
```

## Related ADRs
- ADR-001: Architecture Style
- ADR-004: Error Taxonomy & Exit Codes
- ADR-005: Configuration Management Strategy
