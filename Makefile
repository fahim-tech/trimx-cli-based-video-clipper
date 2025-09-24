# Makefile for TrimX CLI Video Clipper
# Common development tasks and quality gates

.PHONY: help install build test lint format clean coverage benchmark docs release

# Default target
help: ## Show this help message
	@echo "TrimX CLI Video Clipper - Development Commands"
	@echo "=============================================="
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Installation and setup
install: ## Install dependencies and setup development environment
	@echo "Installing Rust toolchain..."
	@curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	@echo "Installing development tools..."
	@cargo install cargo-watch cargo-expand cargo-tarpaulin cargo-audit cargo-deny
	@echo "Setting up pre-commit hooks..."
	@pip install pre-commit
	@pre-commit install

# Building
build: ## Build the project in debug mode
	@echo "Building TrimX in debug mode..."
	@cargo build

build-release: ## Build the project in release mode
	@echo "Building TrimX in release mode..."
	@cargo build --release

build-all: ## Build with all features
	@echo "Building TrimX with all features..."
	@cargo build --all-features

# Testing
test: ## Run all tests
	@echo "Running unit and integration tests..."
	@cargo test --all-features

test-unit: ## Run unit tests only
	@echo "Running unit tests..."
	@cargo test --lib

test-integration: ## Run integration tests only
	@echo "Running integration tests..."
	@cargo test --test integration_tests

test-verbose: ## Run tests with verbose output
	@echo "Running tests with verbose output..."
	@cargo test --all-features -- --nocapture

# Code quality
lint: ## Run clippy linter
	@echo "Running clippy linter..."
	@cargo clippy --all-targets --all-features -- -D warnings

format: ## Format code with rustfmt
	@echo "Formatting code..."
	@cargo fmt --all

format-check: ## Check code formatting
	@echo "Checking code formatting..."
	@cargo fmt --all -- --check

# Security and auditing
audit: ## Run security audit
	@echo "Running security audit..."
	@cargo audit

deny: ## Run license and security checks
	@echo "Running license and security checks..."
	@cargo deny check

security: audit deny ## Run all security checks

# Coverage
coverage: ## Generate code coverage report
	@echo "Generating code coverage report..."
	@cargo tarpaulin --out Html --output-dir coverage
	@echo "Coverage report generated in coverage/ directory"

coverage-open: coverage ## Generate and open coverage report
	@echo "Opening coverage report..."
	@open coverage/tarpaulin-report.html || xdg-open coverage/tarpaulin-report.html

# Performance
benchmark: ## Run performance benchmarks
	@echo "Running performance benchmarks..."
	@cargo bench

benchmark-compare: ## Compare benchmark results
	@echo "Comparing benchmark results..."
	@cargo bench -- --save-baseline current
	@cargo bench -- --baseline current

# Documentation
docs: ## Generate documentation
	@echo "Generating documentation..."
	@cargo doc --no-deps --document-private-items

docs-open: docs ## Generate and open documentation
	@echo "Opening documentation..."
	@open target/doc/trimx_cli_based_video_clipper/index.html || xdg-open target/doc/trimx_cli_based_video_clipper/index.html

docs-serve: docs ## Generate documentation and serve locally
	@echo "Serving documentation on http://localhost:3000"
	@cd target/doc && python3 -m http.server 3000

# Development workflow
watch: ## Watch for changes and run tests
	@echo "Watching for changes..."
	@cargo watch -x test

watch-build: ## Watch for changes and build
	@echo "Watching for changes and building..."
	@cargo watch -x build

dev: ## Run in development mode with sample video
	@echo "Running TrimX in development mode..."
	@cargo run -- --help

# Quality gates (CI/CD pipeline simulation)
quality-gates: format-check lint test security coverage ## Run all quality gates

# Release
release: quality-gates build-release ## Prepare release build
	@echo "Release build prepared successfully!"
	@echo "Binary location: target/release/trimx"

release-check: ## Check if ready for release
	@echo "Checking release readiness..."
	@cargo check --release
	@cargo test --release
	@cargo clippy --release -- -D warnings
	@echo "Release check completed successfully!"

# Cleanup
clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	@cargo clean

clean-all: clean ## Clean all artifacts including dependencies
	@echo "Cleaning all artifacts..."
	@cargo clean
	@rm -rf target/
	@rm -rf coverage/
	@rm -rf .cargo/

# Git workflow
commit: ## Commit with conventional commit format
	@echo "Committing changes..."
	@git add -A
	@git commit

push: ## Push to remote repository
	@echo "Pushing to remote repository..."
	@git push

# Docker (for CI/CD simulation)
docker-build: ## Build Docker image for testing
	@echo "Building Docker image..."
	@docker build -t trimx-cli .

docker-test: docker-build ## Run tests in Docker container
	@echo "Running tests in Docker..."
	@docker run --rm trimx-cli cargo test

# Project management
status: ## Show project status
	@echo "TrimX CLI Video Clipper - Project Status"
	@echo "========================================"
	@echo "Git status:"
	@git status --short
	@echo ""
	@echo "Recent commits:"
	@git log --oneline -5
	@echo ""
	@echo "Test status:"
	@cargo test --quiet 2>/dev/null && echo "✅ All tests passing" || echo "❌ Some tests failing"
	@echo ""
	@echo "Lint status:"
	@cargo clippy --quiet 2>/dev/null && echo "✅ No linting issues" || echo "❌ Linting issues found"

# Help for specific commands
help-test: ## Show test-related help
	@echo "Test Commands:"
	@echo "  test           - Run all tests"
	@echo "  test-unit      - Run unit tests only"
	@echo "  test-integration - Run integration tests only"
	@echo "  test-verbose   - Run tests with verbose output"
	@echo "  coverage       - Generate code coverage report"

help-quality: ## Show quality-related help
	@echo "Quality Commands:"
	@echo "  lint           - Run clippy linter"
	@echo "  format         - Format code with rustfmt"
	@echo "  format-check   - Check code formatting"
	@echo "  audit          - Run security audit"
	@echo "  deny           - Run license and security checks"
	@echo "  security       - Run all security checks"
	@echo "  quality-gates  - Run all quality gates"
