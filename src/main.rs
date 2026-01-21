//! find_duplicates - A CLI tool for finding and managing duplicate files.
//!
//! This tool recursively scans a directory, identifies files with identical content
//! using MD5 hashing, and provides an interactive interface for reviewing and
//! deleting duplicates.

mod duplicates;
mod interactive;
mod scanner;

use std::path::PathBuf;
use std::process;

use clap::Parser;

use duplicates::{find_duplicates, DuplicateStats};
use interactive::{
    delete_all_duplicates, delete_files, display_summary, prompt_rescan, review_group,
    show_main_menu, Action,
};
use scanner::scan_directory;

/// Command-line arguments.
#[derive(Parser, Debug)]
#[command(name = "find_duplicates")]
#[command(version)]
#[command(about = "Find and manage duplicate files in a directory")]
struct Args {
    /// Directory to scan for duplicates
    #[arg(value_name = "DIRECTORY")]
    directory: PathBuf,
}

/// Scans a directory for duplicates and displays the results.
///
/// This function handles the complete scan workflow: directory traversal,
/// duplicate detection, and summary display.
fn scan_and_display(dir: &PathBuf) -> Vec<duplicates::DuplicateGroup> {
    println!("Scanning {}...", dir.display());

    let files = match scan_directory(dir) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error scanning directory: {}", e);
            process::exit(1);
        }
    };

    println!("Found {} files, analyzing for duplicates...", files.len());

    let groups = find_duplicates(files);
    let stats = DuplicateStats::from_groups(&groups);

    display_summary(&groups, &stats);

    groups
}

/// Application entry point.
///
/// Parses command-line arguments, performs initial scan, and runs the
/// interactive main loop for duplicate management.
fn main() {
    let args = Args::parse();

    if !args.directory.is_dir() {
        eprintln!(
            "Error: '{}' is not a valid directory",
            args.directory.display()
        );
        process::exit(1);
    }

    let mut groups = scan_and_display(&args.directory);

    // Main interaction loop
    loop {
        if groups.is_empty() {
            println!("\nNo duplicates to manage. Exiting.");
            break;
        }

        let action = match show_main_menu(groups.len()) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        };

        match action {
            Action::ReviewGroup(idx) => {
                if let Some(group) = groups.get(idx) {
                    match review_group(group, idx + 1) {
                        Ok(to_delete) => {
                            if !to_delete.is_empty() {
                                if let Err(e) = delete_files(group, &to_delete) {
                                    eprintln!("Error deleting files: {}", e);
                                }
                                groups = scan_and_display(&args.directory);
                            }
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
            }
            Action::DeleteAllDuplicates => {
                if let Err(e) = delete_all_duplicates(&groups) {
                    eprintln!("Error deleting files: {}", e);
                }
                groups = scan_and_display(&args.directory);
            }
            Action::Rescan => {
                groups = scan_and_display(&args.directory);
            }
            Action::Quit => {
                match prompt_rescan() {
                    Ok(true) => {
                        groups = scan_and_display(&args.directory);
                        if groups.is_empty() {
                            println!("\nVerified: No duplicate files remain.");
                            break;
                        }
                    }
                    Ok(false) => {
                        println!("Goodbye!");
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
    }
}
