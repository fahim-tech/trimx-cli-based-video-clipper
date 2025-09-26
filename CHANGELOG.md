# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Simplified CLI syntax with positional arguments
- Auto-generated output filenames with descriptive format
- Improved duration detection in libav probe adapter
- Support for AV_NOPTS_VALUE handling in FFmpeg integration

### Changed
- CLI now uses `clipper clip "video.mp4" --start "0:15" --end "0:30"` instead of requiring `--input` flag
- Output filenames auto-generated as `{original}_clip_{start}_to_{end}.{ext}`
- Enhanced probe adapter to handle missing container duration

### Deprecated

### Removed

### Fixed
- Duration detection issues when container duration is not available
- Compilation errors with ffmpeg-next API mismatches
- Stream duration extraction for video and audio streams

### Security

## [0.1.0] - 2024-01-XX

### Added
- Initial release planning
- Project architecture design
- Core module structure definition
- FFmpeg integration strategy
- Windows-specific optimizations

### Changed

### Deprecated

### Removed

### Fixed

### Security

---

## Release Notes Template

### [X.Y.Z] - YYYY-MM-DD

#### Added
- New features and functionality

#### Changed
- Changes to existing functionality

#### Deprecated
- Features that will be removed in future versions

#### Removed
- Features removed in this version

#### Fixed
- Bug fixes

#### Security
- Security improvements and fixes
