//! Duplicate detection and statistics module.
//!
//! This module provides data structures for representing groups of duplicate files
//! and functions for computing statistics about disk space usage.

use std::path::PathBuf;

use crate::scanner::{self, FileInfo};

/// A group of files with identical content.
///
/// Each group contains two or more files that have the same MD5 hash,
/// indicating they are duplicates of each other.
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    /// MD5 hash shared by all files in this group.
    #[allow(dead_code)]
    pub hash: String,

    /// Size in bytes of each file (all files in group have same size).
    pub size: u64,

    /// Paths to all duplicate files.
    pub paths: Vec<PathBuf>,
}

impl DuplicateGroup {
    /// Calculates the total wasted disk space from duplicates.
    ///
    /// Returns the space that could be recovered by keeping only one copy,
    /// i.e., `size * (count - 1)`.
    pub fn wasted_space(&self) -> u64 {
        if self.paths.len() > 1 {
            self.size * (self.paths.len() as u64 - 1)
        } else {
            0
        }
    }

    /// Returns the number of duplicate files (excluding the original).
    ///
    /// For a group of 3 identical files, this returns 2 (the files that
    /// could be deleted while keeping one copy).
    pub fn duplicate_count(&self) -> usize {
        self.paths.len().saturating_sub(1)
    }
}

/// Aggregate statistics for all duplicate groups.
#[derive(Debug, Clone)]
pub struct DuplicateStats {
    /// Number of duplicate groups found.
    pub total_groups: usize,

    /// Total number of duplicate files (excluding one original per group).
    pub total_duplicate_files: usize,

    /// Total bytes that can be recovered by removing duplicates.
    pub total_wasted_bytes: u64,
}

impl DuplicateStats {
    /// Computes statistics from a collection of duplicate groups.
    pub fn from_groups(groups: &[DuplicateGroup]) -> Self {
        Self {
            total_groups: groups.len(),
            total_duplicate_files: groups.iter().map(|g| g.duplicate_count()).sum(),
            total_wasted_bytes: groups.iter().map(|g| g.wasted_space()).sum(),
        }
    }

    /// Formats a byte count as a human-readable string.
    ///
    /// Uses binary units (KB = 1024 bytes, MB = 1024 KB, etc.).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// assert_eq!(DuplicateStats::format_bytes(1536), "1.50 KB");
    /// assert_eq!(DuplicateStats::format_bytes(1048576), "1.00 MB");
    /// ```
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

/// Finds all duplicate files from a list of file information.
///
/// Uses a two-pass algorithm for efficiency:
/// 1. Groups files by size (files with unique sizes can't be duplicates)
/// 2. Hashes only files that share sizes with others
///
/// # Arguments
///
/// * `files` - Vector of file information from [`scanner::scan_directory`].
///
/// # Returns
///
/// A vector of [`DuplicateGroup`]s, each containing files with identical content.
pub fn find_duplicates(files: Vec<FileInfo>) -> Vec<DuplicateGroup> {
    // First pass: group by size (fast filter)
    let size_groups = scanner::group_by_size(files);

    // Flatten all potential duplicates for hashing
    let potential_duplicates: Vec<FileInfo> = size_groups.into_values().flatten().collect();

    // Second pass: group by hash (actual duplicates)
    let hash_groups = scanner::group_by_hash(potential_duplicates);

    // Convert to DuplicateGroup structs
    hash_groups
        .into_iter()
        .map(|(hash, files)| {
            let size = files.first().map(|f| f.size).unwrap_or(0);
            let paths = files.into_iter().map(|f| f.path).collect();
            DuplicateGroup { hash, size, paths }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duplicate_group_wasted_space() {
        let group = DuplicateGroup {
            hash: "abc".to_string(),
            size: 1000,
            paths: vec![
                PathBuf::from("a.txt"),
                PathBuf::from("b.txt"),
                PathBuf::from("c.txt"),
            ],
        };

        // 3 files, 1000 bytes each, 2 are duplicates
        assert_eq!(group.wasted_space(), 2000);
        assert_eq!(group.duplicate_count(), 2);
    }

    #[test]
    fn test_duplicate_group_single_file() {
        let group = DuplicateGroup {
            hash: "abc".to_string(),
            size: 1000,
            paths: vec![PathBuf::from("a.txt")],
        };

        assert_eq!(group.wasted_space(), 0);
        assert_eq!(group.duplicate_count(), 0);
    }

    #[test]
    fn test_duplicate_stats_from_groups() {
        let groups = vec![
            DuplicateGroup {
                hash: "abc".to_string(),
                size: 1000,
                paths: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
            },
            DuplicateGroup {
                hash: "def".to_string(),
                size: 500,
                paths: vec![
                    PathBuf::from("c.txt"),
                    PathBuf::from("d.txt"),
                    PathBuf::from("e.txt"),
                ],
            },
        ];

        let stats = DuplicateStats::from_groups(&groups);

        assert_eq!(stats.total_groups, 2);
        assert_eq!(stats.total_duplicate_files, 3); // 1 + 2
        assert_eq!(stats.total_wasted_bytes, 2000); // 1000 + 500*2
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(DuplicateStats::format_bytes(500), "500 bytes");
        assert_eq!(DuplicateStats::format_bytes(1024), "1.00 KB");
        assert_eq!(DuplicateStats::format_bytes(1536), "1.50 KB");
        assert_eq!(DuplicateStats::format_bytes(1048576), "1.00 MB");
        assert_eq!(DuplicateStats::format_bytes(1073741824), "1.00 GB");
    }
}
