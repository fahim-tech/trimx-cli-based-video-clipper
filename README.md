# TrimX CLI Video Clipper

A Windows-native command-line tool for precise video clipping with intelligent lossless stream-copy and fallback re-encoding capabilities.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-blue.svg)](https://www.microsoft.com/en-us/windows)

## 🎯 Overview

TrimX is designed for editors, content creators, QA teams, and researchers who need precise video clipping without the complexity of full video editing suites. It intelligently chooses between lossless stream-copy and precise re-encoding to deliver accurate cuts while maintaining optimal performance.

### Key Features

- **Smart Clipping Strategy**: Automatically detects optimal clipping method (lossless copy vs re-encode)
- **GOP-Spanning Method**: Re-encodes only necessary leading/trailing GOPs, stream-copies middle segments
- **Stream Preservation**: Maintains video, audio, subtitles, and metadata by default
- **Deterministic Output**: Same inputs produce identical output bytes
- **Multiple Time Formats**: Supports HH:MM:SS.ms, MM:SS.ms, and seconds as float
- **Windows Optimized**: Native Windows 10/11 x64 support with long-path handling

## 🚀 Quick Start

### Installation

Download the latest release from [GitHub Releases](https://github.com/yourusername/trimx-cli-based-video-clipper/releases) or install via winget:

```bash
winget install TrimX.Clipper
```

### Basic Usage

```bash
# Clip a segment from a video file
clipper.exe clip --in "D:\media\lecture.mov" --start 00:10:05.250 --end 00:12:40.000

# Inspect video file information
clipper.exe inspect --in "D:\media\lecture.mov"

# Verify a clipped segment
clipper.exe verify --in "D:\out\lecture_part.mov" --start 00:10:05.250 --end 00:12:40.000
```

## 📖 Detailed Usage

### Commands

#### `clip` - Extract video segments

```bash
clipper.exe clip --in <input_file> --start <time> --end <time> [options]
```

**Required Arguments:**
- `--in <path>`: Input video file path
- `--start <time>`: Start time (HH:MM:SS.ms, MM:SS.ms, or seconds)
- `--end <time>`: End time (HH:MM:SS.ms, MM:SS.ms, or seconds)

**Optional Arguments:**
- `--out <path>`: Output file path (default: auto-generated)
- `--mode <strategy>`: Clipping strategy (`auto`, `copy`, `reencode`)
- `--no-audio`: Remove audio streams
- `--no-subs`: Remove subtitle streams
- `--container <format>`: Output container (`same`, `mp4`, `mkv`)
- `--codec <codec>`: Video codec (`h264`, `hevc`)
- `--crf <quality>`: Constant Rate Factor (0-51, default: 18)
- `--preset <speed>`: Encoding preset (`ultrafast`, `fast`, `medium`, `slow`)

#### `inspect` - Analyze video files

```bash
clipper.exe inspect --in <input_file> [--json]
```

Displays detailed information about the video file including:
- Duration and timebase
- Codec information
- Stream details
- Keyframe intervals
- Rotation metadata

#### `verify` - Validate clipped segments

```bash
clipper.exe verify --in <clipped_file> --start <time> --end <time>
```

Verifies that a clipped segment matches the expected timing and content.

### Global Options

- `--log-level <level>`: Logging level (`error`, `warn`, `info`, `debug`)
- `--overwrite <policy>`: Overwrite behavior (`prompt`, `always`, `never`)

## 🏗️ Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Parser    │───▶│   Probe Service │───▶│   Cut Planner   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Configuration │    │   Stream Info    │    │  Strategy Logic  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Clip Engine   │◀───│   Stream Handler│◀───│   Output Writer │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Clipping Strategies

#### 1. Auto Mode (Default)
Intelligently selects the best strategy based on:
- Keyframe alignment
- Container capabilities
- Codec compatibility
- Stream requirements

#### 2. Copy Mode (`--mode copy`)
Fast lossless stream-copy with approximate cuts:
- ⚡ **Pros**: Extremely fast, no quality loss
- ⚠️ **Cons**: May include frames from previous GOP

#### 3. Re-encode Mode (`--mode reencode`)
Precise frame-accurate cuts with full re-encoding:
- ✅ **Pros**: Exact cuts, perfect timing
- 🐌 **Cons**: Slower, potential quality loss

### GOP-Spanning Method

For non-keyframe starts, TrimX uses a "sandwich" approach:

```
Original Video: [GOP1] [GOP2] [GOP3] [GOP4] [GOP5]
                    │     │     │     │     │
                    ▼     ▼     ▼     ▼     ▼
Clipped Video:   [Re-encode] [Copy] [Copy] [Re-encode]
```

1. **Leading GOP**: Re-encode from start to next keyframe
2. **Middle GOPs**: Stream-copy packets with timestamp correction
3. **Trailing GOP**: Re-encode from last keyframe to end

## 🔧 Technical Details

### Supported Formats

**Containers:**
- MP4 (isom/iso6)
- MKV
- MOV
- TS
- AVI

**Video Codecs:**
- H.264/AVC
- H.265/HEVC
- VP9
- MPEG-2

**Audio Codecs:**
- AAC
- AC-3
- PCM

### Performance Characteristics

- **Copy Mode**: ~1.2× file read speed
- **Re-encode Mode**: Depends on codec and preset
- **Memory Usage**: Bounded for large GOPs
- **Hardware Acceleration**: Optional NVENC/QSV support

### Error Handling

TrimX provides comprehensive error handling with:
- **Categorized Errors**: Input validation, codec support, I/O issues
- **Recovery Hints**: Actionable suggestions for common problems
- **Structured Output**: JSON format for automation
- **Exit Codes**: Standardized return values

## 🧪 Testing

### Test Matrix

The project includes comprehensive tests covering:

- **Containers**: All supported formats
- **Codecs**: Various video/audio combinations
- **Scenarios**: Keyframe/non-keyframe cuts, VFR, rotated videos
- **Edge Cases**: Long paths, corrupt files, boundary conditions

### Running Tests

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration

# Run with verbose output
cargo test -- --nocapture
```

## 📦 Building from Source

### Prerequisites

- Rust 1.70+
- FFmpeg development libraries
- Windows SDK (for Windows-specific features)

### Build Instructions

```bash
# Clone the repository
git clone https://github.com/yourusername/trimx-cli-based-video-clipper.git
cd trimx-cli-based-video-clipper

# Build in release mode
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch cargo-expand

# Run in watch mode during development
cargo watch -x run -- clip --in test.mov --start 00:01:00 --end 00:02:00
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

### Code Style

- Follow Rust naming conventions
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Write comprehensive documentation
- Include tests for new features

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🐛 Bug Reports & Feature Requests

Please use [GitHub Issues](https://github.com/yourusername/trimx-cli-based-video-clipper/issues) to report bugs or request features.

### Bug Report Template

```markdown
**Describe the bug**
A clear description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Run command '...'
2. See error

**Expected behavior**
What you expected to happen.

**Environment**
- OS: Windows 10/11
- TrimX version: [e.g., 1.0.0]
- Input file format: [e.g., MP4, H.264]

**Additional context**
Add any other context about the problem here.
```

## 📚 Documentation

- [API Documentation](https://docs.rs/trimx-cli)
- [User Guide](docs/user-guide.md)
- [Developer Guide](docs/developer-guide.md)
- [Troubleshooting](docs/troubleshooting.md)

## 🙏 Acknowledgments

- FFmpeg project for media processing capabilities
- Rust community for excellent tooling and ecosystem
- Contributors and users for feedback and improvements

---

**TrimX CLI Video Clipper** - Precise video clipping made simple.
