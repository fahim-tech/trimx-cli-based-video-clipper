# TrimX CLI Video Clipper - Architecture Documentation

## 🚧 Architecture Status

**Current Implementation**: Hexagonal architecture from start  
**Target Architecture**: Hexagonal (Ports & Adapters) with Clean Layers  
**Implementation Plan**: Single Phase (Weeks 1-16)  
**Status**: Implementing hexagonal architecture from beginning

> ⚠️ **Important**: This document describes the hexagonal architecture being implemented from the start. See [ADR-007](ADRs/ADR-007-architecture-implementation-strategy.md) for implementation strategy.

## Overview

TrimX is a Windows-native command-line tool for precise video clipping built using **Hexagonal Architecture (Ports & Adapters)** with **Clean Layer** separation. This architecture enables direct libav FFI integration for optimal performance and precise control.

## Current Implementation

### Hexagonal Architecture Structure
```
src/
├── cli/           # Primary adapter (Command-line interface)
├── domain/        # Pure business logic (no external dependencies)
│   ├── model/     # Core types and data structures
│   ├── rules/     # Business rules and policies
│   ├── usecases/  # Use case orchestration
│   └── errors/    # Domain error types
├── app/           # Application layer (Use case interactors)
│   ├── clip_interactor/
│   ├── inspect_interactor/
│   └── verify_interactor/
├── ports/         # Interface definitions (contracts)
│   ├── probe_port.rs
│   ├── execute_port.rs
│   ├── fs_port.rs
│   ├── config_port.rs
│   └── log_port.rs
└── adapters/      # External system implementations
    ├── probe_libav.rs
    ├── exec_libav.rs
    ├── fs_windows.rs
    ├── tracing_log.rs
    └── toml_config.rs
```

**Characteristics**:
- Clean separation of concerns
- Pure domain logic with no external dependencies
- Testable business logic with mock implementations
- Flexible adapter swapping for different implementations
- Better long-term maintainability

## Implementation Strategy

### Single Phase: Hexagonal Architecture (Weeks 1-16)
- **Goal**: Complete implementation with hexagonal architecture from start
- **Focus**: All features implemented using hexagonal patterns
- **Architecture**: Hexagonal (Ports & Adapters) with Clean Layers
- **Benefits**: Clean start, consistent patterns, no technical debt

### Implementation Benefits
- **Immediate**: Clean hexagonal architecture from day one
- **Long-term**: Better maintainability and testability
- **Risk Mitigation**: No migration complexity or technical debt
- **Consistency**: All code follows same architectural patterns

See [ADR-007](ADRs/ADR-007-architecture-implementation-strategy.md) for detailed implementation strategy.

### ADR-001: Architecture Style — Hexagonal (Ports & Adapters) with Clean Layers

**Status:** Accepted  
**Context:** Windows-first Rust CLI for local video clipping (--start, --end). Needs precise control, deterministic behavior, and direct libav FFI integration for optimal performance.

**Decision:** Use Hexagonal Architecture (aka Ports & Adapters) applied with Clean layering:
- **Domain** (pure) → **Application** (orchestration) → **Ports** (interfaces) → **Adapters** (IO implementations)

**Consequences:**
- Domain stays pure and testable
- libav FFI integration provides precise control
- Packaging, OS quirks, and installers are isolated to adapters
- Clean separation enables comprehensive testing

### ADR-002: Execution Backend Strategy — Direct libav FFI Integration

**Status:** Accepted  
**Context:** Direct integration with libav libraries for precise control and optimal performance.

**Decision:**
- **Direct libav FFI:** Use in-process libav libraries (libavformat/libavcodec/libavfilter/libswresample) for all media processing
- **No external dependencies:** Eliminate external ffmpeg.exe/ffprobe.exe processes
- **Hardware acceleration:** Built-in support for NVENC/QSV through libav

**Consequences:**
- Precise timestamp control and fewer temp files
- Single binary with no external dependencies
- Increased build complexity but better performance
- Direct access to all libav features

### ADR-003: Core Domain Policies — Timebase, Keyframe, and Plan Selection

**Status:** Accepted  
**Context:** Accurate cuts with reliable defaults while preserving speed when possible.

**Decision:**
- Normalize all timing in stream timebase (PTS); avoid frame counts
- **Keyframe proximity rule:** copy allowed if `distance_to_prev_keyframe ≤ ε`, where `ε ≈ 0.5 * avg_frame_time`, and container copy is safe
- **Plan modes:**
  - Copy (fast, approximate but safe when aligned)
  - Hybrid ("sandwich": re-encode head/tail GOPs, copy middle)
  - Reencode (exact, fallback)
