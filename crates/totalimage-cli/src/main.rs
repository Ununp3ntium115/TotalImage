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
    println!("    info <image>     Display vault information");
    println!("    zones <image>    List partition zones");
    println!("    help             Print this help message");
    println!("    version          Print version");
    println!();
    println!("EXAMPLES:");
    println!("    {} info disk.img", program);
    println!("    {} zones floppy.img", program);
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
