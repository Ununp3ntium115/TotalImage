# Fuzzing TotalImage

This directory contains fuzz targets for critical parsers in TotalImage using cargo-fuzz and libFuzzer.

## Fuzz Targets

1. **fuzz_mbr_parser** - MBR partition table parser
2. **fuzz_gpt_parser** - GPT partition table parser
3. **fuzz_fat_bpb** - FAT BIOS Parameter Block parser
4. **fuzz_vhd_footer** - VHD footer parser
5. **fuzz_e01_header** - E01 file header parser

## Running Fuzz Tests

### Prerequisites

```bash
cargo install cargo-fuzz
rustup install nightly  # Fuzzing requires nightly Rust
```

### Run Individual Fuzz Target

```bash
# Run for 60 seconds
cargo +nightly fuzz run fuzz_mbr_parser -- -max_total_time=60

# Run for 1 hour (recommended minimum)
cargo +nightly fuzz run fuzz_mbr_parser -- -max_total_time=3600

# Run with parallel jobs
cargo +nightly fuzz run fuzz_mbr_parser -- -workers=4 -max_total_time=3600
```

### Run All Fuzz Targets

```bash
# Run each target for 1 hour
for target in fuzz_mbr_parser fuzz_gpt_parser fuzz_fat_bpb fuzz_vhd_footer fuzz_e01_header; do
    echo "Fuzzing $target..."
    cargo +nightly fuzz run $target -- -max_total_time=3600
done
```

### Check Coverage

```bash
cargo +nightly fuzz coverage fuzz_mbr_parser
```

## Interpreting Results

- **Crashes**: Saved to `fuzz/artifacts/<target>/crash-*`
- **Hangs**: Saved to `fuzz/artifacts/<target>/timeout-*`
- **Corpus**: Interesting inputs saved to `fuzz/corpus/<target>/`

### Reproducing Crashes

```bash
cargo +nightly fuzz run fuzz_mbr_parser fuzz/artifacts/fuzz_mbr_parser/crash-*
```

## Continuous Fuzzing

For production hardening, run fuzzing continuously on a dedicated server:

```bash
# Run indefinitely with 8 parallel jobs
cargo +nightly fuzz run fuzz_mbr_parser -- -workers=8
```

## Expected Behavior

These parsers are designed to handle malformed input gracefully:
- Invalid signatures → Return error
- Out-of-bounds offsets → Return error
- Overflow in calculations → Return error (checked arithmetic)
- Invalid checksums → Return error

Fuzzing should NOT find:
- Panics or unwraps
- Segfaults
- Undefined behavior
- Memory leaks