- Preserve rotation/color metadata; MP4 outputs must have faststart

**Consequences:** Predictable accuracy with performance wins when alignment allows.

### ADR-004: Error Taxonomy & Exit Codes

**Status:** Accepted  
**Context:** Clear automation-friendly errors with stable exit codes.

**Decision:**
- **Exit codes:**
  - 0: success
  - 2: invalid arguments
  - 3: probe failure
  - 4: execution/clip failure
  - 5: write/FS failure
- **Error classes (domain-level):** BadArgs, OutOfRange, ProbeFail, PlanUnsupported, ExecFail, FsFail, VerifyFail
- **Reporting:** human-readable by default; `--json` emits `{code, message, hint, phase, ctx}`

**Consequences:** Scripts and CI can reliably branch on outcomes; users get actionable hints.

### ADR-005: Configuration Management Strategy

**Status:** Accepted  
**Context:** Need centralized configuration management for user preferences, system settings, and runtime parameters.

**Decision:**
- **Configuration Sources**: CLI args (highest priority) → Environment variables → Config file → Defaults
- **Config File**: `%APPDATA%/TrimX/config.toml` for user preferences
- **Environment Variables**: `TRIMX_*` prefix for system-wide settings
- **Validation**: All configuration validated at startup with clear error messages

**Consequences:**
- Flexible configuration hierarchy
- User-friendly defaults with override capability
- Clear validation and error reporting

### ADR-006: Logging & Observability Strategy

**Status:** Accepted  
**Context:** Need comprehensive logging for debugging, monitoring, and user feedback.

**Decision:**
- **Structured Logging**: Use `tracing` crate with structured fields
- **Log Levels**: `error`, `warn`, `info`, `debug`, `trace`
- **Output Formats**: Human-readable (default), JSON (`--json`), and structured events
- **Performance**: Async logging to avoid blocking main thread
- **Context**: Include phase, operation, timing, and resource usage

**Consequences:**
- Rich debugging information
- Automation-friendly structured output
- Performance monitoring capabilities

## Ports & Interfaces

### ProbePort
```rust
pub trait ProbePort {
    async fn probe(&self, path: &Path) -> Result<MediaInfo, ProbeError>;
    async fn probe_streams(&self, path: &Path) -> Result<Vec<StreamInfo>, ProbeError>;
    async fn probe_keyframes(&self, path: &Path, stream_index: usize) -> Result<Vec<KeyframeInfo>, ProbeError>;
}
```
- **Input:** file path
- **Output:** `MediaInfo { container, duration, streams: [StreamInfo] }`
- **Errors:** ProbeFail (maps to exit 3)
- **libav Functions:** `avformat_open_input`, `avformat_find_stream_info`, `av_read_frame`

### ExecutePort
```rust
pub trait ExecutePort {
    async fn execute(&self, plan: ExecutionPlan) -> Result<OutputReport, ExecError>;
    async fn execute_copy(&self, plan: CopyPlan) -> Result<OutputReport, ExecError>;
    async fn execute_hybrid(&self, plan: HybridPlan) -> Result<OutputReport, ExecError>;
    async fn execute_reencode(&self, plan: ReencodePlan) -> Result<OutputReport, ExecError>;
}
```
- **Input:** `Plan { mode: Copy|Hybrid|Reencode, cut range, stream map, container out }`
- **Output:** `OutputReport { ok, first_pts, last_pts, duration_out, warnings[] }`
- **Errors:** ExecFail (exit 4)
- **libav Functions:** `avformat_alloc_output_context2`, `avcodec_open2`, `av_interleaved_write_frame`

