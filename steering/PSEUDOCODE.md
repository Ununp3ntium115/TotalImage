# TotalImage Pseudocode Specification

**Version:** 0.1.0-alpha
**Last Updated:** 2025-11-28
**Status:** Complete

This document provides pseudocode specifications for all TotalImage components using the CRYPTEX anarchist terminology framework.

---

## Table of Contents

1. [Core Arsenal (totalimage-core)](#1-core-arsenal)
2. [Vault Cells (totalimage-vaults)](#2-vault-cells)
3. [Zone Cells (totalimage-zones)](#3-zone-cells)
4. [Territory Cells (totalimage-territories)](#4-territory-cells)
5. [Pipeline Cells (totalimage-pipeline)](#5-pipeline-cells)
6. [MCP Collective (totalimage-mcp)](#6-mcp-collective)
7. [CLI Front (totalimage-cli)](#7-cli-front)
8. [Web Front (totalimage-web)](#8-web-front)
9. [Fire Marshal (fire-marshal)](#9-fire-marshal)
10. [Acquire Collective (totalimage-acquire)](#10-acquire-collective)

---

## 1. Core Arsenal

**Real Name:** `totalimage-core`
**Brand Name:** Arsenal
**Purpose:** Foundational abstractions for disk image processing

### 1.1 Vault Trait (Container Interface)

```pseudocode
TRAIT Vault:
    """Interface for all container format handlers"""

    ACTION identify() -> string:
        """Return vault type identifier (e.g., "VHD", "E01", "AFF4")"""
        RETURN self.type_name

    ACTION length() -> uint64:
        """Return total size of contained data in bytes"""
        RETURN self.total_size

    ACTION content() -> ReadSeekStream:
        """Return mutable reference to data stream"""
        RETURN self.inner_stream
```

### 1.2 ZoneTable Trait (Partition Interface)

```pseudocode
TRAIT ZoneTable:
    """Interface for partition table parsers"""

    ACTION identify() -> string:
        """Return partition table type (e.g., "MBR", "GPT")"""
        RETURN self.table_type

    ACTION enumerate_zones() -> list[Zone]:
        """Return all partitions in table"""
        RETURN self.zones

    ACTION get_zone(index: uint) -> Zone or None:
        """Get specific partition by index"""
        IF index < length(self.zones):
            RETURN self.zones[index]
        ELSE:
            RETURN None
```

### 1.3 Territory Trait (Filesystem Interface)

```pseudocode
TRAIT Territory:
    """Interface for filesystem implementations"""

    ACTION identify() -> string:
        """Return filesystem type (e.g., "FAT32", "NTFS")"""
        RETURN self.fs_type

    ACTION banner() -> string:
        """Return volume label"""
        RETURN self.volume_label or ""

    ACTION headquarters() -> DirectoryCell:
        """Return root directory"""
        RETURN self.root_directory

    ACTION domain_size() -> uint64:
        """Return total filesystem size"""
        RETURN self.total_size

    ACTION liberated_space() -> uint64:
        """Return free space available"""
        RETURN self.free_space

    ACTION navigate_to(path: string) -> DirectoryCell:
        """Navigate to specific path"""
        current = self.headquarters()
        FOR component IN split_path(path):
            current = current.enter(component)
        RETURN current

    ACTION extract_file(path: string) -> bytes:
        """Extract file contents to memory"""
        directory = self.navigate_to(parent_of(path))
        file_info = directory.get_occupant(filename_of(path))
        RETURN self.read_file_data(file_info)
```

### 1.4 DirectoryCell Trait (Directory Interface)

```pseudocode
TRAIT DirectoryCell:
    """Interface for directory entries"""

    ACTION name() -> string:
        """Return directory name"""
        RETURN self.dir_name

    ACTION list_occupants() -> list[OccupantInfo]:
        """List all files and subdirectories"""
        RETURN self.entries

    ACTION enter(name: string) -> DirectoryCell:
        """Enter subdirectory by name"""
        FOR entry IN self.entries:
            IF entry.name == name AND entry.is_directory:
                RETURN load_directory(entry)
        RAISE NotFound(name)

    ACTION get_occupant(name: string) -> OccupantInfo or None:
        """Get specific file/directory info"""
        FOR entry IN self.entries:
            IF entry.name == name:
                RETURN entry
        RETURN None
```

### 1.5 OccupantInfo Structure

```pseudocode
STRUCTURE OccupantInfo:
    """File or directory metadata"""

    name: string              # Filename or directory name
    is_directory: boolean     # True if directory
    size: uint64              # File size in bytes (0 for directories)
    created: datetime or None # Creation timestamp
    modified: datetime or None # Last modification timestamp
    accessed: datetime or None # Last access timestamp
    attributes: uint32        # Filesystem-specific attributes

    CONSTRUCTOR file(name: string, size: uint64):
        RETURN OccupantInfo(name, is_directory=False, size=size)

    CONSTRUCTOR directory(name: string):
        RETURN OccupantInfo(name, is_directory=True, size=0)
```

### 1.6 Zone Structure

```pseudocode
STRUCTURE Zone:
    """Partition/zone metadata"""

    index: uint               # Partition index (0-based)
    offset: uint64            # Byte offset from start of disk
    length: uint64            # Size in bytes
    zone_type: string         # Partition type identifier
    territory_type: string or None  # Detected filesystem type
```

### 1.7 Security Validation Actions

```pseudocode
CONST MAX_SECTOR_SIZE = 4096
CONST MAX_ALLOCATION_SIZE = 256 * 1024 * 1024  # 256 MB
CONST MAX_FILE_EXTRACT_SIZE = 1024 * 1024 * 1024  # 1 GB
CONST MAX_CLUSTER_CHAIN_LENGTH = 1_000_000
CONST MAX_PARTITION_COUNT = 256

ACTION validate_allocation_size(size: uint64, limit: uint, context: string):
    """Prevent memory exhaustion attacks"""
    IF size > limit:
        RAISE InvalidOperation("Allocation too large: " + context)
    RETURN size as uint

ACTION checked_multiply(a: uint64, b: uint64, context: string) -> uint64:
    """Prevent integer overflow"""
    result = a * b
    IF result / b != a:
        RAISE InvalidOperation("Integer overflow: " + context)
    RETURN result

ACTION validate_file_path(path: string) -> PathBuf:
    """Prevent directory traversal attacks"""
    IF ".." IN path OR path.starts_with("/"):
        RAISE InvalidPath("Path traversal detected")
    RETURN canonicalize(path)
```

---

## 2. Vault Cells

**Real Name:** `totalimage-vaults`
**Brand Name:** Vault Cells
**Purpose:** Container format handlers

### 2.1 Vault Factory (Underground Network)

```pseudocode
ENUM VaultType:
    Raw, Vhd, E01, Aff4, Unknown

ACTION detect_vault_type(path: Path) -> VaultType:
    """Reconnaissance: Identify container format by magic bytes"""

    file = open(path, "rb")
    header = file.read(16)

    IF header[0:8] == "conectix":
        RETURN VaultType.Vhd
    ELSE IF header[0:8] == EVF_SIGNATURE:
        RETURN VaultType.E01
    ELSE IF is_zip_file(path) AND contains_aff4_metadata(path):
        RETURN VaultType.Aff4
    ELSE:
        RETURN VaultType.Raw

ACTION open_vault(path: Path, config: VaultConfig) -> Vault:
    """Underground Network: Create appropriate vault instance"""

    vault_type = detect_vault_type(path)

    MATCH vault_type:
        CASE VaultType.Raw:
            RETURN RawVault.open(path, config)
        CASE VaultType.Vhd:
            RETURN VhdVault.open(path, config)
        CASE VaultType.E01:
            RETURN E01Vault.open(path)
        CASE VaultType.Aff4:
            RETURN Aff4Vault.open(path)
        DEFAULT:
            RAISE InvalidVault("Unknown format")
```

### 2.2 Raw Vault Cell

```pseudocode
CELL RawVault IMPLEMENTS Vault:
    """Raw/DD image format handler"""

    pipeline: ReadSeekStream  # Direct file access
    length: uint64            # File size

    CONSTRUCTOR open(path: Path, config: VaultConfig):
        IF config.use_mmap AND file_size(path) <= MAX_MMAP_SIZE:
            pipeline = MmapPipeline.open(path)
        ELSE:
            pipeline = FileStream.open(path)
        length = file_size(path)
        RETURN RawVault(pipeline, length)

    ACTION identify():
        RETURN "Raw"

    ACTION length():
        RETURN self.length

    ACTION content():
        RETURN self.pipeline
```

### 2.3 VHD Vault Cell

```pseudocode
ENUM VhdType:
    Fixed = 2, Dynamic = 3, Differencing = 4

STRUCTURE VhdFooter:
    """VHD footer at end of file (512 bytes)"""

    cookie: bytes[8]          # "conectix"
    features: uint32
    version: uint32
    data_offset: uint64       # Offset to dynamic header (or 0xFFFFFFFF)
    timestamp: uint32
    creator_app: bytes[4]
    original_size: uint64
    current_size: uint64
    geometry: DiskGeometry
    disk_type: VhdType
    checksum: uint32
    uuid: bytes[16]

STRUCTURE VhdDynamicHeader:
    """Dynamic disk header (1024 bytes)"""

    cookie: bytes[8]          # "cxsparse"
    table_offset: uint64      # Offset to BAT
    header_version: uint32
    max_table_entries: uint32
    block_size: uint32        # Usually 2 MB

CELL VhdVault IMPLEMENTS Vault:
    """VHD format handler with differencing chain support"""

    file: FileStream
    footer: VhdFooter
    dynamic_header: VhdDynamicHeader or None
    bat: BlockAllocationTable or None
    parent: VhdVault or None  # For differencing disks

    CONSTRUCTOR open(path: Path, config: VaultConfig):
        file = open(path, "rb")

        # Read footer from end of file
        file.seek(-512, END)
        footer_bytes = file.read(512)
        footer = VhdFooter.parse(footer_bytes)

        # Verify checksum
        IF NOT footer.verify_checksum():
            RAISE InvalidVault("VHD footer checksum mismatch")

        # Load dynamic header if needed
        IF footer.disk_type IN [Dynamic, Differencing]:
            file.seek(footer.data_offset)
            dynamic_header = VhdDynamicHeader.parse(file.read(1024))
            bat = load_block_allocation_table(file, dynamic_header)

        # Load parent for differencing disks
        IF footer.disk_type == Differencing:
            parent_path = resolve_parent_locator(footer, path)
            parent = VhdVault.open(parent_path, config)

        RETURN VhdVault(file, footer, dynamic_header, bat, parent)

    ACTION read_block(block_index: uint) -> bytes:
        """Read a block, following differencing chain if needed"""

        IF self.bat[block_index] == 0xFFFFFFFF:
            # Block not allocated
            IF self.parent:
                RETURN self.parent.read_block(block_index)
            ELSE:
                RETURN zeros(self.dynamic_header.block_size)
        ELSE:
            offset = self.bat[block_index] * 512
            self.file.seek(offset)
            RETURN self.file.read(self.dynamic_header.block_size)
```

### 2.4 E01 Vault Cell

```pseudocode
CONST EVF_SIGNATURE = [0x45, 0x56, 0x46, 0x09, 0x0D, 0x0A, 0xFF, 0x00]

ENUM E01SectionType:
    Header, Volume, Disk, Sectors, Table, Table2, Hash, Done, Next, Data

STRUCTURE E01ChunkInfo:
    """Chunk location and compression info"""
    offset: uint64
    compressed_size: uint32
    is_compressed: boolean

CELL E01Vault IMPLEMENTS Vault:
    """EnCase E01 format handler"""

    reader: ReadSeekStream
    volume: E01VolumeSection
    chunk_table: list[E01ChunkInfo]
    hash: E01HashSection or None
    cache: E01Cache           # Single-chunk cache
    position: uint64

    CONSTRUCTOR open(path: Path):
        reader = open(path, "rb")

        # Read file header
        header_bytes = reader.read(13)
        IF header_bytes[0:8] != EVF_SIGNATURE:
            RAISE InvalidVault("Not a valid E01 file")

        # Parse sections
        volume = None
        chunk_table = []
        hash = None

        WHILE True:
            section = read_section_header(reader)

            MATCH section.type:
                CASE Volume:
                    volume = parse_volume_section(reader, section)
                CASE Table, Table2:
                    chunks = parse_chunk_table(reader, section)
                    chunk_table.extend(chunks)
                CASE Hash:
                    hash = parse_hash_section(reader, section)
                CASE Done:
                    BREAK
                CASE Next:
                    # Multi-file: open next segment
                    reader = open_next_segment(path)

        RETURN E01Vault(reader, volume, chunk_table, hash, E01Cache.new(), 0)

    ACTION read_chunk(chunk_index: uint) -> bytes:
        """Read and decompress a chunk"""

        # Check cache first
        IF self.cache.chunk_index == chunk_index:
            RETURN self.cache.data

        chunk_info = self.chunk_table[chunk_index]
        self.reader.seek(chunk_info.offset)
        raw_data = self.reader.read(chunk_info.compressed_size)

        IF chunk_info.is_compressed:
            data = zlib_decompress(raw_data)
        ELSE:
            data = raw_data

        # Update cache (single-chunk, self-limiting)
        self.cache = E01Cache(chunk_index, data)

        RETURN data
```

### 2.5 AFF4 Vault Cell

```pseudocode
ENUM Aff4Compression:
    None, Deflate, Snappy, Lz4

STRUCTURE Aff4Statement:
    """RDF triple from turtle metadata"""
    subject: string
    predicate: string
    object: string

STRUCTURE Aff4ImageStream:
    """Image stream metadata"""
    urn: string
    size: uint64
    chunk_size: uint32
    chunks_per_segment: uint32
    compression: Aff4Compression
    data_path: string or None
    index_path: string or None

CELL Aff4Vault IMPLEMENTS Vault:
    """AFF4 (Advanced Forensic Format 4) handler"""

    archive: ZipArchive
    volume: Aff4Volume
    stream: Aff4ImageStream
    bevy_index: list[Aff4BevyIndexEntry]
    chunk_cache: LRUCache[uint, bytes]  # LRU-limited cache
    position: uint64

    CONSTRUCTOR open(path: Path):
        archive = ZipArchive.open(path)

        # Parse turtle metadata
        turtle_content = archive.read("information.turtle")
        statements = TurtleParser.parse(turtle_content)

        volume = extract_volume_metadata(statements)
        stream = find_primary_image_stream(volume)

        # Load bevy index
        index_content = archive.read(stream.index_path)
        bevy_index = parse_bevy_index(index_content)

        # Initialize LRU cache with limit
        chunk_cache = LRUCache(max_size=100)

        RETURN Aff4Vault(archive, volume, stream, bevy_index, chunk_cache, 0)

    ACTION read_chunk(chunk_index: uint) -> bytes:
        """Read chunk with LRU caching"""

        # Check cache
        IF chunk_index IN self.chunk_cache:
            RETURN self.chunk_cache.get(chunk_index)

        # Calculate bevy and offset
        bevy_index = chunk_index / self.stream.chunks_per_segment
        chunk_in_bevy = chunk_index % self.stream.chunks_per_segment

        # Read from archive
        bevy_path = format("{}/bevy_{:08d}", self.stream.data_path, bevy_index)
        bevy_data = self.archive.read(bevy_path)

        entry = self.bevy_index[chunk_index]
        compressed = bevy_data[entry.offset : entry.offset + entry.length]

        # Decompress
        MATCH self.stream.compression:
            CASE Deflate:
                data = zlib_decompress(compressed)
            CASE Snappy:
                data = snappy_decompress(compressed)
            CASE Lz4:
                data = lz4_decompress(compressed)
            DEFAULT:
                data = compressed

        # Cache with LRU eviction
        self.chunk_cache.put(chunk_index, data)

        RETURN data
```

---

## 3. Zone Cells

**Real Name:** `totalimage-zones`
**Brand Name:** Zone Cells
**Purpose:** Partition table parsers

### 3.1 MBR Zone Table

```pseudocode
CONST MBR_BOOT_SIGNATURE = 0xAA55
CONST PARTITION_TABLE_OFFSET = 0x1BE
CONST PARTITION_ENTRY_SIZE = 16

ENUM MbrPartitionType:
    Empty = 0x00
    Fat12 = 0x01
    Fat16 = 0x06
    Ntfs = 0x07
    Fat32Lba = 0x0C
    LinuxNative = 0x83
    GptProtective = 0xEE

STRUCTURE MbrPartitionEntry:
    """16-byte partition entry"""
    status: uint8             # 0x80 = bootable
    chs_start: CHSAddress
    type_code: MbrPartitionType
    chs_end: CHSAddress
    lba_start: uint32         # Start sector (LBA)
    sector_count: uint32      # Number of sectors

CELL MbrZoneTable IMPLEMENTS ZoneTable:
    """Master Boot Record parser"""

    zones: list[Zone]
    disk_signature: uint32
    boot_signature: uint16

    CONSTRUCTOR parse(stream: ReadSeekStream, sector_size: uint32):
        stream.seek(0)
        mbr_data = stream.read(512)

        # Verify boot signature
        boot_sig = read_uint16_le(mbr_data, 0x1FE)
        IF boot_sig != MBR_BOOT_SIGNATURE:
            RAISE InvalidZoneTable("Invalid MBR boot signature")

        disk_signature = read_uint32_le(mbr_data, 0x1B8)

        # Parse 4 primary partitions
        zones = []
        FOR i IN range(4):
            offset = PARTITION_TABLE_OFFSET + (i * PARTITION_ENTRY_SIZE)
            entry = MbrPartitionEntry.from_bytes(mbr_data[offset:offset+16])

            IF entry.type_code != Empty AND entry.sector_count > 0:
                zone = Zone(
                    index = i,
                    offset = entry.lba_start * sector_size,
                    length = entry.sector_count * sector_size,
                    zone_type = entry.type_code.name()
                )
                zones.append(zone)

        RETURN MbrZoneTable(zones, disk_signature, boot_sig)

    ACTION is_gpt_protective() -> boolean:
        """Check if this is a protective MBR for GPT"""
        RETURN length(self.zones) == 1 AND
               self.zones[0].zone_type == "GPT Protective"
```

### 3.2 GPT Zone Table

```pseudocode
CONST GPT_SIGNATURE = "EFI PART"
CONST GPT_HEADER_LBA = 1

STRUCTURE GptHeader:
    """GPT header (92 bytes minimum)"""
    signature: bytes[8]       # "EFI PART"
    revision: uint32
    header_size: uint32
    header_crc32: uint32
    reserved: uint32
    my_lba: uint64
    alternate_lba: uint64
    first_usable_lba: uint64
    last_usable_lba: uint64
    disk_guid: bytes[16]
    partition_entries_lba: uint64
    num_partition_entries: uint32
    partition_entry_size: uint32
    partition_entries_crc32: uint32

STRUCTURE GptPartitionEntry:
    """128-byte partition entry"""
    type_guid: bytes[16]
    unique_guid: bytes[16]
    first_lba: uint64
    last_lba: uint64
    attributes: uint64
    name: string              # UTF-16LE, 72 bytes

CELL GptZoneTable IMPLEMENTS ZoneTable:
    """GUID Partition Table parser"""

    zones: list[Zone]
    header: GptHeader

    CONSTRUCTOR parse(stream: ReadSeekStream, sector_size: uint32):
        # First check for protective MBR
        mbr = MbrZoneTable.parse(stream, sector_size)
        IF NOT mbr.is_gpt_protective():
            RAISE InvalidZoneTable("No GPT protective MBR found")

        # Read GPT header at LBA 1
        stream.seek(sector_size)
        header_bytes = stream.read(92)

        IF header_bytes[0:8] != GPT_SIGNATURE:
            RAISE InvalidZoneTable("Invalid GPT signature")

        header = GptHeader.from_bytes(header_bytes)

        # Verify header CRC32
        IF NOT verify_header_crc32(header_bytes, header.header_crc32):
            RAISE InvalidZoneTable("GPT header CRC32 mismatch")

        # Read partition entries
        stream.seek(header.partition_entries_lba * sector_size)
        entries_size = header.num_partition_entries * header.partition_entry_size
        validate_allocation_size(entries_size, MAX_ALLOCATION_SIZE, "GPT entries")
        entries_bytes = stream.read(entries_size)

        # Verify entries CRC32
        IF NOT verify_crc32(entries_bytes, header.partition_entries_crc32):
            RAISE InvalidZoneTable("GPT entries CRC32 mismatch")

        # Parse entries
        zones = []
        FOR i IN range(header.num_partition_entries):
            offset = i * header.partition_entry_size
            entry = GptPartitionEntry.from_bytes(entries_bytes[offset:])

            IF NOT entry.is_unused():
                zone = Zone(
                    index = i,
                    offset = entry.first_lba * sector_size,
                    length = (entry.last_lba - entry.first_lba + 1) * sector_size,
                    zone_type = entry.type_guid.name()
                )
                zone.territory_type = detect_filesystem_hint(entry.type_guid)
                zones.append(zone)

        RETURN GptZoneTable(zones, header)
```

---

## 4. Territory Cells

**Real Name:** `totalimage-territories`
**Brand Name:** Territory Cells
**Purpose:** Filesystem implementations

### 4.1 FAT Territory

```pseudocode
ENUM FatType:
    Fat12, Fat16, Fat32

STRUCTURE BiosParameterBlock:
    """FAT boot sector BPB"""
    bytes_per_sector: uint16
    sectors_per_cluster: uint8
    reserved_sectors: uint16
    num_fats: uint8
    root_entries: uint16      # 0 for FAT32
    total_sectors_16: uint16
    sectors_per_fat_16: uint16
    total_sectors_32: uint32
    # FAT32 extended fields
    sectors_per_fat_32: uint32
    root_cluster: uint32

STRUCTURE FatDirectoryEntry:
    """32-byte directory entry"""
    name: bytes[8]
    ext: bytes[3]
    attributes: uint8
    creation_time: uint16
    creation_date: uint16
    cluster_high: uint16      # FAT32 only
    cluster_low: uint16
    file_size: uint32

CELL FatTerritory IMPLEMENTS Territory:
    """FAT12/16/32 filesystem"""

    bpb: BiosParameterBlock
    fat_table: bytes
    fat_type: FatType
    stream: ReadSeekStream

    CONSTRUCTOR parse(stream: ReadSeekStream):
        # Read boot sector
        stream.seek(0)
        boot_sector = stream.read(512)
        bpb = BiosParameterBlock.from_bytes(boot_sector)

        # Validate sector size
        validate_sector_size(bpb.bytes_per_sector)

        # Determine FAT type by cluster count
        total_sectors = bpb.total_sectors_16 or bpb.total_sectors_32
        data_sectors = total_sectors - bpb.reserved_sectors -
                       (bpb.num_fats * bpb.sectors_per_fat())
        cluster_count = data_sectors / bpb.sectors_per_cluster

        IF cluster_count < 4085:
            fat_type = FatType.Fat12
        ELSE IF cluster_count < 65525:
            fat_type = FatType.Fat16
        ELSE:
            fat_type = FatType.Fat32

        # Load FAT table with size validation
        fat_size = bpb.sectors_per_fat() * bpb.bytes_per_sector
        validate_allocation_size(fat_size, MAX_FAT_TABLE_SIZE, "FAT table")

        stream.seek(bpb.reserved_sectors * bpb.bytes_per_sector)
        fat_table = stream.read(fat_size)

        RETURN FatTerritory(bpb, fat_table, fat_type, stream)

    ACTION read_fat_entry(cluster: uint32) -> uint32:
        """Read FAT entry for cluster chain traversal"""

        MATCH self.fat_type:
            CASE Fat12:
                offset = cluster + (cluster / 2)
                value = read_uint16_le(self.fat_table, offset)
                IF cluster % 2 == 0:
                    RETURN value & 0x0FFF
                ELSE:
                    RETURN value >> 4

            CASE Fat16:
                offset = cluster * 2
                RETURN read_uint16_le(self.fat_table, offset)

            CASE Fat32:
                offset = cluster * 4
                RETURN read_uint32_le(self.fat_table, offset) & 0x0FFFFFFF

    ACTION read_cluster_chain(start_cluster: uint32) -> bytes:
        """Read all clusters in chain with loop protection"""

        data = bytes()
        cluster = start_cluster
        visited = set()
        chain_length = 0

        WHILE NOT is_end_of_chain(cluster) AND chain_length < MAX_CLUSTER_CHAIN_LENGTH:
            IF cluster IN visited:
                RAISE InvalidOperation("Cluster chain loop detected")
            visited.add(cluster)

            offset = self.cluster_offset(cluster)
            self.stream.seek(offset)
            data.extend(self.stream.read(self.cluster_size()))

            cluster = self.read_fat_entry(cluster)
            chain_length += 1

        RETURN data

    ACTION headquarters() -> FatDirectoryCell:
        """Return root directory"""

        IF self.fat_type == Fat32:
            root_data = self.read_cluster_chain(self.bpb.root_cluster)
        ELSE:
            root_offset = self.bpb.reserved_sectors * self.bpb.bytes_per_sector +
                          self.bpb.num_fats * self.bpb.sectors_per_fat() * self.bpb.bytes_per_sector
            root_size = self.bpb.root_entries * 32
            self.stream.seek(root_offset)
            root_data = self.stream.read(root_size)

        RETURN FatDirectoryCell("/", root_data, self)
```

### 4.2 NTFS Territory

```pseudocode
CONST NTFS_SIGNATURE = "NTFS    "
CONST MFT_RECORD_SIZE = 1024
CONST FILE_ATTRIBUTE_DIRECTORY = 0x10000000

STRUCTURE NtfsBootSector:
    """NTFS boot sector"""
    oem_id: bytes[8]          # "NTFS    "
    bytes_per_sector: uint16
    sectors_per_cluster: uint8
    mft_lcn: uint64           # MFT logical cluster number
    mft_mirror_lcn: uint64
    clusters_per_mft_record: int8
    clusters_per_index_block: int8
    volume_serial: uint64

CELL NtfsTerritory IMPLEMENTS Territory:
    """NTFS filesystem (read-only via ntfs crate)"""

    ntfs: NtfsHandle          # ntfs crate handle
    reader: ReadSeekStream
    volume_info: NtfsVolumeInfo

    CONSTRUCTOR parse(reader: ReadSeekStream):
        # Parse boot sector
        reader.seek(0)
        boot_sector = reader.read(512)

        IF boot_sector[3:11] != NTFS_SIGNATURE:
            RAISE InvalidTerritory("Not an NTFS filesystem")

        # Initialize ntfs crate
        ntfs = Ntfs.new(reader)

        # Get volume information
        volume_info = NtfsVolumeInfo(
            label = get_volume_label(ntfs, reader),
            major_version = ntfs.major_version(),
            minor_version = ntfs.minor_version(),
            total_size = ntfs.total_clusters() * ntfs.cluster_size(),
            cluster_size = ntfs.cluster_size(),
            sector_size = ntfs.sector_size()
        )

        RETURN NtfsTerritory(ntfs, reader, volume_info)

    ACTION headquarters() -> NtfsDirectoryCell:
        """Return root directory"""
        root = self.ntfs.root_directory(self.reader)
        RETURN NtfsDirectoryCell("\\", root, self)

    ACTION extract_file(path: string) -> bytes:
        """Extract file using ntfs crate"""

        # Navigate to file
        file = self.ntfs.file_by_path(self.reader, path)

        # Get $DATA attribute
        data_attr = file.data(self.reader, "")

        # Read all data
        size = data_attr.len()
        validate_allocation_size(size, MAX_FILE_EXTRACT_SIZE, "NTFS file")

        buffer = bytes(size)
        data_attr.read(self.reader, buffer)

        RETURN buffer

CELL NtfsDirectoryCell IMPLEMENTS DirectoryCell:
    """NTFS directory entry"""

    name: string
    index: NtfsIndex
    territory: NtfsTerritory

    ACTION list_occupants() -> list[OccupantInfo]:
        """List directory contents"""

        entries = []
        entry_count = 0

        FOR entry IN self.index.iter(self.territory.reader):
            IF entry_count >= MAX_DIRECTORY_ENTRIES:
                BREAK

            file_name = entry.file_name()
            IF file_name.is_dot_or_dotdot():
                CONTINUE

            info = OccupantInfo(
                name = file_name.name(),
                is_directory = entry.is_directory(),
                size = entry.data_size(),
                created = entry.creation_time(),
                modified = entry.modification_time(),
                accessed = entry.access_time(),
                attributes = entry.file_attributes()
            )
            entries.append(info)
            entry_count += 1

        RETURN entries
```

### 4.3 ISO Territory

```pseudocode
CONST ISO_SECTOR_SIZE = 2048
CONST VOLUME_DESCRIPTOR_LBA = 16

ENUM VolumeDescriptorType:
    Boot = 0
    Primary = 1
    Supplementary = 2
    Terminator = 255

STRUCTURE IsoDirectoryRecord:
    """ISO 9660 directory record"""
    record_length: uint8
    extent_location: uint32   # LBA of file data
    data_length: uint32
    recording_date: bytes[7]
    file_flags: uint8
    name: string

CELL IsoTerritory IMPLEMENTS Territory:
    """ISO 9660 filesystem"""

    primary_descriptor: PrimaryVolumeDescriptor
    root_directory: IsoDirectoryRecord
    stream: ReadSeekStream

    CONSTRUCTOR parse(stream: ReadSeekStream):
        # Scan volume descriptors starting at LBA 16
        stream.seek(VOLUME_DESCRIPTOR_LBA * ISO_SECTOR_SIZE)

        primary_descriptor = None

        WHILE True:
            descriptor_bytes = stream.read(ISO_SECTOR_SIZE)
            type_code = descriptor_bytes[0]

            IF type_code == VolumeDescriptorType.Primary:
                primary_descriptor = PrimaryVolumeDescriptor.from_bytes(descriptor_bytes)
            ELSE IF type_code == VolumeDescriptorType.Terminator:
                BREAK

        IF primary_descriptor IS None:
            RAISE InvalidTerritory("No primary volume descriptor")

        root_directory = primary_descriptor.root_directory_record

        RETURN IsoTerritory(primary_descriptor, root_directory, stream)

    ACTION read_directory(directory: IsoDirectoryRecord) -> list[IsoDirectoryRecord]:
        """Read directory entries"""

        self.stream.seek(directory.extent_location * ISO_SECTOR_SIZE)
        data = self.stream.read(directory.data_length)

        entries = []
        offset = 0

        WHILE offset < length(data):
            record_length = data[offset]
            IF record_length == 0:
                # Padding to sector boundary
                offset = ((offset / ISO_SECTOR_SIZE) + 1) * ISO_SECTOR_SIZE
                CONTINUE

            entry = IsoDirectoryRecord.from_bytes(data[offset:offset+record_length])
            IF NOT entry.is_dot_or_dotdot():
                entries.append(entry)

            offset += record_length

        RETURN entries
```

---

## 5. Pipeline Cells

**Real Name:** `totalimage-pipeline`
**Brand Name:** Pipeline Cells
**Purpose:** I/O abstractions

### 5.1 Memory-Mapped Pipeline (Direct Action)

```pseudocode
CELL MmapPipeline:
    """Memory-mapped file access for fast reads"""

    mmap: MemoryMap
    position: uint64

    CONSTRUCTOR open(path: Path):
        file = open(path, "rb")
        size = file_size(path)

        IF size > MAX_MMAP_SIZE:
            RAISE InvalidOperation("File too large for mmap")

        mmap = create_memory_map(file, size)
        RETURN MmapPipeline(mmap, 0)

    ACTION read(count: uint) -> bytes:
        """Direct memory read"""
        end = min(self.position + count, length(self.mmap))
        data = self.mmap[self.position:end]
        self.position = end
        RETURN data

    ACTION seek(offset: int64, whence: SeekFrom):
        """Update position"""
        MATCH whence:
            CASE Start:
                self.position = offset
            CASE Current:
                self.position += offset
            CASE End:
                self.position = length(self.mmap) + offset
```

### 5.2 Partial Pipeline (Window View)

```pseudocode
CELL PartialPipeline:
    """Window into a larger stream (for partition access)"""

    inner: ReadSeekStream
    start: uint64             # Absolute start offset
    length: uint64            # Window size
    position: uint64          # Current position within window

    CONSTRUCTOR new(inner: ReadSeekStream, start: uint64, length: uint64):
        inner.seek(start, Start)
        RETURN PartialPipeline(inner, start, length, 0)

    ACTION read(count: uint) -> bytes:
        """Read within window bounds"""
        remaining = self.length - self.position
        to_read = min(count, remaining)
        data = self.inner.read(to_read)
        self.position += length(data)
        RETURN data

    ACTION seek(offset: int64, whence: SeekFrom):
        """Seek within window bounds"""
        new_pos = MATCH whence:
            CASE Start: offset
            CASE Current: self.position + offset
            CASE End: self.length + offset

        IF new_pos < 0 OR new_pos > self.length:
            RAISE InvalidOperation("Seek outside window bounds")

        self.position = new_pos
        self.inner.seek(self.start + new_pos, Start)
```

---

## 6. MCP Collective

**Real Name:** `totalimage-mcp`
**Brand Name:** MCP Collective
**Purpose:** Model Context Protocol server for AI integration

### 6.1 MCP Protocol

```pseudocode
CONST MCP_VERSION = "2024-11-05"

ENUM MCPErrorCode:
    ParseError = -32700
    InvalidRequest = -32600
    MethodNotFound = -32601
    InvalidParams = -32602
    InternalError = -32603

STRUCTURE MCPRequest:
    jsonrpc: string           # "2.0"
    id: RequestId
    method: string            # "initialize", "tools/list", "tools/call"
    params: object or None

STRUCTURE MCPResponse:
    jsonrpc: string
    id: RequestId
    result: object or None
    error: MCPError or None

STRUCTURE ToolDefinition:
    name: string
    description: string
    inputSchema: JsonSchema

STRUCTURE ToolResult:
    content: list[Content]
    isError: boolean or None

ACTION handle_request(request: MCPRequest) -> MCPResponse:
    """Route MCP request to handler"""

    MATCH request.method:
        CASE "initialize":
            RETURN handle_initialize(request)
        CASE "tools/list":
            RETURN handle_list_tools(request)
        CASE "tools/call":
            RETURN handle_call_tool(request)
        DEFAULT:
            RETURN error_response(MCPErrorCode.MethodNotFound)
```

### 6.2 Tool Trait

```pseudocode
TRAIT Tool:
    """Interface for MCP tools"""

    ACTION name() -> string
    ACTION description() -> string
    ACTION input_schema() -> JsonSchema
    ASYNC ACTION execute(args: object or None) -> ToolResult

    ACTION definition() -> ToolDefinition:
        RETURN ToolDefinition(
            name = self.name(),
            description = self.description(),
            inputSchema = self.input_schema()
        )
```

### 6.3 Analyze Disk Image Tool

```pseudocode
CELL AnalyzeDiskImageTool IMPLEMENTS Tool:
    """Analyze forensic disk image structure"""

    cache: ToolCache

    ACTION name():
        RETURN "analyze_disk_image"

    ACTION description():
        RETURN "Analyze a forensic disk image file to identify format, " +
               "partitions, and filesystems"

    ACTION input_schema():
        RETURN {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to disk image"},
                "cache": {"type": "boolean", "default": true},
                "deep_scan": {"type": "boolean", "default": false}
            },
            "required": ["path"]
        }

    ASYNC ACTION execute(args: object) -> ToolResult:
        path = args.path
        use_cache = args.cache ?? true

        # Check cache
        cache_key = "analyze:" + hash(path)
        IF use_cache:
            cached = self.cache.get(cache_key)
            IF cached:
                RETURN ToolResult(content=[TextContent(cached)])

        # Open vault
        vault = open_vault(path, VaultConfig.default())

        result = {
            "vault": {
                "path": path,
                "type": vault.identify(),
                "size_bytes": vault.length()
            },
            "zones": [],
            "filesystems": []
        }

        # Parse partition table
        TRY:
            zone_table = parse_zone_table(vault.content())
            result["partition_table"] = {
                "type": zone_table.identify(),
                "zone_count": length(zone_table.enumerate_zones())
            }

            FOR zone IN zone_table.enumerate_zones():
                zone_info = {
                    "index": zone.index,
                    "offset": zone.offset,
                    "length": zone.length,
                    "type": zone.zone_type
                }

                # Try to identify filesystem
                TRY:
                    partition = PartialPipeline.new(vault.content(), zone.offset, zone.length)
                    territory = detect_territory(partition)
                    zone_info["filesystem"] = {
                        "type": territory.identify(),
                        "label": territory.banner()
                    }
                CATCH:
                    zone_info["filesystem"] = None

                result["zones"].append(zone_info)
        CATCH:
            result["partition_table"] = None

        # Cache result
        IF use_cache:
            self.cache.set(cache_key, result, ttl=3600)

        RETURN ToolResult(content=[TextContent(json_encode(result))])
```

### 6.4 Tool Cache

```pseudocode
CELL ToolCache:
    """Persistent cache using redb"""

    db: RedbDatabase
    tool_name: string
    version: string

    CONSTRUCTOR new(cache_path: Path, tool_name: string, version: string):
        db = RedbDatabase.create(cache_path)

        # Initialize table
        txn = db.begin_write()
        txn.open_table("tool_results")
        txn.commit()

        RETURN ToolCache(db, tool_name, version)

    ACTION get(key: string) -> object or None:
        """Get cached value if not expired"""

        txn = self.db.begin_read()
        table = txn.open_table("tool_results")

        entry = table.get(key)
        IF entry IS None:
            RETURN None

        cached = deserialize(entry)

        # Check expiration
        IF cached.is_expired(TTL_SECONDS):
            # Expired - remove asynchronously
            self.remove(key)
            RETURN None

        RETURN cached.data

    ACTION set(key: string, value: object, ttl: uint or None):
        """Store value with TTL"""

        entry = CacheEntry(
            data = value,
            created_at = current_timestamp(),
            tool = self.tool_name,
            version = self.version
        )

        txn = self.db.begin_write()
        table = txn.open_table("tool_results")
        table.insert(key, serialize(entry))
        txn.commit()
```

---

## 7. CLI Front

**Real Name:** `totalimage-cli`
**Brand Name:** CLI Front
**Purpose:** Command-line interface

### 7.1 Main Entry Point

```pseudocode
ACTION main():
    """CLI entry point (Ignition)"""

    args = parse_command_line()

    IF length(args) < 2:
        print_usage()
        exit(1)

    command = args[1]

    MATCH command:
        CASE "info":
            cmd_info(args[2])
        CASE "zones":
            cmd_zones(args[2])
        CASE "list":
            zone_index = parse_zone_arg(args)
            cmd_list(args[2], zone_index)
        CASE "extract":
            zone_index = parse_zone_arg(args)
            output_path = parse_output_arg(args)
            cmd_extract(args[2], args[3], zone_index, output_path)
        DEFAULT:
            print_usage()
            exit(1)

ACTION cmd_info(image_path: string):
    """Display vault information"""

    vault = open_vault(image_path, VaultConfig.default())

    print("Vault Type: " + vault.identify())
    print("Size: " + format_bytes(vault.length()))

    TRY:
        zone_table = parse_zone_table(vault.content())
        print("Partition Table: " + zone_table.identify())
        print("Partitions: " + string(length(zone_table.enumerate_zones())))
    CATCH:
        print("Partition Table: None detected")

ACTION cmd_extract(image_path: string, file_path: string,
                   zone_index: uint or None, output_path: string or None):
    """Extract file from image"""

    vault = open_vault(image_path, VaultConfig.default())

    # Get partition
    IF zone_index IS NOT None:
        zone_table = parse_zone_table(vault.content())
        zone = zone_table.get_zone(zone_index)
        partition = PartialPipeline.new(vault.content(), zone.offset, zone.length)
    ELSE:
        partition = vault.content()

    # Parse filesystem
    territory = detect_territory(partition)

    # Extract file
    data = territory.extract_file(file_path)

    # Write output
    out_path = output_path or basename(file_path)
    write_file(out_path, data)

    print("Extracted " + format_bytes(length(data)) + " to " + out_path)
```

---

## 8. Web Front

**Real Name:** `totalimage-web`
**Brand Name:** Web Front
**Purpose:** REST API server

### 8.1 API Handlers

```pseudocode
ROUTE GET /health:
    """Health check endpoint"""
    RETURN {"status": "healthy", "version": VERSION}

ROUTE GET /api/vault/info:
    """Get vault information"""

    path = query_param("path")

    vault = open_vault(path, VaultConfig.default())

    response = {
        "path": path,
        "vault_type": vault.identify(),
        "size_bytes": vault.length()
    }

    TRY:
        zone_table = parse_zone_table(vault.content())
        response["partition_table"] = {
            "type": zone_table.identify(),
            "zone_count": length(zone_table.enumerate_zones())
        }
    CATCH:
        response["partition_table"] = None

    RETURN response

ROUTE GET /api/vault/zones:
    """List partitions/zones"""

    path = query_param("path")

    vault = open_vault(path, VaultConfig.default())
    zone_table = parse_zone_table(vault.content())

    zones = []
    FOR zone IN zone_table.enumerate_zones():
        zones.append({
            "index": zone.index,
            "offset": zone.offset,
            "length": zone.length,
            "zone_type": zone.zone_type
        })

    RETURN {
        "path": path,
        "zone_count": length(zones),
        "zones": zones
    }
```

---

## 9. Fire Marshal

**Real Name:** `fire-marshal`
**Brand Name:** Fire Marshal
**Purpose:** Tool orchestration framework

### 9.1 Tool Registry

```pseudocode
STRUCTURE ToolInfo:
    """Tool registration information"""
    name: string
    version: string
    description: string
    tools: list[ToolMethod]
    executor: ToolExecutor

ENUM ToolExecutor:
    Http { url: string, auth: AuthConfig or None }
    Process { executable: Path, args: list[string] }
    Native { module: string }

CELL ToolRegistry:
    """Registry of available tools"""

    tools: RwLock[map[string, RegisteredTool]]

    ACTION register(info: ToolInfo):
        """Register a new tool"""

        lock = self.tools.write_lock()

        IF info.name IN lock:
            RAISE ToolAlreadyRegistered(info.name)

        registered = RegisteredTool(
            info = info,
            registered_at = current_timestamp(),
            healthy = True
        )

        lock[info.name] = registered

    ACTION call_tool(name: string, method: string, args: object) -> object:
        """Execute a tool method"""

        lock = self.tools.read_lock()

        IF name NOT IN lock:
            RAISE ToolNotFound(name)

        tool = lock[name]

        MATCH tool.info.executor:
            CASE Http(url, auth):
                RETURN http_call(url + "/" + method, args, auth)
            CASE Process(executable, env_args):
                RETURN process_call(executable, method, args)
            CASE Native(module):
                RETURN native_call(module, method, args)
```

### 9.2 Platform Database

```pseudocode
CELL PlatformDatabase:
    """Shared cache database using redb"""

    db: Mutex[RedbDatabase]
    config: DatabaseConfig

    ACTION get(key: string) -> object or None:
        """Get cached value, handling expiration without deadlock"""

        # Check if expired in a scoped lock
        expired = False
        {
            db = self.db.lock()
            txn = db.begin_read()
            table = txn.open_table("cache")

            entry = table.get(key)
            IF entry IS NOT None:
                cached = deserialize(entry)
                IF cached.is_expired(self.config.ttl_seconds):
                    expired = True
                ELSE:
                    RETURN cached.data
        }  # Lock released here

        # Remove expired entry outside the lock
        IF expired:
            self.remove(key)

        RETURN None

    ACTION set(key: string, value: object, tool: string, version: string):
        """Store value with metadata"""

        entry = CacheEntry(
            data = value,
            created_at = current_timestamp(),
            tool = tool,
            version = version
        )

        db = self.db.lock()
        txn = db.begin_write()
        table = txn.open_table("cache")
        table.insert(key, serialize(entry))
        txn.commit()
```

### 9.3 Fire Marshal Server

```pseudocode
CELL FireMarshal:
    """Tool orchestration server"""

    config: FireMarshalConfig
    registry: ToolRegistry
    database: PlatformDatabase

    CONSTRUCTOR new(config: FireMarshalConfig):
        database = PlatformDatabase.new(config.database_path, config.to_db_config())
        registry = ToolRegistry.new()

        RETURN FireMarshal(config, registry, database)

    ASYNC ACTION serve():
        """Start HTTP server"""

        router = Router()
            .route("/health", GET, health_handler)
            .route("/tools/register", POST, register_handler)
            .route("/tools/list", GET, list_handler)
            .route("/tools/call", POST, call_handler)
            .route("/stats", GET, stats_handler)

        server = HttpServer.bind(("0.0.0.0", self.config.port))
        server.serve(router)
```

---

## 10. Acquire Collective

**Real Name:** `totalimage-acquire`
**Brand Name:** Acquire Collective
**Purpose:** Disk image acquisition

### 10.1 Raw Acquisition

```pseudocode
CELL RawAcquirer:
    """Acquire raw disk images"""

    options: AcquireOptions

    ACTION acquire_from_device(device_path: Path, output_path: Path,
                               callback: ProgressCallback or None):
        """Create raw image from device"""

        source = open(device_path, "rb")
        dest = create(output_path, "wb")

        total_size = device_size(device_path)
        bytes_processed = 0
        hasher = Hasher.new(self.options.hash_algorithm) IF self.options.hash_algorithm

        buffer = bytes(self.options.buffer_size)

        WHILE True:
            read_count = source.read(buffer)
            IF read_count == 0:
                BREAK

            dest.write(buffer[0:read_count])

            IF hasher:
                hasher.update(buffer[0:read_count])

            bytes_processed += read_count

            IF callback:
                callback.on_progress(AcquireProgress(
                    bytes_processed = bytes_processed,
                    total_bytes = total_size,
                    percentage = (bytes_processed * 100) / total_size
                ))

        result = AcquireResult(
            path = output_path,
            size = bytes_processed,
            hash = hasher.finalize() IF hasher ELSE None
        )

        IF self.options.verify_on_complete AND result.hash:
            verify_hash(output_path, result.hash)

        RETURN result
```

### 10.2 VHD Creation

```pseudocode
CELL VhdCreator:
    """Create VHD images"""

    options: VhdOptions

    ACTION create(source_path: Path, output_path: Path,
                  callback: ProgressCallback or None) -> VhdCreationResult:
        """Create VHD from source"""

        source = open(source_path, "rb")
        source_size = file_size(source_path)

        # Round up to sector boundary
        virtual_size = round_up(source_size, 512)

        dest = create(output_path, "wb")

        MATCH self.options.output_type:
            CASE Fixed:
                create_fixed_vhd(source, dest, virtual_size, callback)
            CASE Dynamic:
                create_dynamic_vhd(source, dest, virtual_size,
                                   self.options.block_size, callback)

        # Write VHD footer
        footer = VhdFooter(
            cookie = "conectix",
            disk_type = self.options.output_type,
            original_size = virtual_size,
            current_size = virtual_size,
            uuid = generate_uuid()
        )
        dest.write(footer.to_bytes())

        RETURN VhdCreationResult(
            path = output_path,
            physical_size = file_size(output_path),
            virtual_size = virtual_size
        )
```

---

## Cross-Reference Index

| Real Name | Brand Name | Pseudocode Section |
|-----------|------------|-------------------|
| `totalimage-core` | Arsenal | 1. Core Arsenal |
| `Vault` trait | Vault Interface | 1.1 |
| `ZoneTable` trait | Zone Interface | 1.2 |
| `Territory` trait | Territory Interface | 1.3 |
| `DirectoryCell` trait | Directory Interface | 1.4 |
| `totalimage-vaults` | Vault Cells | 2. Vault Cells |
| `RawVault` | Raw Vault Cell | 2.2 |
| `VhdVault` | VHD Vault Cell | 2.3 |
| `E01Vault` | E01 Vault Cell | 2.4 |
| `Aff4Vault` | AFF4 Vault Cell | 2.5 |
| `totalimage-zones` | Zone Cells | 3. Zone Cells |
| `MbrZoneTable` | MBR Zone Table | 3.1 |
| `GptZoneTable` | GPT Zone Table | 3.2 |
| `totalimage-territories` | Territory Cells | 4. Territory Cells |
| `FatTerritory` | FAT Territory | 4.1 |
| `NtfsTerritory` | NTFS Territory | 4.2 |
| `IsoTerritory` | ISO Territory | 4.3 |
| `totalimage-pipeline` | Pipeline Cells | 5. Pipeline Cells |
| `MmapPipeline` | Direct Action | 5.1 |
| `PartialPipeline` | Window View | 5.2 |
| `totalimage-mcp` | MCP Collective | 6. MCP Collective |
| `ToolCache` | Tool Cache | 6.4 |
| `totalimage-cli` | CLI Front | 7. CLI Front |
| `totalimage-web` | Web Front | 8. Web Front |
| `fire-marshal` | Fire Marshal | 9. Fire Marshal |
| `ToolRegistry` | Tool Registry | 9.1 |
| `PlatformDatabase` | Platform Database | 9.2 |
| `totalimage-acquire` | Acquire Collective | 10. Acquire Collective |

---

**Document Status:** Complete
**Next Update:** As codebase evolves
