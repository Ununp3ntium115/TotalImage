//! TotalImage CLI - Command-line liberation tool
//!
//! A tool for inspecting disk images, partition tables, and file systems.

use std::env;
use std::path::Path;
use std::process;
use totalimage_core::{Result, Vault, ZoneTable};
use totalimage_pipeline::PartialPipeline;
use totalimage_vaults::{RawVault, VaultConfig};
use totalimage_zones::{GptZoneTable, MbrZoneTable};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "info" => {
            if args.len() < 3 {
                eprintln!("Usage: {} info <image_file>", args[0]);
                process::exit(1);
            }
            if let Err(e) = cmd_info(&args[2]) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        "zones" => {
            if args.len() < 3 {
                eprintln!("Usage: {} zones <image_file>", args[0]);
                process::exit(1);
            }
            if let Err(e) = cmd_zones(&args[2]) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        "list" => {
            if args.len() < 3 {
                eprintln!("Usage: {} list <image_file> [--zone INDEX]", args[0]);
                process::exit(1);
            }
            let zone_index = parse_zone_arg(&args);
            if let Err(e) = cmd_list(&args[2], zone_index) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        "extract" => {
            if args.len() < 4 {
                eprintln!("Usage: {} extract <image_file> <file_path> [--zone INDEX] [--output PATH]", args[0]);
                process::exit(1);
            }
            let zone_index = parse_zone_arg(&args);
            let output_path = parse_output_arg(&args);
            if let Err(e) = cmd_extract(&args[2], &args[3], zone_index, output_path.as_deref()) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        "--help" | "-h" | "help" => {
            print_usage(&args[0]);
        }
        "--version" | "-v" | "version" => {
            println!("TotalImage CLI v{}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage(&args[0]);
            process::exit(1);
        }
    }
}

fn print_usage(program: &str) {
    println!("TotalImage - Total Liberation Project");
    println!();
    println!("USAGE:");
    println!("    {} <COMMAND> [OPTIONS]", program);
    println!();
    println!("COMMANDS:");
    println!("    info <image>                           Display vault information");
    println!("    zones <image>                          List partition zones");
    println!("    list <image> [--zone INDEX]            List files in filesystem");
    println!("    extract <image> <file> [OPTIONS]       Extract a file");
    println!("    help                                   Print this help message");
    println!("    version                                Print version");
    println!();
    println!("EXTRACT OPTIONS:");
    println!("    --zone INDEX     Partition zone index (default: 0)");
    println!("    --output PATH    Output file path (default: stdout)");
    println!();
    println!("EXAMPLES:");
    println!("    {} info disk.img", program);
    println!("    {} zones floppy.img", program);
    println!("    {} list disk.img --zone 0", program);
    println!("    {} extract disk.img AUTOEXEC.BAT --output autoexec.bat", program);
}

fn cmd_info(image_path: &str) -> Result<()> {
    let path = Path::new(image_path);
    let mut vault = RawVault::open(path, VaultConfig::default())?;

    println!("=== Vault Information ===");
    println!("Path:   {}", image_path);
    println!("Type:   {}", vault.identify());
    println!("Size:   {} bytes ({:.2} MB)", vault.length(), vault.length() as f64 / 1_048_576.0);
    println!();

    // Try to detect sector size (assume 512 for now)
    let sector_size = 512;

    // Try MBR first
    if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
        println!("=== Partition Table ===");
        println!("Type:        {}", mbr.identify());
        println!("Disk Sig:    0x{:08X}", mbr.disk_signature());
        println!("Boot Sig:    0x{:04X}", mbr.boot_signature());
        println!("Partitions:  {}", mbr.enumerate_zones().len());

        if mbr.is_gpt_protective() {
            println!();
            println!("Note: This disk has a GPT protective MBR.");
            println!("      Use GPT zone table for full information.");
        }
    } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
        println!("=== Partition Table ===");
        println!("Type:        {}", gpt.identify());
        println!("Partitions:  {}", gpt.enumerate_zones().len());
        println!("Usable LBA:  {}", gpt.usable_lba_count());
    } else {
        println!("No recognized partition table found.");
    }

    Ok(())
}

fn cmd_zones(image_path: &str) -> Result<()> {
    let path = Path::new(image_path);
    let mut vault = RawVault::open(path, VaultConfig::default())?;

    println!("=== Partition Zones ===");
    println!();

    let sector_size = 512;

    // Try MBR first
    if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
        println!("Partition table: {}", mbr.identify());
        println!();

        if mbr.enumerate_zones().is_empty() {
            println!("No partitions found.");
        } else {
            println!("{:<5} {:<15} {:<15} {:<20}", "Index", "Offset", "Size", "Type");
            println!("{}", "-".repeat(60));

            for zone in mbr.enumerate_zones() {
                println!(
                    "{:<5} {:<15} {:<15} {:<20}",
                    zone.index,
                    format_bytes(zone.offset),
                    format_bytes(zone.length),
                    zone.zone_type
                );
            }

            // Try to parse FAT from first partition
            if let Some(first_zone) = mbr.enumerate_zones().first() {
                println!();
                println!("=== First Partition Analysis ===");

                let mut partial = PartialPipeline::new(
                    vault.content(),
                    first_zone.offset,
                    first_zone.length,
                )?;

                if let Ok(fat) = totalimage_territories::FatTerritory::parse(&mut partial) {
                    use totalimage_core::Territory;

                    println!("Filesystem:  {}", fat.identify());
                    println!("Domain:      {}", format_bytes(fat.domain_size()));
                    println!("Block size:  {}", format_bytes(fat.block_size()));
                    println!("Hierarchical: {}", if fat.hierarchical() { "Yes" } else { "No" });
                }
            }
        }
    } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
        println!("Partition table: {}", gpt.identify());
        println!();

        if gpt.enumerate_zones().is_empty() {
            println!("No partitions found.");
        } else {
            println!("{:<5} {:<15} {:<15} {:<40}", "Index", "Offset", "Size", "Type");
            println!("{}", "-".repeat(80));

            for zone in gpt.enumerate_zones() {
                println!(
                    "{:<5} {:<15} {:<15} {:<40}",
                    zone.index,
                    format_bytes(zone.offset),
                    format_bytes(zone.length),
                    zone.zone_type
                );
            }
        }
    } else {
        println!("No recognized partition table found.");
        println!("This may be an unpartitioned volume.");
    }

    Ok(())
}

