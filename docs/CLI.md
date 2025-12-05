# TotalImage CLI Documentation

## Installation

```bash
cargo install --path crates/totalimage-cli
```

Or build from source:

```bash
cargo build --release
./target/release/totalimage-cli --help
```

---

## Commands

### `info` - Display Vault Information

Show metadata about a disk image container.

**Syntax:**
```bash
totalimage-cli info <IMAGE_FILE>
```

**Example:**
```bash
$ totalimage-cli info /data/evidence.vhd

Vault: /data/evidence.vhd
Type: VHD Dynamic
Size: 10.0 GB (10737418240 bytes)

Partition Table: GPT
  GUID: a1b2c3d4-e5f6-7890-abcd-ef0123456789
  Partitions: 3
```

---

### `zones` - List Partitions

List all partitions in a disk image.

**Syntax:**
```bash
totalimage-cli zones <IMAGE_FILE>
```

**Example:**
```bash
$ totalimage-cli zones /data/disk.e01

Partition Table: MBR
Disk Signature: 0xDEADBEEF

Zone 0:
  Type: NTFS (0x07)
  Offset: 1048576 (1.0 MB)
  Length: 536870912 (512 MB)

Zone 1:
  Type: FAT32 LBA (0x0C)
  Offset: 537919488 (513 MB)
  Length: 10200547328 (9.5 GB)
```

---

### `list` - List Files in Partition

Browse files and directories in a partition's filesystem.

**Syntax:**
```bash
totalimage-cli list <IMAGE_FILE> [OPTIONS]
```

**Options:**
- `--zone <N>` - Partition index (default: 0)
- `--path <DIR>` - Directory path (default: "/")

**Example:**
```bash
$ totalimage-cli list /data/usb.vhd --zone 1 --path "/Users"

Directory: /Users
Filesystem: FAT32

alice/               [DIR]  2024-01-15 14:30:00
bob/                 [DIR]  2024-02-20 09:15:30
Shared/              [DIR]  2023-12-01 11:22:45
readme.txt           2.4 KB 2024-03-10 16:48:12
```

---

### `extract` - Extract File from Image

Extract a file from a partition to local filesystem.

**Syntax:**
```bash
totalimage-cli extract <IMAGE_FILE> <FILE_PATH> [OPTIONS]
```

**Options:**
- `--zone <N>` - Partition index (default: 0)
- `--output <PATH>` - Output file path (default: current directory)

**Example:**
```bash
$ totalimage-cli extract /data/evidence.vhd "/Users/alice/document.pdf" \
    --zone 1 \
    --output ./extracted/document.pdf

Extracting: /Users/alice/document.pdf
  From: /data/evidence.vhd (zone 1)
  To: ./extracted/document.pdf
  Size: 2.35 MB (2458624 bytes)

✓ File extracted successfully
```

---

## Common Usage Patterns

### Quick Image Inspection

```bash
# Check what type of image you have
totalimage-cli info disk.img

# See all partitions
totalimage-cli zones disk.img

# Browse root directory of first partition
totalimage-cli list disk.img
```

### Multi-Partition Analysis

```bash
# List all partitions
totalimage-cli zones evidence.e01

# Browse each partition
totalimage-cli list evidence.e01 --zone 0
totalimage-cli list evidence.e01 --zone 1
totalimage-cli list evidence.e01 --zone 2
```

### Batch File Extraction

```bash
#!/bin/bash
IMAGE="/data/evidence.vhd"
ZONE=1
OUTPUT_DIR="./extracted"

# Extract multiple files
files=(
  "/Users/suspect/Documents/report.pdf"
  "/Users/suspect/Downloads/evidence.zip"
  "/Users/suspect/Pictures/photo.jpg"
)

mkdir -p "$OUTPUT_DIR"

for file in "${files[@]}"; do
  basename=$(basename "$file")
  totalimage-cli extract "$IMAGE" "$file" \
    --zone "$ZONE" \
    --output "$OUTPUT_DIR/$basename"
done
```

---

## Error Handling

### File Not Found

```bash
$ totalimage-cli info /nonexistent/image.vhd

Error: File not found: /nonexistent/image.vhd

Suggestions:
  - Check the file path is correct
  - Verify you have read permissions
  - Use absolute paths to avoid confusion
```

### Invalid Image Format

```bash
$ totalimage-cli info /path/to/text_file.txt

Error: Unable to detect vault format

Supported formats:
  - Raw disk images (.img, .ima, .bin)
  - VHD (Virtual Hard Disk)
  - E01 (EnCase Evidence)
  - AFF4 (Advanced Forensic Format)

The file may be corrupted or in an unsupported format.
```

### Permission Denied

```bash
$ totalimage-cli zones /root/secure.vhd

Error: Permission denied: /root/secure.vhd

Suggestions:
  - Run with sudo: sudo totalimage-cli zones /root/secure.vhd
  - Check file permissions: ls -l /root/secure.vhd
  - Copy file to accessible location
```

---

## Supported Formats

### Vault Containers

| Format | Extension | Read | Notes |
|--------|-----------|------|-------|
| Raw | .img, .ima, .bin | ✅ | Sector-by-sector copies |
| VHD | .vhd, .vhdx | ✅ | Fixed, Dynamic, Differencing |
| E01 | .e01 | ✅ | EnCase Evidence Format |
| AFF4 | .aff4 | ✅ | All compression methods |

### Partition Tables

| Type | Read | Notes |
|------|------|-------|
| MBR | ✅ | Master Boot Record, 4 primary partitions |
| GPT | ✅ | GUID Partition Table, CRC validation |

### Filesystems

| Type | Read | Write | Notes |
|------|------|-------|-------|
| FAT12/16/32 | ✅ | ❌ | Long File Names supported |
| exFAT | ✅ | ❌ | 64-bit file sizes |
| NTFS | ✅ | ❌ | Read-only, no compression yet |
| ISO-9660 | ✅ | ❌ | Rock Ridge extensions |

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TOTALIMAGE_ALLOWED_ROOT` | Allowed base paths | Current directory |
| `RUST_LOG` | Logging level | `info` |
| `RUST_BACKTRACE` | Show backtraces on panic | `0` |

**Example:**
```bash
# Enable debug logging
RUST_LOG=debug totalimage-cli info disk.vhd

# Allow specific paths
TOTALIMAGE_ALLOWED_ROOT=/data:/evidence totalimage-cli list /data/case1.e01
```

---

## Performance Tips

### Large Images

For multi-gigabyte images, operations may take time:

```bash
# Use --zone to target specific partitions
totalimage-cli list huge-disk.vhd --zone 0  # Faster

# Avoid listing root of large partitions
totalimage-cli list huge-disk.vhd --zone 0 --path "/Specific/Folder"
```

### Network Paths

Avoid running directly on network mounts:

```bash
# Slow ❌
totalimage-cli info /mnt/network/image.vhd

# Fast ✅ (copy locally first)
cp /mnt/network/image.vhd /tmp/
totalimage-cli info /tmp/image.vhd
```

---

## Troubleshooting

### Stuck or Slow Operations

Press `Ctrl+C` to cancel. Then try:

1. Check image isn't corrupted: `file image.vhd`
2. Verify format: `hexdump -C image.vhd | head`
3. Test with small partition first: `--zone 0`

### Memory Issues

For very large operations:

```bash
# Limit memory via ulimit
ulimit -v 2000000  # 2GB limit
totalimage-cli extract huge.vhd "/large/file.iso"
```

---

## See Also

- [API Documentation](API.md)
- [MCP Integration](MCP.md)
- [Architecture Guide](ARCHITECTURE.md)
