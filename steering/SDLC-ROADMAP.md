# TotalImage SDLC Roadmap

**Created:** 2025-11-26
**Target:** 100% Production-Ready FTK Imager Replacement

---

## Executive Summary

Based on comprehensive gap analysis (28 issues identified across 6 crates), this roadmap outlines the path from current state to production-ready deployment.

**Current State:** ~95% feature complete, 100+ tests, 23 gaps remaining (5 fixed)
**Target State:** 100% complete, >80% test coverage, PYRO Platform integrated

**Progress (2025-11-26):**
- ✅ GAP-001, GAP-002, GAP-004, GAP-007 fixed
- ✅ NTFS filesystem implemented (864 lines)
- ✅ 22 MCP protocol tests added

---

## Phase Breakdown

### Phase 1: CRITICAL FIXES (Week 1) - DEV
**Focus:** Fix data corruption risks and security issues

| ID | Task | Priority | Hours | Status |
|----|------|----------|-------|--------|
| 1.1 | Fix AFF4 silent decompression failure | P0 | 2 | ✅ Done |
| 1.2 | Fix E01 silent decompression failure | P0 | 2 | ✅ Done |
| 1.3 | Fix AFF4 chunk offset calculation | P0 | 4 | Pending |
| 1.4 | Add path traversal whitelist | P0 | 4 | Pending |
| 1.5 | Add cache size limits (AFF4/E01) | P1 | 3 | ✅ Done |
| 1.6 | Add integer overflow checks | P1 | 2 | Pending |

**Exit Criteria:** All P0 issues resolved, no data corruption paths

---

### Phase 2: TEST COVERAGE (Weeks 1-2) - DEV/QA
**Focus:** Achieve minimum viable test coverage

| ID | Task | Priority | Hours | Status |
|----|------|----------|-------|--------|
| 2.1 | MCP Server tests (auth, protocol) | P0 | 8 | ✅ Done (22 tests) |
| 2.2 | Fire Marshal server tests | P0 | 6 | Pending |
| 2.3 | MCP Tools tests (extract, validate) | P0 | 8 | Pending |
| 2.4 | E01 vault edge case tests | P1 | 6 | Pending |
| 2.5 | AFF4 vault edge case tests | P1 | 6 | Pending |
| 2.6 | FAT/exFAT cluster chain tests | P1 | 4 | Pending |
| 2.7 | VHD differencing chain tests | P1 | 4 | Pending |

**Exit Criteria:** 64 → 137 tests, critical paths covered

---

### Phase 3: NTFS FILESYSTEM (Weeks 2-4) - DEV
**Focus:** Read-only NTFS support using `ntfs` crate

| ID | Task | Priority | Hours | Status |
|----|------|----------|-------|--------|
| 3.1 | Add ntfs dependency, scaffold module | P2 | 4 | ✅ Done |
| 3.2 | Boot sector parsing | P2 | 8 | ✅ Done (via ntfs crate) |
| 3.3 | MFT record parsing | P2 | 12 | ✅ Done (via ntfs crate) |
| 3.4 | Data run decoding | P2 | 10 | ✅ Done (via ntfs crate) |
| 3.5 | MFT caching layer | P2 | 12 | ✅ Done (via ntfs crate) |
| 3.6 | Directory index (B-tree) | P2 | 14 | ✅ Done |
| 3.7 | File data reading | P2 | 14 | ✅ Done |
| 3.8 | Territory trait implementation | P2 | 10 | ✅ Done |
| 3.9 | Sparse file handling | P2 | 12 | ✅ Done (via ntfs crate) |
| 3.10 | Comprehensive NTFS tests | P2 | 8 | Partial (4 tests) |

**Exit Criteria:** NTFS read, list, extract working on Windows 10 images ✅

---

### Phase 4: DOCUMENTATION (Week 4) - DEV
**Focus:** Complete rustdoc coverage

| ID | Task | Priority | Hours | Status |
|----|------|----------|-------|--------|
| 4.1 | Document all public types in AFF4 | P1 | 4 | Pending |
| 4.2 | Document MCP auth module | P1 | 2 | Pending |
| 4.3 | Document MCP protocol module | P1 | 2 | Pending |
| 4.4 | Add API examples to vaults | P2 | 4 | Pending |
| 4.5 | Add API examples to territories | P2 | 4 | Pending |
| 4.6 | Security notes in public APIs | P1 | 2 | Pending |

**Exit Criteria:** All public items documented, examples provided

---

### Phase 5: SVELTE UI (Weeks 5-7) - DEV
**Focus:** Web frontend for disk image analysis

| ID | Task | Priority | Hours | Status |
|----|------|----------|-------|--------|
| 5.1 | Project scaffold (SvelteKit + Tailwind) | P2 | 4 | Pending |
| 5.2 | Image upload/selection component | P2 | 8 | Pending |
| 5.3 | Partition table viewer | P2 | 8 | Pending |
| 5.4 | File browser component | P2 | 12 | Pending |
| 5.5 | File extraction UI | P2 | 8 | Pending |
| 5.6 | Integrity validation UI | P2 | 6 | Pending |
| 5.7 | WebSocket progress display | P2 | 6 | Pending |
| 5.8 | Dark mode + responsive design | P3 | 8 | Pending |

**Exit Criteria:** Functional web UI connected to MCP server

---

### Phase 6: PRODUCTION HARDENING (Week 8) - QA/PROD
**Focus:** Security, performance, deployment