fn parse_zone_arg(args: &[String]) -> usize {
    for i in 0..args.len() - 1 {
        if args[i] == "--zone" {
            return args[i + 1].parse().unwrap_or(0);
        }
    }
    0
}

fn parse_output_arg(args: &[String]) -> Option<String> {
    for i in 0..args.len() - 1 {
        if args[i] == "--output" {
            return Some(args[i + 1].clone());
        }
    }
    None
}

fn cmd_list(image_path: &str, zone_index: usize) -> Result<()> {
    use totalimage_core::Territory;

    let path = Path::new(image_path);
    let mut vault = RawVault::open(path, VaultConfig::default())?;
    let sector_size = 512;

    // Try to parse partition table
    let zone = if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
        let zones = mbr.enumerate_zones();
        if zone_index >= zones.len() {
            eprintln!("Error: Zone index {} out of range (0-{})", zone_index, zones.len() - 1);
            process::exit(1);
        }
        zones[zone_index].clone()
    } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
        let zones = gpt.enumerate_zones();
        if zone_index >= zones.len() {
            eprintln!("Error: Zone index {} out of range (0-{})", zone_index, zones.len() - 1);
            process::exit(1);
        }
        zones[zone_index].clone()
    } else {
        // Unpartitioned disk - use entire disk as zone 0
        if zone_index != 0 {
            eprintln!("Error: No partition table found. Use zone 0 for unpartitioned disk.");
            process::exit(1);
        }
        use totalimage_core::Zone;
        Zone {
            index: 0,
            offset: 0,
            length: vault.length(),
            zone_type: "Unpartitioned".to_string(),
            territory_type: None,
        }
    };

    // Create partial pipeline for the zone
    let mut partial = PartialPipeline::new(vault.content(), zone.offset, zone.length)?;

    // Try to parse FAT filesystem
    if let Ok(fat) = totalimage_territories::FatTerritory::parse(&mut partial) {
        println!("=== Files in {} (Zone {}) ===", image_path, zone_index);
        println!("Filesystem: {}", fat.identify());
        println!();

        // List directory contents
        let occupants = fat.list_root_directory(&mut partial)?;

        if occupants.is_empty() {
            println!("No files found.");
        } else {
            println!("{:<30} {:<10} {:<15}", "Name", "Type", "Size");
            println!("{}", "-".repeat(60));

            for occupant in occupants {
                let file_type = if occupant.is_directory { "Dir" } else { "File" };
                println!(
                    "{:<30} {:<10} {:<15}",
                    occupant.name,
                    file_type,
                    format_bytes(occupant.size)
                );
            }
        }
    } else {
        eprintln!("Error: Unable to parse filesystem in zone {}. Only FAT filesystems are currently supported.", zone_index);
        process::exit(1);
    }

    Ok(())
}

fn cmd_extract(image_path: &str, file_path: &str, zone_index: usize, output_path: Option<&str>) -> Result<()> {
    use std::io::Write;

    let path = Path::new(image_path);
    let mut vault = RawVault::open(path, VaultConfig::default())?;
    let sector_size = 512;

    // Try to parse partition table
    let zone = if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
        let zones = mbr.enumerate_zones();
        if zone_index >= zones.len() {
            eprintln!("Error: Zone index {} out of range (0-{})", zone_index, zones.len() - 1);
            process::exit(1);
        }
        zones[zone_index].clone()
    } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
        let zones = gpt.enumerate_zones();
        if zone_index >= zones.len() {
            eprintln!("Error: Zone index {} out of range (0-{})", zone_index, zones.len() - 1);
            process::exit(1);
        }
        zones[zone_index].clone()
    } else {
        // Unpartitioned disk - use entire disk as zone 0
        if zone_index != 0 {
            eprintln!("Error: No partition table found. Use zone 0 for unpartitioned disk.");
            process::exit(1);
        }
        use totalimage_core::Zone;
        Zone {
            index: 0,
            offset: 0,
            length: vault.length(),
            zone_type: "Unpartitioned".to_string(),
            territory_type: None,
        }
    };

    // Create partial pipeline for the zone
    let mut partial = PartialPipeline::new(vault.content(), zone.offset, zone.length)?;

    // Try to parse FAT filesystem
    if let Ok(fat) = totalimage_territories::FatTerritory::parse(&mut partial) {
        // Find the file
        let entry = fat.find_file_in_root(&mut partial, file_path)?;

        // Read file data
        let data = fat.read_file_data(&mut partial, &entry)?;

        // Write to output
        if let Some(output) = output_path {
            std::fs::write(output, &data)?;
            println!("Extracted {} ({} bytes) to {}", file_path, data.len(), output);
        } else {
            // Write to stdout
            std::io::stdout().write_all(&data)?;
        }
    } else {
        eprintln!("Error: Unable to parse filesystem in zone {}. Only FAT filesystems are currently supported.", zone_index);
        process::exit(1);
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1_048_576 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1_073_741_824 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    }
}
