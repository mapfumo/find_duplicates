//! Interactive user interface module.
//!
//! Provides terminal-based user interaction for reviewing duplicate files,
//! selecting files to delete, and confirming destructive actions.

use std::fs;
use std::io;

use dialoguer::{Confirm, MultiSelect, Select};

use crate::duplicates::{DuplicateGroup, DuplicateStats};

/// Actions available from the main menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Review a specific duplicate group by index.
    ReviewGroup(usize),
    /// Delete all duplicates, keeping the first file in each group.
    DeleteAllDuplicates,
    /// Rescan the directory for duplicates.
    Rescan,
    /// Exit the program.
    Quit,
}

/// Displays the scan results summary and all duplicate groups.
///
/// Shows aggregate statistics (total groups, files, reclaimable space)
/// followed by a detailed listing of each duplicate group.
pub fn display_summary(groups: &[DuplicateGroup], stats: &DuplicateStats) {
    println!("\n{}", "=".repeat(60));
    println!("DUPLICATE FILE SCAN RESULTS");
    println!("{}", "=".repeat(60));

    if groups.is_empty() {
        println!("\nNo duplicate files found.");
        return;
    }

    println!(
        "\nFound {} duplicate group(s), {} duplicate file(s)",
        stats.total_groups, stats.total_duplicate_files
    );
    println!(
        "Space that can be recovered: {}",
        DuplicateStats::format_bytes(stats.total_wasted_bytes)
    );

    println!("\n{}", "-".repeat(60));
    for (i, group) in groups.iter().enumerate() {
        println!(
            "\nGroup {} - {} ({} files)",
            i + 1,
            DuplicateStats::format_bytes(group.size),
            group.paths.len()
        );
        for path in &group.paths {
            println!("  {}", path.display());
        }
    }
    println!("\n{}", "-".repeat(60));
}

/// Displays the main menu and returns the user's selected action.
///
/// # Arguments
///
/// * `group_count` - Number of duplicate groups available to review.
///
/// # Returns
///
/// The selected [`Action`], or an IO error if the terminal is unavailable.
pub fn show_main_menu(group_count: usize) -> io::Result<Action> {
    if group_count == 0 {
        return Ok(Action::Quit);
    }

    let options = vec![
        format!("Review a specific group (1-{})", group_count),
        "Delete all duplicates (keep first of each group)".to_string(),
        "Rescan directory".to_string(),
        "Quit".to_string(),
    ];

    let selection = Select::new()
        .with_prompt("What would you like to do?")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    match selection {
        0 => {
            let group_options: Vec<String> =
                (1..=group_count).map(|i| format!("Group {}", i)).collect();

            let group_idx = Select::new()
                .with_prompt("Select a group to review")
                .items(&group_options)
                .default(0)
                .interact()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            Ok(Action::ReviewGroup(group_idx))
        }
        1 => Ok(Action::DeleteAllDuplicates),
        2 => Ok(Action::Rescan),
        _ => Ok(Action::Quit),
    }
}

/// Presents a duplicate group for review and file selection.
///
/// Displays all files in the group and allows the user to select which
/// files to delete using a multi-select interface. By default, all files
/// except the first are pre-selected for deletion.
///
/// # Arguments
///
/// * `group` - The duplicate group to review.
/// * `group_num` - Display number for the group (1-indexed).
///
/// # Returns
///
/// Indices of files selected for deletion, or an empty vector if cancelled.
pub fn review_group(group: &DuplicateGroup, group_num: usize) -> io::Result<Vec<usize>> {
    println!(
        "\nGroup {} - {} each",
        group_num,
        DuplicateStats::format_bytes(group.size)
    );

    let path_options: Vec<String> = group
        .paths
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i == 0 {
                format!("{} (will be kept)", p.display())
            } else {
                p.display().to_string()
            }
        })
        .collect();

    println!("\nSelect files to DELETE (the first file is kept by default):");
    println!("Use SPACE to select/deselect, ENTER to confirm\n");

    let defaults: Vec<bool> = (0..group.paths.len()).map(|i| i > 0).collect();

    let selections = MultiSelect::new()
        .with_prompt("Files to delete")
        .items(&path_options)
        .defaults(&defaults)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Safety check: warn if deleting all copies
    if selections.len() == group.paths.len() {
        println!("\nWarning: You've selected ALL files for deletion!");
        let proceed = Confirm::new()
            .with_prompt("This will delete all copies. Are you sure?")
            .default(false)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        if !proceed {
            return Ok(vec![]);
        }
    }

    Ok(selections)
}

/// Deletes files at the specified indices within a duplicate group.
///
/// # Arguments
///
/// * `group` - The duplicate group containing the files.
/// * `indices` - Indices of files to delete.
///
/// # Returns
///
/// Total bytes deleted, or an IO error.
pub fn delete_files(group: &DuplicateGroup, indices: &[usize]) -> io::Result<u64> {
    let mut deleted_bytes = 0u64;

    for &idx in indices {
        if let Some(path) = group.paths.get(idx) {
            match fs::remove_file(path) {
                Ok(()) => {
                    println!("  Deleted: {}", path.display());
                    deleted_bytes += group.size;
                }
                Err(e) => {
                    eprintln!("  Error deleting {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(deleted_bytes)
}

/// Deletes all duplicate files, keeping the first file in each group.
///
/// Prompts for confirmation before proceeding. For each group, deletes
/// all files except the first one.
///
/// # Arguments
///
/// * `groups` - All duplicate groups to process.
///
/// # Returns
///
/// Total bytes deleted, or 0 if cancelled.
pub fn delete_all_duplicates(groups: &[DuplicateGroup]) -> io::Result<u64> {
    let total_to_delete: usize = groups.iter().map(|g| g.paths.len() - 1).sum();

    println!(
        "\nThis will delete {} file(s), keeping the first file from each group.",
        total_to_delete
    );

    let proceed = Confirm::new()
        .with_prompt("Are you sure you want to proceed?")
        .default(false)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if !proceed {
        println!("Cancelled.");
        return Ok(0);
    }

    let mut total_deleted = 0u64;

    for group in groups {
        let indices: Vec<usize> = (1..group.paths.len()).collect();
        total_deleted += delete_files(group, &indices)?;
    }

    println!(
        "\nDeleted {} file(s), recovered {}",
        total_to_delete,
        DuplicateStats::format_bytes(total_deleted)
    );

    Ok(total_deleted)
}

/// Prompts the user to rescan the directory for verification.
///
/// # Returns
///
/// `true` if the user wants to rescan, `false` otherwise.
pub fn prompt_rescan() -> io::Result<bool> {
    Confirm::new()
        .with_prompt("Would you like to rescan to verify no duplicates remain?")
        .default(true)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
