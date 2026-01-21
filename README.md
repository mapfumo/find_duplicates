# find_duplicates

<p align="center">
  <img src="logo.svg" alt="find_duplicates logo" width="150">
</p>

A fast, memory-efficient command-line tool for finding and managing duplicate files.

## Features

- **Scalable**: Handles large directories (100GB+) efficiently
- **Memory-efficient**: Uses chunked file hashing - works with any file size
- **Smart filtering**: Groups files by size first, only hashes potential duplicates
- **Interactive**: Review duplicates, select files to delete, verify results
- **Safe**: Confirmation prompts before deletion, option to keep at least one copy

## Installation

```bash
cargo install --path .
```

## Usage

```bash
find_duplicates <directory>
```

### Example

```bash
find_duplicates ~/Music
```

Output:
```
Scanning /home/user/Music...
Found 14250 files, analyzing for duplicates...

============================================================
DUPLICATE FILE SCAN RESULTS
============================================================

Found 49 duplicate group(s), 79 duplicate file(s)
Space that can be recovered: 86.81 MB

------------------------------------------------------------

Group 1 - 6.51 KB (4 files)
  /home/user/Music/Album1/cover.jpg
  /home/user/Music/Album1/Folder.jpg
  /home/user/Music/Album2/cover.jpg
  /home/user/Music/Album2/Folder.jpg

...
```

### Interactive Menu

After scanning, you can:

1. **Review a specific group** - Select which files to delete from a duplicate group
2. **Delete all duplicates** - Automatically remove all duplicates, keeping the first file in each group
3. **Rescan directory** - Re-run the scan to verify changes
4. **Quit** - Exit with optional verification scan

## How It Works

1. **Scan**: Recursively walks the directory collecting file paths and sizes
2. **Filter by size**: Groups files by size - files with unique sizes can't be duplicates
3. **Hash duplicates**: Computes MD5 hashes only for files that share sizes
4. **Group by hash**: Files with identical hashes are duplicates
5. **Interactive management**: Review and delete duplicates safely

## Performance

The two-pass approach (size filtering, then hashing) significantly reduces work:

- If 10,000 files exist but only 500 share sizes with other files, only 500 files are hashed
- Chunked hashing (8KB buffer) keeps memory usage constant regardless of file size

## License

MIT
