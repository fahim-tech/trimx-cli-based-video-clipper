# TrimX CLI Video Clipper

## Development Environment Setup

### Prerequisites
- Rust 1.70+
- FFmpeg development libraries
- Windows SDK (for Windows-specific features)

### Quick Start
```bash
# Clone the repository
git clone https://github.com/yourusername/trimx-cli-based-video-clipper.git
cd trimx-cli-based-video-clipper

# Build the project
cargo build

# Run tests
cargo test

# Run the CLI
cargo run -- clip --in test.mov --start 00:01:00 --end 00:02:00
```

### Development Commands
```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Run tests with output
cargo test -- --nocapture

# Build release
cargo build --release

# Install locally
cargo install --path .
```

### Project Structure
```
src/
├── cli/           # Command-line interface
├── error/         # Error handling
├── probe/         # Media file inspection
├── planner/       # Cut strategy planning
├── engine/        # Core clipping engine
├── streams/       # Stream handling
├── output/        # Output file writing
└── utils/         # Common utilities
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test module
cargo test probe::tests

# Run integration tests
cargo test --test integration

# Run benchmarks
cargo bench
```

### Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

### License
MIT License - see [LICENSE](LICENSE) for details.
