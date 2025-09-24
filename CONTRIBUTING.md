# Contributing to TrimX CLI Video Clipper

Thank you for your interest in contributing to TrimX! This document provides guidelines and information for contributors.

## 🚀 Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- Windows 10/11 (for testing)
- FFmpeg development libraries (for building)

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/yourusername/trimx-cli-based-video-clipper.git
   cd trimx-cli-based-video-clipper
   ```

2. **Install Dependencies**
   ```bash
   # Install Rust toolchain
   rustup install stable
   
   # Install development tools
   cargo install cargo-watch cargo-expand cargo-audit
   ```

3. **Build and Test**
   ```bash
   cargo build
   cargo test
   ```

## 📋 Development Workflow

### Branch Strategy

- `main`: Stable, production-ready code
- `develop`: Integration branch for features
- `feature/*`: Feature development branches
- `bugfix/*`: Bug fix branches
- `hotfix/*`: Critical bug fixes

### Commit Message Format

We use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Test additions/modifications
- `ci`: CI/CD changes
- `build`: Build system changes
- `chore`: Maintenance tasks

**Examples:**
```
feat(cli): add --mode auto option for intelligent clipping strategy
fix(engine): resolve timestamp overflow in GOP-spanning method
docs(readme): update installation instructions for Windows
test(probe): add test cases for VFR video files
```

### Pull Request Process

1. **Create Feature Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make Changes**
   - Write clean, well-documented code
   - Add tests for new functionality
   - Update documentation as needed
   - Follow Rust naming conventions

3. **Test Your Changes**
   ```bash
   cargo test
   cargo clippy
   cargo fmt
   ```

4. **Commit Changes**
   ```bash
   git add .
   git commit -m "feat(module): description of changes"
   ```

5. **Push and Create PR**
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Submit Pull Request**
   - Use the PR template
   - Provide clear description
   - Link related issues
   - Request reviews from maintainers

## 🏗️ Code Architecture

### Project Structure

```
src/
├── cli/           # Command-line interface
│   ├── mod.rs
│   ├── args.rs    # Argument parsing
│   └── commands.rs # Command implementations
├── probe/         # Media file inspection
│   ├── mod.rs
│   ├── inspector.rs
│   └── validator.rs
├── planner/        # Cut strategy planning
│   ├── mod.rs
│   ├── strategy.rs
│   └── gop.rs
├── engine/         # Core clipping engine
│   ├── mod.rs
│   ├── copy.rs
│   ├── reencode.rs
│   └── hybrid.rs
├── streams/        # Stream handling
│   ├── mod.rs
│   ├── mapper.rs
│   └── processor.rs
├── output/         # Output file writing
│   ├── mod.rs
│   ├── writer.rs
│   └── verifier.rs
├── error/          # Error handling
│   ├── mod.rs
│   └── types.rs
├── utils/          # Common utilities
│   ├── mod.rs
│   ├── time.rs
│   └── path.rs
└── lib.rs
```

### Module Guidelines

- **Single Responsibility**: Each module should have a clear, single purpose
- **Loose Coupling**: Minimize dependencies between modules
- **High Cohesion**: Related functionality should be grouped together
- **Clear Interfaces**: Public APIs should be well-documented

## 🧪 Testing Guidelines

### Test Categories

1. **Unit Tests**: Test individual functions and methods
2. **Integration Tests**: Test module interactions
3. **End-to-End Tests**: Test complete workflows
4. **Performance Tests**: Benchmark critical paths

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_output);
    }
}
```

### Test Data

- Use small, focused test files
- Include edge cases and error conditions
- Test with various formats and codecs
- Mock external dependencies when appropriate

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test probe::tests

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration

# Run benchmarks
cargo bench
```

## 📝 Code Style Guidelines

### Rust Conventions

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Prefer `snake_case` for functions and variables
- Use `PascalCase` for types and traits

### Documentation

- Document all public APIs with `///` comments
- Include examples in documentation
- Use `# Examples` sections for complex functions
- Document error conditions and return values

### Error Handling

- Use `anyhow` for application-level errors
- Create custom error types for domain-specific errors
- Implement `Display` and `Error` traits
- Provide context and recovery hints

### Performance

- Use `cargo bench` to measure performance
- Profile with `cargo flamegraph`
- Consider memory usage and allocation patterns
- Use zero-copy operations where possible

## 🔍 Code Review Process

### Review Checklist

**Functionality:**
- [ ] Code works as intended
- [ ] Edge cases are handled
- [ ] Error conditions are properly managed
- [ ] Performance is acceptable

**Code Quality:**
- [ ] Code is readable and well-structured
- [ ] Follows Rust conventions
- [ ] No unnecessary complexity
- [ ] Proper error handling

**Testing:**
- [ ] Tests cover new functionality
- [ ] Tests pass consistently
- [ ] Edge cases are tested
- [ ] Performance tests updated if needed

**Documentation:**
- [ ] Public APIs are documented
- [ ] Examples are provided
- [ ] README updated if needed
- [ ] Changelog updated

### Review Guidelines

**For Reviewers:**
- Be constructive and respectful
- Focus on code, not the person
- Provide specific feedback
- Suggest improvements, don't just criticize

**For Authors:**
- Respond to feedback promptly
- Ask questions if feedback is unclear
- Make requested changes
- Explain design decisions when needed

## 🐛 Bug Reports

### Before Reporting

1. Check existing issues
2. Verify the bug with latest version
3. Try to reproduce the issue
4. Gather relevant information

### Bug Report Template

```markdown
**Bug Description**
Clear description of the bug.

**Steps to Reproduce**
1. Run command: `clipper.exe ...`
2. Observe behavior: ...
3. Error occurs: ...

**Expected Behavior**
What should happen instead.

**Environment**
- OS: Windows 10/11
- TrimX version: 1.0.0
- Rust version: 1.70.0
- Input file: [format, codec, size]

**Additional Context**
- Error messages
- Log files
- Screenshots
- Related issues
```

## 💡 Feature Requests

### Before Requesting

1. Check existing feature requests
2. Consider if it fits project scope
3. Think about implementation complexity
4. Consider alternative approaches

### Feature Request Template

```markdown
**Feature Description**
Clear description of the requested feature.

**Use Case**
Why is this feature needed? What problem does it solve?

**Proposed Solution**
How should this feature work?

**Alternatives Considered**
What other approaches were considered?

**Additional Context**
- Related issues
- Examples from other tools
- Implementation ideas
```

## 📚 Documentation

### Documentation Types

1. **API Documentation**: Generated from code comments
2. **User Documentation**: README, user guides
3. **Developer Documentation**: Architecture, design decisions
4. **Troubleshooting**: Common issues and solutions

### Writing Guidelines

- Use clear, concise language
- Provide examples and code snippets
- Keep documentation up-to-date
- Use consistent formatting

## 🏷️ Release Process

### Version Numbering

We use [Semantic Versioning](https://semver.org/):
- `MAJOR`: Breaking changes
- `MINOR`: New features (backward compatible)
- `PATCH`: Bug fixes (backward compatible)

### Release Checklist

- [ ] All tests pass
- [ ] Documentation updated
- [ ] Changelog updated
- [ ] Version bumped
- [ ] Release notes prepared
- [ ] Binaries built and signed
- [ ] GitHub release created

## 🤝 Community Guidelines

### Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Respect different perspectives

### Getting Help

- Check documentation first
- Search existing issues
- Ask questions in discussions
- Join community channels

## 📞 Contact

- **Issues**: [GitHub Issues](https://github.com/yourusername/trimx-cli-based-video-clipper/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/trimx-cli-based-video-clipper/discussions)
- **Email**: [maintainer@example.com](mailto:maintainer@example.com)

---

Thank you for contributing to TrimX! Your efforts help make video clipping more accessible and reliable for everyone.
