# ADR-002: Execution Backend Strategy — Direct libav FFI Integration

**Status:** Accepted (Target Architecture)  
**Implementation Status:** Single-Phase Implementation  
**Target Implementation:** Weeks 1-16  
**Current Implementation:** Hexagonal architecture from start  
**Date:** 2024-01-01  
**Context:** Direct integration with libav libraries for precise control and optimal performance.

## Decision

### Direct libav FFI Integration
- Use in-process libav libraries (libavformat/libavcodec/libavfilter/libswresample) for all media processing
- Eliminate external ffmpeg.exe/ffprobe.exe processes
- Built-in support for hardware acceleration (NVENC/QSV) through libav
- Single binary with no external dependencies

## Implementation Status

### Single Phase Implementation (Weeks 1-16)
- **Weeks 1-4**: Basic libav FFI integration and port interfaces
- **Weeks 5-8**: Copy mode implementation using hexagonal architecture
- **Weeks 9-12**: Re-encode and hybrid modes with hardware acceleration
- **Weeks 13-16**: Performance optimization and production readiness

## Consequences

### Positive
- Precise timestamp control and fewer temp files
- Single binary with no external dependencies
- Direct access to all libav features
- Better performance with in-process operations
- Simplified deployment and distribution

### Negative
- Increased build complexity and binary size
- Requires libav development libraries
- More complex FFI integration
- Platform-specific build requirements

## Implementation Strategy

### Core Features
- Direct FFI bindings to libav libraries
- In-memory packet processing
- Hardware acceleration support (NVENC/QSV)
- Reduced temporary file usage
- Precise timestamp handling

### Build Configuration
- Use `ffmpeg-next` crate for FFI bindings
- Feature flags for hardware acceleration (`nvenc`, `qsv`)
- Platform-specific build scripts for libav linking
- Static linking for single binary distribution

## Related ADRs
- ADR-001: Architecture Style
- ADR-003: Core Domain Policies
