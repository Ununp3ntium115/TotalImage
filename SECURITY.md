# Security Policy

## Overview

TotalImage is a disk image analysis tool that parses untrusted binary data from disk images, partition tables, and filesystems. Security is a critical concern, as maliciously crafted disk images could exploit parsing vulnerabilities.

## Security Guarantees

### Memory Safety

TotalImage is written in Rust, which provides:
- **Memory safety**: No buffer overflows, use-after-free, or null pointer dereferences
- **Thread safety**: Data race protection through the type system
- **Safe abstractions**: Bounds-checked array access and validated pointer operations

### Input Validation

All parsers implement defense-in-depth validation:

1. **Integer Overflow Protection**
   - All arithmetic uses checked operations (`checked_add`, `checked_mul`, etc.)
   - Explicit validation before type casts that could truncate values
   - Safe conversion helpers in `totalimage_core::security` module

2. **Allocation Limits**
   - Maximum sector size: 4KB
   - Maximum single buffer: 256 MB
   - Maximum FAT table: 100 MB
   - Maximum file extraction: 1 GB
   - Maximum cluster chain: 1 million entries

3. **Path Traversal Prevention**
   - All file paths validated through `validate_file_path()`
   - Canonical path resolution to prevent `../` attacks
   - Null byte rejection in paths

4. **Resource Limits**
   - Maximum partition count: 256 (prevents table exhaustion)
   - Maximum directory entries per read: 10,000
   - Timeout protection on web API endpoints (2 minutes)

### Secure Coding Practices

- **No unsafe code**: All code uses safe Rust (no `unsafe` blocks except in well-audited dependencies)
- **Minimal dependencies**: Careful vetting of third-party crates
- **Explicit error handling**: All errors propagated, no silent failures
- **Security-focused tests**: Test cases for malformed inputs and edge cases

## Security Features by Component

### Core (`totalimage-core`)
- **Security module** (`src/security.rs`): Centralized validation functions
- **Checked arithmetic**: `checked_multiply_u64()`, `validate_allocation_size()`
- **Validation constants**: `MAX_SECTOR_SIZE`, `MAX_ALLOCATION_SIZE`, etc.

### Vaults (`totalimage-vaults`)
- **VHD parser**: Validates footer checksums, header sizes, BAT offsets
- **Raw vault**: Bounds-checked seeking and reading

### Zones (`totalimage-zones`)
- **MBR parser**: Validates boot signature, partition table bounds
- **GPT parser**: Validates header CRC32, partition array CRC32, LBA ranges

### Territories (`totalimage-territories`)
- **FAT parser**: Checked BPB arithmetic, validated cluster chains, file size limits
- **ISO-9660 parser**: Both-endian validation, directory record bounds checking

### Web API (`totalimage-web`)
- **Path validation**: All file paths validated before filesystem access
- **Error sanitization**: Internal details not exposed in error responses
- **Cache isolation**: Separate cache database per instance

## Known Limitations

### Current Security Considerations

1. **Symbolic Links**: Path validation canonicalizes symlinks, which may allow access outside intended directories if symlinks exist
2. **Time-of-check Time-of-use (TOCTOU)**: File validation and access are not atomic
3. **Resource Exhaustion**: Large valid disk images can still consume significant memory
4. **Denial of Service**: Web API has no rate limiting (planned for production)

### Not Implemented (Future Work)

- [ ] Fuzzing harness for automated vulnerability discovery
- [ ] Rate limiting on web API endpoints
- [ ] Request size limits on API payloads
- [ ] CORS policy configuration
- [ ] TLS/HTTPS support for web server
- [ ] Authentication/authorization for API access
- [ ] Audit logging for security events
- [ ] Sandboxing for parser execution

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

If you discover a security vulnerability in TotalImage, please report it privately:

1. **Email**: [MAINTAINER_EMAIL] (TODO: Add actual contact)
2. **Subject**: "TotalImage Security Vulnerability Report"
3. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Suggested fix (if available)
   - Whether you'd like to be credited in the disclosure

### Response Timeline

- **Acknowledgment**: Within 48 hours of report
- **Initial Assessment**: Within 1 week
- **Fix Development**: Depends on severity (critical: immediate, high: 1-2 weeks)
- **Public Disclosure**: After fix is released and users have time to update (typically 2-4 weeks)

### Coordinated Disclosure

We follow responsible disclosure practices:
1. Reporter notifies maintainers privately
2. Maintainers develop and test fix
3. Fix is released in a security patch
4. Public advisory is published after release
5. Reporter is credited (if desired)

## Security Hardening Checklist

For production deployments:

### Web API
- [ ] Run behind reverse proxy (nginx, Caddy) with TLS
- [ ] Enable rate limiting (e.g., 100 requests/minute per IP)
- [ ] Set `TOTALIMAGE_CACHE_DIR` to isolated directory
- [ ] Restrict file access to specific directories only
- [ ] Configure CORS policy for your domain
- [ ] Enable request logging for audit trail
- [ ] Set resource limits (ulimit, cgroups)

### CLI Tool
- [ ] Run as non-privileged user
- [ ] Validate all input file paths
- [ ] Set file size limits (e.g., max 10GB images)
- [ ] Use read-only mounts when possible

### General
- [ ] Keep dependencies updated (`cargo update`)
- [ ] Run `cargo audit` regularly for known vulnerabilities
- [ ] Monitor security advisories for Rust ecosystem
- [ ] Review logs for suspicious patterns

## Security Audit History

- **2025-11-22**: Initial security hardening
  - Added security validation module
  - Implemented checked arithmetic throughout
  - Fixed path traversal vulnerability (SEC-003)
  - Created comprehensive gap analysis

## References

### Security Resources
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [Memory Safety in Rust](https://doc.rust-lang.org/nomicon/)

### Vulnerability Categories Addressed
- **CWE-190**: Integer Overflow or Wraparound ✓
- **CWE-22**: Path Traversal ✓
- **CWE-400**: Uncontrolled Resource Consumption (partial)
- **CWE-125**: Out-of-bounds Read ✓ (via Rust memory safety)
- **CWE-787**: Out-of-bounds Write ✓ (via Rust memory safety)

## Security Contact

For security-related questions or concerns:
- **GitHub**: Open a security advisory at https://github.com/Ununp3ntium115/TotalImage/security/advisories
- **Email**: [TODO: Add security contact]

---

**Last Updated**: 2025-11-22
**Version**: 0.1.0