### FsPort
```rust
pub trait FsPort {
    async fn prepare_temp(&self, output_path: &Path) -> Result<PathBuf, FsError>;
    async fn commit_temp(&self, temp_path: &Path, final_path: &Path) -> Result<(), FsError>;
    async fn resolve_path(&self, path: &str) -> Result<PathBuf, FsError>;
    async fn ensure_directory(&self, path: &Path) -> Result<(), FsError>;
}
```
- **Ops:** resolve output path, create temp file, atomic rename, long-path (`\\?\`) handling
- **Errors:** FsFail (exit 5)
- **Windows Features:** Long-path support, atomic operations, Unicode filenames

### ConfigPort
```rust
pub trait ConfigPort {
    fn load_config(&self) -> Result<Config, ConfigError>;
    fn save_config(&self, config: &Config) -> Result<(), ConfigError>;
    fn validate_config(&self, config: &Config) -> Result<(), ConfigError>;
}
```
- **Configuration Management:** Load/save user preferences and system settings
- **Sources:** CLI args, environment variables, config file, defaults

### LogPort
```rust
pub trait LogPort {
    fn log_event(&self, level: LogLevel, message: &str, context: &LogContext);
    fn log_structured(&self, event: &StructuredEvent);
    fn set_level(&self, level: LogLevel);
}
```
- **Logging:** Structured logging with context and performance metrics
- **Formats:** Human-readable, JSON, structured events

## Adapters

### Primary (Driving)
- **CliAdapter** - Command-line interface parsing and validation
- **ConfigAdapter** - Configuration loading and validation

### Secondary (Driven)
- **LibavProbeAdapter** - Direct libav FFI for media probing
- **LibavPipelineAdapter** - In-process decode/encode pipelines
- **WindowsFsAdapter** - Windows filesystem operations (long-path, atomic writes)
- **TracingLogAdapter** - Structured logging with tracing crate
- **TomlConfigAdapter** - TOML configuration file handling

### Feature Flags
- `nvenc`: Hardware acceleration support (NVIDIA)
- `qsv`: Intel Quick Sync support
- `debug`: Debug logging and additional diagnostics

## Layering & Dependencies

```
adapters → ports → application → domain (one-way inward)
```

- Domain has no dependency on FFmpeg, Windows, or CLI crates
- Clean separation enables testing and backend swapping

## Domain Model

### Core Types

#### Time and Timing
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSpec {
    pub seconds: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Timebase {
    pub num: i32,
    pub den: i32,
}

impl Timebase {
    pub fn rescale_pts(&self, pts: i64, target: &Timebase) -> i64;
    pub fn to_seconds(&self, pts: i64) -> f64;
}
```

#### Stream Information
```rust
#[derive(Debug, Clone)]
pub enum StreamKind {
    Video,
    Audio,
    Subtitle,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub index: usize,
    pub kind: StreamKind,
    pub codec: String,
    pub timebase: Timebase,
    pub duration: Option<TimeSpec>,
    pub bitrate: Option<u32>,
    pub language: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct VideoStreamInfo {
    pub base: StreamInfo,
    pub width: u32,
    pub height: u32,
    pub frame_rate: Option<f64>,
    pub pixel_format: String,
    pub color_space: Option<String>,
    pub rotation: Option<f32>,
}
```

#### Media Information
```rust
#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub container: String,
    pub duration: TimeSpec,
    pub bitrate: Option<u32>,
    pub best_video: Option<VideoStreamInfo>,
    pub audios: Vec<StreamInfo>,
    pub subtitles: Vec<StreamInfo>,
    pub metadata: HashMap<String, String>,
}
```

#### Clipping Operations
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CutRange {
    pub start: TimeSpec,
    pub end: TimeSpec,
}

#[derive(Debug, Clone)]
pub enum ClippingMode {
    Copy,
    Hybrid,
    Reencode,
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub mode: ClippingMode,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub cut_range: CutRange,
    pub stream_map: StreamMap,
    pub container_format: String,
    pub codec_settings: CodecSettings,
}
```

#### Output and Results
```rust
#[derive(Debug, Clone)]
pub struct OutputReport {
    pub success: bool,
    pub first_pts: Option<i64>,
    pub last_pts: Option<i64>,
    pub duration_out: TimeSpec,
    pub file_size: u64,
    pub warnings: Vec<String>,
    pub processing_time: Duration,
}
```

#### Configuration
```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub log_level: LogLevel,
    pub output_format: String,
    pub codec_preset: String,
    pub crf: Option<u8>,
    pub hardware_acceleration: bool,
    pub overwrite_policy: OverwritePolicy,
}
```

## Use-Case Flows

### Clip Flow
1. CLI → validate → `CutRange`
2. `ProbePort.run()` → `MediaInfo`
3. Domain `PlanClip.decide()` → `Plan` (Copy/Hybrid/Reencode)
4. `FsPort.prepare()` → temp output
5. `ExecutePort.execute(Plan)` → `OutputReport`
6. VerifyOutput (re-probe; tolerance checks)
7. `FsPort.commit()` → final path
8. Return report → map to exit code

### Inspect Flow
1. CLI → `ProbePort.run()` → pretty/JSON out

### Verify Flow
1. CLI → `ProbePort.run()` on output → tolerance checks → exit code

## Observability

- `--log-level {error,warn,info,debug}`
- `--json` structured events: phase, cmd_hash, plan_mode, bytes, timings, warnings
- `--dry-run` prints the plan without executing

## Packaging Boundary

MSI (WiX/MSIX) is an outer adapter:
1. **Single Binary**: Self-contained executable with embedded libav libraries
2. **Dependencies**: No external dependencies required
3. **Installation**: Add TrimX install dir to PATH
4. **Signing**: Code sign .exe & .msi for Windows security
5. **Distribution**: winget package for easy installation

## Test Plan

### Unit Tests
- **Domain**: timebase math, keyframe rule, plan selection, range validation
- **App**: use mock Probe/Execute ports to test orchestrations and error mapping
- **Ports**: test interface contracts and error handling
- **Adapters**: test libav FFI integration and filesystem operations

### Integration Tests
- **Real libav**: Test with actual media files (MP4/MKV/MOV/TS/AVI; H.264/HEVC/VP9; AAC/AC-3/PCM)
- **End-to-End**: Complete workflows from CLI to output file
- **Performance**: Benchmark processing times and memory usage
- **Error Scenarios**: Corrupt files, invalid ranges, permission issues

### Test Infrastructure
- **Fixtures**: Sample media files in `tests/fixtures/`
- **Mocks**: Mock implementations for all ports
- **Helpers**: Test utilities for common operations
- **CI Integration**: Automated testing in GitHub Actions

### Acceptance Criteria
- `--mode auto` start within ±1 frame, end ≤ requested
- Copy mode throughput ≈ file copy speed
- MP4 outputs have faststart metadata
- Long-path and Unicode filenames work correctly
- Subtitle handling: text subs copied; bitmap subs warning + MKV fallback
- Memory usage bounded (< 500MB for typical files)
- Processing time < 2× real-time for copy mode

## Implementation Roadmap

### Phase 1: Core Infrastructure (Weeks 1-4)
**Goal**: Establish foundation with port interfaces and domain model

1. **Port Interfaces** (Week 1)
   - Define ProbePort, ExecutePort, FsPort, ConfigPort, LogPort traits
   - Implement async method signatures
   - Create comprehensive documentation for each port
   - **Success Criteria**: All port interfaces defined and documented

2. **Domain Model** (Week 2)
   - Implement core types (TimeSpec, Timebase, StreamInfo, MediaInfo)
   - Create business rules for mode selection and keyframe analysis
   - Implement timebase conversion utilities
   - **Success Criteria**: All domain types implemented with unit tests

3. **Error Handling** (Week 3)
   - Create error types and mapping system
   - Implement structured error output (JSON format)
   - Add error recovery hints and context
   - **Success Criteria**: Complete error taxonomy with exit codes

4. **Configuration & Logging** (Week 4)
   - Implement config loading and validation
   - Set up structured logging with tracing
   - Create configuration hierarchy (CLI → Env → File → Defaults)
   - **Success Criteria**: Configuration system working with validation

### Phase 2: libav Integration (Weeks 5-8)
**Goal**: Implement media processing capabilities with libav FFI

5. **LibavProbeAdapter** (Week 5)
   - Implement media probing with libav FFI
   - Add stream analysis and keyframe detection
   - Handle various container formats and codecs
   - **Success Criteria**: Can probe all supported media formats

6. **LibavPipelineAdapter** (Week 6)
   - Implement copy mode execution
   - Add packet-level processing and timestamp correction
   - Handle stream mapping and container conversion
   - **Success Criteria**: Copy mode working for keyframe-aligned cuts

7. **Hybrid & Re-encode Modes** (Week 7)
   - Implement hybrid mode (GOP-spanning method)
   - Add re-encode mode with precise cuts
   - Implement hardware acceleration support (NVENC/QSV)
   - **Success Criteria**: All three modes working with quality validation

8. **Resource Management** (Week 8)
   - Implement proper cleanup of libav contexts
   - Add memory management and resource pooling
   - Handle error recovery and resource cleanup
   - **Success Criteria**: No memory leaks, proper error recovery

### Phase 3: Application Layer (Weeks 9-12)
**Goal**: Implement use case orchestration and CLI interface

9. **Use Case Interactors** (Week 9)
   - Implement ClipInteractor with mode selection logic
   - Add InspectInteractor for media analysis
   - Create VerifyInteractor for output validation
   - **Success Criteria**: All use cases implemented and tested

10. **CLI Implementation** (Week 10)
    - Complete command parsing and validation
    - Add comprehensive help and error messages
    - Implement structured output options (JSON)
    - **Success Criteria**: CLI interface complete with all commands

11. **Filesystem Operations** (Week 11)
    - Implement Windows-specific path handling
    - Add atomic writes and temp file management
    - Handle long-path support and Unicode filenames
    - **Success Criteria**: Robust file operations on Windows

12. **Output Verification** (Week 12)
    - Implement tolerance checks and validation
    - Add duration and quality verification
    - Create comprehensive output reports
    - **Success Criteria**: Reliable output validation

### Phase 4: Testing & Quality (Weeks 13-16)
**Goal**: Comprehensive testing and quality assurance

13. **Unit Tests** (Week 13)
    - Comprehensive domain and application layer tests
    - Mock implementations for all ports
    - Edge case and error condition testing
    - **Success Criteria**: 90%+ test coverage for domain and app layers

14. **Integration Tests** (Week 14)
    - End-to-end testing with real media files
    - Test matrix covering all formats and codecs
    - Performance and memory usage testing
    - **Success Criteria**: All test scenarios passing

15. **Performance Benchmarks** (Week 15)
    - Establish performance baselines
    - Optimize critical paths
    - Memory usage profiling and optimization
    - **Success Criteria**: Performance targets met

16. **Quality Assurance** (Week 16)
    - Code review and refactoring
    - Documentation review and updates
    - Security audit and vulnerability assessment
    - **Success Criteria**: Production-ready code quality

### Phase 5: Distribution & Documentation (Weeks 17-20)
**Goal**: Package and distribute the application

17. **MSI Packaging** (Week 17)
    - Create Windows installer with embedded libav
    - Implement automatic dependency management
    - Add installation and uninstallation procedures
    - **Success Criteria**: Working MSI installer

18. **Code Signing** (Week 18)
    - Sign executables for Windows security
    - Implement signature verification
    - Set up automated signing process
    - **Success Criteria**: Signed binaries with valid certificates

19. **Documentation** (Week 19)
    - Complete API docs, user guides, troubleshooting
    - Create video tutorials and examples
    - Update all documentation for accuracy
    - **Success Criteria**: Comprehensive documentation suite

20. **CI/CD Pipeline** (Week 20)
    - Automated testing, building, and deployment
    - Release automation and version management
    - Monitoring and alerting setup
    - **Success Criteria**: Fully automated release pipeline

### Success Metrics
- **Functional**: All clipping modes working with <1% error rate
- **Performance**: Copy mode ≥1.2× file read speed, Re-encode mode <2× real-time
- **Quality**: 90%+ test coverage, no memory leaks, proper error handling
- **Usability**: Clear error messages, comprehensive help, intuitive CLI
- **Reliability**: Handles edge cases gracefully, robust error recovery

## Directory Structure

### Current Implementation Status
**Note**: The project uses hexagonal architecture from the start with clean layer separation and ports & adapters pattern.

### Implementation Structure
```
src/
├── cli/           # Command-line interface and argument parsing
│   ├── mod.rs
│   ├── args.rs    # CLI argument definitions
│   └── commands.rs # Command implementations
├── domain/        # Pure business logic
│   ├── model/     # Core types and data structures
│   ├── rules/     # Business rules and policies
│   ├── usecases/  # Use case orchestration
│   └── errors/    # Domain error types
├── app/           # Application layer
│   ├── clip_interactor/
│   ├── inspect_interactor/
│   └── verify_interactor/
├── ports/         # Interface definitions
│   ├── probe_port.rs
│   ├── execute_port.rs
│   ├── fs_port.rs
│   ├── config_port.rs
│   └── log_port.rs
├── adapters/      # External system implementations
│   ├── probe_libav.rs
│   ├── exec_libav.rs
│   ├── fs_windows.rs
│   ├── tracing_log.rs
│   └── toml_config.rs
└── utils/         # Common utilities
    ├── time.rs
    └── path.rs
```

This architecture ensures maintainability, testability, and flexibility while keeping the core domain pure and independent of external dependencies.
