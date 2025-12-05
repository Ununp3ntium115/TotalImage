# Contributing to TotalImage

Thank you for your interest in contributing to TotalImage! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Documentation](#documentation)

---

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Prioritize security and correctness

---

## Getting Started

### Prerequisites

- **Rust:** 1.75 or later (`rustup update`)
- **Git:** For version control
- **Optional:** Docker for containerized builds

### Fork and Clone

```bash
# Fork the repository on GitHub
# Then clone your fork
git checkout https://github.com/YOUR_USERNAME/TotalImage.git
cd TotalImage

# Add upstream remote
git remote add upstream https://github.com/Ununp3ntium115/TotalImage.git
```

---

## Development Setup

### Build the Project

```bash
# Build all crates
cargo build --workspace

# Build in release mode
cargo build --workspace --release

# Build specific crate
cargo build -p totalimage-cli
```

### Run Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p totalimage-vaults

# Run with output
cargo test --workspace -- --nocapture

# Run ignored tests (requires fixtures)
cargo test --workspace -- --include-ignored
```

### Check Code Quality

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Run clippy (linter)
cargo clippy --workspace --all-targets

# Strict mode (no warnings)
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Project Structure

```
TotalImage/
├── crates/
│   ├── totalimage-core/       # Core types and traits
│   ├── totalimage-pipeline/   # I/O pipeline abstractions
│   ├── totalimage-vaults/     # Container format parsers (VHD, E01, AFF4)
│   ├── totalimage-zones/      # Partition table parsers (MBR, GPT)
│   ├── totalimage-territories/# Filesystem parsers (FAT, NTFS, exFAT, ISO)
│   ├── totalimage-cli/        # Command-line interface
│   ├── totalimage-web/        # REST API server
│   ├── totalimage-mcp/        # Model Context Protocol server
│   ├── totalimage-acquire/    # Disk image creation
│   └── fire-marshal/          # Tool orchestration
├── docs/                      # Documentation
├── k8s/                       # Kubernetes manifests
├── steering/                  # Project planning and analysis
└── tests/                     # Integration tests
```

### Crate Responsibilities

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| **totalimage-core** | Shared types, traits, security | None (base crate) |
| **totalimage-pipeline** | I/O abstractions, streaming | core |
| **totalimage-vaults** | Container format parsing | core, pipeline |
| **totalimage-zones** | Partition table parsing | core |
| **totalimage-territories** | Filesystem parsing | core |
| **totalimage-cli** | User-facing CLI tool | All domain crates |
| **totalimage-web** | REST API server | All domain crates |
| **totalimage-mcp** | MCP server for AI integration | All domain crates |

---

## Coding Standards

### Rust Style

- **Follow Rust API Guidelines:** https://rust-lang.github.io/api-guidelines/
- **Use `cargo fmt`:** Format all code before committing
- **Fix clippy warnings:** `cargo clippy` should pass with zero warnings
- **Prefer explicit types:** Avoid `impl Trait` in public APIs where clarity helps

### Naming Conventions

```rust
// Types: PascalCase
struct VhdFooter { }
enum Aff4Compression { }

// Functions: snake_case
fn parse_sector() -> Result<Sector> { }

// Constants: SCREAMING_SNAKE_CASE
const MAX_SECTOR_SIZE: usize = 4096;

// Modules: snake_case
mod partition_table;
```

### Error Handling

```rust
// ✅ Good: Use Result for recoverable errors
pub fn open_vault(path: &Path) -> Result<Vault> {
    let file = File::open(path)?;
    Ok(Vault { file })
}

// ❌ Bad: Don't use unwrap() in library code
pub fn get_sector(offset: u64) -> Sector {
    self.file.seek(offset).unwrap(); // Don't do this!
    // ...
}

// ✅ Good: Use expect() with context in examples/tests only
#[test]
fn test_sector_read() {
    let vault = Vault::open("test.vhd")
        .expect("Test fixture should exist");
}
```

### Documentation

```rust
/// Parse a VHD footer from raw bytes.
///
/// # Arguments
///
/// * `data` - 512-byte footer sector
///
/// # Returns
///
/// Returns `Ok(VhdFooter)` if valid, or `Err` if:
/// - Data is not exactly 512 bytes
/// - Cookie/magic bytes are invalid
/// - Checksum validation fails
///
/// # Example
///
/// ```
/// use totalimage_vaults::vhd::parse_footer;
///
/// let data = [0u8; 512];
/// match parse_footer(&data) {
///     Ok(footer) => println!("Disk size: {}", footer.current_size),
///     Err(e) => eprintln!("Invalid footer: {}", e),
/// }
/// ```
pub fn parse_footer(data: &[u8]) -> Result<VhdFooter> {
    // ...
}
```

### Security Best Practices

1. **Validate all inputs:**
   ```rust
   if bytes.len() < REQUIRED_SIZE {
       return Err(Error::invalid("Buffer too small"));
   }
   ```

2. **Use checked arithmetic:**
   ```rust
   let total_size = cluster_size.checked_mul(cluster_count)
       .ok_or_else(|| Error::overflow("Size calculation"))?;
   ```

3. **Sanitize paths:**
   ```rust
   validate_file_path(path, &allowed_roots)?;
   ```

4. **Limit allocations:**
   ```rust
   if size > MAX_ALLOCATION {
       return Err(Error::too_large("Allocation too large"));
   }
   ```

---

## Testing

### Unit Tests

Place tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sector_alignment() {
        assert_eq!(align_to_sector(100, 512), 512);
        assert_eq!(align_to_sector(512, 512), 512);
        assert_eq!(align_to_sector(513, 512), 1024);
    }

    #[test]
    fn test_invalid_sector_size() {
        let result = Vault::new(0); // Invalid size
        assert!(result.is_err());
    }
}
```

### Integration Tests

Add to `tests/` directory:

```rust
// tests/vhd_integration.rs
use totalimage_vaults::open_vault;

#[test]
#[ignore] // Requires test fixtures
fn test_vhd_fat32_pipeline() {
    let vault = open_vault("tests/fixtures/test.vhd", Default::default())
        .expect("Test fixture should exist");

    // Test full pipeline...
}
```

### Test Coverage Goals

- **Core parsing:** 90%+ coverage
- **Error paths:** Test error conditions
- **Edge cases:** Boundary conditions, empty inputs, maximum values
- **Security:** Malformed inputs, overflow attempts

---

## Pull Request Process

### 1. Create a Branch

```bash
git checkout -b feature/add-ext4-support
# or
git checkout -b fix/vhd-checksum-bug
```

Branch naming:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation only
- `refactor/` - Code refactoring
- `test/` - Test additions

### 2. Make Changes

- Write tests for new functionality
- Update documentation
- Follow coding standards
- Keep commits focused and atomic

### 3. Commit Messages

```
Add ext4 filesystem support

- Implement ext4 superblock parsing
- Add inode table reading
- Support extent trees for large files
- Add comprehensive tests for ext4 features

Closes #123
```

Format:
- First line: Imperative summary (<72 chars)
- Blank line
- Detailed description with bullet points
- Reference issues: `Closes #123`, `Fixes #456`

### 4. Update and Rebase

```bash
git fetch upstream
git rebase upstream/master
```

### 5. Run Quality Checks

```bash
# Format code
cargo fmt --all

# Run tests
cargo test --workspace

# Run clippy
cargo clippy --workspace --all-targets -- -D warnings

# Check docs build
cargo doc --no-deps --workspace
```

### 6. Push and Create PR

```bash
git push origin feature/add-ext4-support
```

Then create a Pull Request on GitHub with:
- Clear title describing the change
- Description of what changed and why
- Link to related issues
- Screenshots/examples if applicable

### 7. Code Review

- Address review feedback promptly
- Push additional commits (don't force-push during review)
- Discuss technical decisions constructively

---

## Documentation

### Public API Documentation

All public items must have doc comments:

```rust
/// Types of VHD disk images.
///
/// VHD supports three structural variations, each optimized
/// for different use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VhdType {
    /// Fixed-size allocation with all sectors pre-allocated.
    Fixed,
    /// Dynamic allocation that grows as data is written.
    Dynamic,
    /// Differencing disk that stores only changes from a parent.
    Differencing,
}
```

### Examples

Include examples in documentation:

```rust
/// # Example
///
/// ```
/// use totalimage_vaults::{VhdVault, VaultConfig};
///
/// let vault = VhdVault::open("disk.vhd", VaultConfig::default())?;
/// println!("Disk size: {} bytes", vault.size());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
```

### Updating Documentation

- **API changes:** Update `docs/API.md`
- **CLI changes:** Update `docs/CLI.md`
- **Architecture changes:** Update `docs/ARCHITECTURE.md`
- **README:** Keep examples current

---

## Getting Help

- **Questions:** Open a GitHub Discussion
- **Bugs:** File an issue with reproduction steps
- **Security:** Email security@totalimage.com (do not file public issue)

---

## License

By contributing, you agree that your contributions will be licensed under the GPL-3.0 License.
