//! File scanning and hashing module.
//!
//! This module provides functionality for recursively scanning directories,
//! collecting file metadata, and computing content hashes for duplicate detection.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use md5::{Digest, Md5};
use walkdir::WalkDir;

/// Buffer size for chunked file reading (8 KB).
const HASH_BUFFER_SIZE: usize = 8192;

/// Metadata about a file used for duplicate detection.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// File size in bytes.
    pub size: u64,
}

/// Computes the MD5 hash of a file using chunked reading.
///
/// This function reads the file in chunks to maintain constant memory usage
/// regardless of file size, making it suitable for large files.
///
/// # Arguments
///
/// * `path` - Path to the file to hash.
///
/// # Returns
///
/// The MD5 hash as a lowercase hexadecimal string, or an IO error.
///
/// # Example
///
/// ```ignore
/// let hash = hash_file(Path::new("/path/to/file.txt"))?;
/// println!("MD5: {}", hash);
/// ```
pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Md5::new();
    let mut buffer = [0u8; HASH_BUFFER_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Recursively scans a directory and collects file information.
///
/// Walks the directory tree, collecting path and size for each regular file.
/// Symbolic links are followed. Files that cannot be accessed are silently skipped.
///
/// # Arguments
///
/// * `dir` - Root directory to scan.
///
/// # Returns
///
/// A vector of [`FileInfo`] for all accessible files, or an IO error.
pub fn scan_directory(dir: &Path) -> io::Result<Vec<FileInfo>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            if let Ok(metadata) = fs::metadata(entry.path()) {
                files.push(FileInfo {
                    path: entry.path().to_path_buf(),
                    size: metadata.len(),
                });
            }
        }
    }

    Ok(files)
}

/// Groups files by size, filtering to only potential duplicates.
///
/// Files with unique sizes cannot be duplicates, so this serves as a fast
/// first-pass filter before the more expensive hash computation.
///
/// # Arguments
///
/// * `files` - Vector of files to group.
///
/// # Returns
///
/// A map from file size to files of that size, containing only sizes
/// with two or more files.
pub fn group_by_size(files: Vec<FileInfo>) -> HashMap<u64, Vec<FileInfo>> {
    let mut size_groups: HashMap<u64, Vec<FileInfo>> = HashMap::new();

    for file in files {
        size_groups.entry(file.size).or_default().push(file);
    }

    size_groups.retain(|_, group| group.len() > 1);
    size_groups
}

/// Groups files by content hash, identifying actual duplicates.
///
/// Computes MD5 hashes for each file and groups them. Files that fail
/// to hash (e.g., permission denied) are silently skipped.
///
/// # Arguments
///
/// * `files` - Vector of files to hash and group.
///
/// # Returns
///
/// A map from hash to files with that hash, containing only hashes
/// with two or more files (actual duplicates).
pub fn group_by_hash(files: Vec<FileInfo>) -> HashMap<String, Vec<FileInfo>> {
    let mut hash_groups: HashMap<String, Vec<FileInfo>> = HashMap::new();

    for file in files {
        if let Ok(hash) = hash_file(&file.path) {
            hash_groups.entry(hash).or_default().push(file);
        }
    }

    hash_groups.retain(|_, group| group.len() > 1);
    hash_groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_hash_file_consistent() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");

        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap();
        drop(file);

        let hash1 = hash_file(&file_path).unwrap();
        let hash2 = hash_file(&file_path).unwrap();

        assert_eq!(hash1, hash2);
        // Known MD5 hash for "hello world"
        assert_eq!(hash1, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    }

    #[test]
    fn test_hash_file_different_content() {
        let dir = TempDir::new().unwrap();

        let file1_path = dir.path().join("file1.txt");
        let file2_path = dir.path().join("file2.txt");

        File::create(&file1_path)
            .unwrap()
            .write_all(b"content a")
            .unwrap();
        File::create(&file2_path)
            .unwrap()
            .write_all(b"content b")
            .unwrap();

        let hash1 = hash_file(&file1_path).unwrap();
        let hash2 = hash_file(&file2_path).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_scan_directory() {
        let dir = TempDir::new().unwrap();

        File::create(dir.path().join("file1.txt"))
            .unwrap()
            .write_all(b"test")
            .unwrap();
        File::create(dir.path().join("file2.txt"))
            .unwrap()
            .write_all(b"test")
            .unwrap();

        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        File::create(subdir.join("file3.txt"))
            .unwrap()
            .write_all(b"test")
            .unwrap();

        let files = scan_directory(dir.path()).unwrap();

        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_group_by_size() {
        let files = vec![
            FileInfo {
                path: PathBuf::from("a.txt"),
                size: 100,
            },
            FileInfo {
                path: PathBuf::from("b.txt"),
                size: 100,
            },
            FileInfo {
                path: PathBuf::from("c.txt"),
                size: 200,
            },
        ];

        let groups = group_by_size(files);

        // Only size 100 has duplicates
        assert_eq!(groups.len(), 1);
        assert!(groups.contains_key(&100));
        assert_eq!(groups[&100].len(), 2);
    }

    #[test]
    fn test_group_by_hash() {
        let dir = TempDir::new().unwrap();

        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");
        let file3 = dir.path().join("file3.txt");

        File::create(&file1)
            .unwrap()
            .write_all(b"same content")
            .unwrap();
        File::create(&file2)
            .unwrap()
            .write_all(b"same content")
            .unwrap();
        File::create(&file3)
            .unwrap()
            .write_all(b"different")
            .unwrap();

        let files = vec![
            FileInfo {
                path: file1,
                size: 12,
            },
            FileInfo {
                path: file2,
                size: 12,
            },
            FileInfo {
                path: file3,
                size: 9,
            },
        ];

        let groups = group_by_hash(files);

        // Only files with "same content" are duplicates
        assert_eq!(groups.len(), 1);
        let (_, duplicates) = groups.iter().next().unwrap();
        assert_eq!(duplicates.len(), 2);
    }
}