| ID | Task | Priority | Hours | Status |
|----|------|----------|-------|--------|
| 6.1 | TLS/HTTPS support | P1 | 8 | Pending |
| 6.2 | Performance benchmarking | P2 | 8 | Pending |
| 6.3 | Load testing (100+ concurrent) | P2 | 6 | Pending |
| 6.4 | Fuzzing harness setup | P2 | 8 | Pending |
| 6.5 | Docker image optimization | P2 | 4 | Pending |
| 6.6 | Kubernetes manifests | P3 | 8 | Pending |
| 6.7 | CI/CD pipeline | P2 | 8 | Pending |

**Exit Criteria:** Production-ready deployment artifacts

---

### Phase 7: PYRO INTEGRATION (Week 9) - PROD
**Focus:** Full PYRO Platform integration

| ID | Task | Priority | Hours | Status |
|----|------|----------|-------|--------|
| 7.1 | Deploy to PYRO infrastructure | P1 | 8 | Pending |
| 7.2 | Configure job queues | P1 | 4 | Pending |
| 7.3 | Set up monitoring/alerting | P1 | 6 | Pending |
| 7.4 | Integration testing with PYRO | P1 | 8 | Pending |
| 7.5 | Documentation for PYRO users | P2 | 4 | Pending |

**Exit Criteria:** TotalImage live on PYRO Platform

---

## Issue Tracker

### Critical Issues (P0) - Must Fix Before Any Deployment

| ID | Issue | Location | Status |
|----|-------|----------|--------|
| GAP-001 | Silent AFF4 decompression failure | `aff4/mod.rs:361-366` | ✅ Fixed |
| GAP-002 | Silent E01 decompression failure | `e01/mod.rs:298-304` | ✅ Fixed |
| GAP-003 | AFF4 chunk offset calculation bug | `aff4/mod.rs:346` | Open |
| GAP-004 | No MCP server tests | `mcp/server.rs` | ✅ Fixed (22 tests) |
| GAP-005 | No Fire Marshal tests | `fire-marshal/server.rs` | Open |

### High Priority Issues (P1) - Must Fix Before Production

| ID | Issue | Location | Status |
|----|-------|----------|--------|
| GAP-006 | Path traversal incomplete | `core/security.rs:133` | Open |
| GAP-007 | Unbounded AFF4 cache | `aff4/mod.rs:375-378` | ✅ Fixed (LRU eviction) |
| GAP-008 | E01 cache not limited | `e01/mod.rs:73-92` | Open |
| GAP-009 | VHD chain depth undocumented | `vhd/mod.rs:315-320` | Open |
| GAP-010 | Missing rustdoc on auth module | `mcp/auth.rs` | Open |
| GAP-011 | Snappy/LZ4 not implemented | `aff4/mod.rs:369-372` | Open |

### Medium Priority Issues (P2) - Should Fix

| ID | Issue | Location | Status |
|----|-------|----------|--------|
| GAP-012 | Integer overflow in AFF4 | `aff4/mod.rs:303` | Open |
| GAP-013 | Redundant calculation in E01 | `e01/mod.rs:217-232` | Open |
| GAP-014 | Inefficient AFF4 chunk lookup | `aff4/mod.rs:312-326` | Open |
| GAP-015 | Dead code annotations | Multiple files | Open |
| GAP-016 | Missing parameter documentation | `vhd/mod.rs:262-268` | Open |

---

## Timeline Summary

| Phase | Duration | Start | End | Key Deliverable |
|-------|----------|-------|-----|-----------------|
| Phase 1: Critical Fixes | 1 week | Week 1 | Week 1 | No data corruption |
| Phase 2: Test Coverage | 2 weeks | Week 1 | Week 2 | 137+ tests |
| Phase 3: NTFS | 3 weeks | Week 2 | Week 4 | NTFS read-only |
| Phase 4: Documentation | 1 week | Week 4 | Week 4 | Full rustdoc |
| Phase 5: Svelte UI | 3 weeks | Week 5 | Week 7 | Web frontend |
| Phase 6: Production | 1 week | Week 8 | Week 8 | TLS, benchmarks |
| Phase 7: PYRO | 1 week | Week 9 | Week 9 | Live deployment |

**Total Estimated Effort:** 280-320 hours (~7-8 weeks)

---

## Quality Gates

### DEV → UA Gate
- [ ] All P0 issues resolved
- [ ] 100+ tests passing
- [ ] No compiler warnings
- [ ] Clippy clean

### UA → QA Gate
- [ ] All P1 issues resolved
- [ ] 137+ tests passing
- [ ] NTFS basic operations working
- [ ] Documentation complete

### QA → PROD Gate
- [ ] All P2 issues resolved
- [ ] Performance benchmarks met
- [ ] Security scan passed
- [ ] TLS configured
- [ ] Docker images built

---

## Test Coverage Targets

| Crate | Current | Target | Gap |
|-------|---------|--------|-----|
| totalimage-core | 4 | 10 | +6 |
| totalimage-vaults | 59 | 85 | +26 |
| totalimage-zones | 20 | 30 | +10 |
| totalimage-territories | 36 | 55 | +19 |
| totalimage-mcp | 5 | 25 | +20 |
| fire-marshal | 7 | 15 | +8 |
| **Total** | **131** | **220** | **+89** |

---

## Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| NTFS complexity underestimated | Medium | High | Use ntfs crate, incremental delivery |
| Test coverage takes longer | Medium | Medium | Prioritize critical paths first |
| PYRO integration delays | Low | High | Early integration testing |
| Performance issues at scale | Low | Medium | Benchmark early, optimize hot paths |

---

## Next Actions

### Immediate (Today)
1. Start fixing GAP-001 (AFF4 decompression)
2. Start fixing GAP-002 (E01 decompression)

### This Week
3. Fix GAP-003 through GAP-005
4. Add MCP server tests
5. Add Fire Marshal tests

### Next Week
6. Start NTFS Phase 3.1-3.4
7. Complete test coverage Phase 2

---

*This document should be updated weekly with progress.*
