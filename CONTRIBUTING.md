# Contributing to Wirerust

Thank you for your interest in contributing to Wirerust! This document provides guidelines and information for contributors.

## Development Setup

1. **Clone the repository:**
   ```bash
   git clone https://github.com/pyrowall/wirerust.git
   cd wirerust
   ```

2. **Install Rust:**
   Make sure you have Rust installed. Visit [rustup.rs](https://rustup.rs/) for installation instructions.

3. **Run tests:**
   ```bash
   cargo test
   ```

4. **Run clippy:**
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```

5. **Check formatting:**
   ```bash
   cargo fmt --all -- --check
   ```

## Code Style

- Follow Rust style guidelines and idioms
- Use `cargo fmt` to format your code
- Run `cargo clippy` to check for common issues
- Write comprehensive tests for new features
- Add documentation for public APIs

## Testing

- Write unit tests for all new functionality
- Ensure integration tests cover end-to-end scenarios
- Use property-based testing with proptest for complex logic
- Run tests with all features enabled: `cargo test --all-features`

## Pull Request Process

1. **Fork the repository** and create a feature branch
2. **Make your changes** following the code style guidelines
3. **Add tests** for new functionality
4. **Update documentation** if needed
5. **Run the full test suite** to ensure everything works
6. **Submit a pull request** with a clear description of your changes

## Commit Messages

Use conventional commit messages:
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `test:` for adding or updating tests
- `refactor:` for code refactoring
- `chore:` for maintenance tasks

Example:
```
feat: add support for IP address literals in parser

- Add IP address parsing to FilterParser
- Update tests to cover IP literal parsing
- Add documentation for IP address support
```

## Release Process

### For Maintainers

1. **Bump version:**
   ```bash
   ./scripts/bump-version.sh [patch|minor|major]
   ```

2. **The script will:**
   - Update version in Cargo.toml and Cargo.lock
   - Commit the version change
   - Create a git tag
   - Push changes and tag to trigger GitHub Actions

3. **GitHub Actions will:**
   - Run all tests
   - Publish to crates.io (if tag is pushed)
   - Create a GitHub release

### Version Bumping

- **Patch** (0.1.0 → 0.1.1): Bug fixes and minor improvements
- **Minor** (0.1.0 → 0.2.0): New features, backward compatible
- **Major** (0.1.0 → 1.0.0): Breaking changes

## Architecture Overview

Wirerust is organized into several modules:

- **`schema`**: Field and type definitions
- **`types`**: Literal values and type system
- **`expr`**: Abstract syntax tree and parser
- **`compiler`**: Compilation to intermediate representation
- **`context`**: Runtime value storage
- **`functions`**: Built-in and user-defined functions
- **`ir`**: Intermediate representation for execution
- **`filter`**: High-level filter API

## Adding New Features

1. **Plan the feature** and discuss it in an issue first
2. **Implement the core functionality** in the appropriate module
3. **Add tests** to ensure correctness
4. **Update documentation** and examples
5. **Consider performance implications** and add benchmarks if needed

## Reporting Bugs

When reporting bugs, please include:

- A clear description of the problem
- Steps to reproduce the issue
- Expected vs actual behavior
- Rust version and platform information
- Any relevant code snippets

## Getting Help

- Open an issue for bug reports or feature requests
- Join our discussions for general questions
- Check existing issues and pull requests

## License

By contributing to Wirerust, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

Thank you for contributing to Wirerust! 🦀 