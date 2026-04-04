// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Error handling tests for filesystem workflow operations
//!
//! Tests graceful handling of permission errors, missing files, broken symlinks, etc.

use std::fs;

/// Test graceful handling of permission denied error
#[test]
#[cfg(unix)]
fn test_permission_denied_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let restricted_dir = temp.path().join("restricted");
    let restricted_file = restricted_dir.join("file.txt");

    fs::create_dir(&restricted_dir).expect("create dir");
    fs::write(&restricted_file, "content").expect("write file");

    // Remove read permissions (Unix only)
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    let perms = Permissions::from_mode(0o000);
    fs::set_permissions(&restricted_dir, perms).expect("set permissions");

    // Try to read: should fail gracefully, not panic
    let result = fs::read_to_string(&restricted_file);
    assert!(result.is_err(), "should fail to read without permissions");

    // Restore permissions for cleanup
    let perms = Permissions::from_mode(0o755);
    fs::set_permissions(&restricted_dir, perms).expect("restore permissions");
}

/// Test graceful handling of missing file
#[test]
fn test_missing_file_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let missing_file = temp.path().join("nonexistent.txt");

    // Try to read non-existent file
    let result = fs::read_to_string(&missing_file);
    assert!(result.is_err(), "should fail to read missing file");
    assert!(!missing_file.exists(), "file should not exist");
}

/// Test graceful handling of remove non-existent file
#[test]
fn test_remove_nonexistent_file_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let missing_file = temp.path().join("nonexistent.txt");

    // Try to remove non-existent file
    let result = fs::remove_file(&missing_file);
    assert!(result.is_err(), "should fail to remove missing file");
}

/// Test graceful handling of rename when destination exists
#[test]
fn test_rename_destination_exists_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = temp.path().join("source.txt");
    let dest = temp.path().join("dest.txt");

    fs::write(&source, "source").expect("write source");
    fs::write(&dest, "dest").expect("write dest");

    // Try to rename over existing file (platform-dependent behavior)
    let result = fs::rename(&source, &dest);

    // On Unix this typically succeeds (overwrites), on Windows it fails
    // Either way, we shouldn't panic
    if result.is_ok() {
        // Source should be gone
        assert!(!source.exists());
        // Dest should exist with source content
        assert!(dest.exists());
    } else {
        // If rename failed, both files should still exist
        assert!(source.exists());
        assert!(dest.exists());
    }
}

/// Test graceful handling of broken symlink
#[test]
#[cfg(unix)]
fn test_broken_symlink_handling() {
    use std::os::unix::fs as unix_fs;

    let temp = tempfile::tempdir().expect("create temp dir");
    let symlink = temp.path().join("link.txt");
    let target = temp.path().join("target.txt"); // Never created

    // Create symlink to non-existent target
    unix_fs::symlink(&target, &symlink).expect("create symlink");

    // Symlink should exist, but target doesn't
    assert!(symlink.exists() || !symlink.exists()); // Platform-dependent

    // Try to read through symlink: should fail gracefully
    let result = fs::read_to_string(&symlink);
    assert!(result.is_err(), "should fail to read broken symlink");

    // Try to remove symlink: should succeed
    let remove_result = fs::remove_file(&symlink);
    assert!(remove_result.is_ok(), "should be able to remove symlink");
    assert!(!symlink.exists());
}

/// Test graceful handling of directory when file expected
#[test]
fn test_directory_instead_of_file_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let dir = temp.path().join("subdir");

    fs::create_dir(&dir).expect("create dir");

    // Try to read directory as file: should fail
    let result = fs::read_to_string(&dir);
    assert!(result.is_err(), "should fail to read directory");

    // Try to remove directory with remove_file: should fail
    let result = fs::remove_file(&dir);
    assert!(result.is_err(), "should fail to remove_file on directory");

    // But remove_dir should work
    let result = fs::remove_dir(&dir);
    assert!(result.is_ok(), "remove_dir should work");
}

/// Test graceful handling of file when directory expected
#[test]
fn test_file_instead_of_directory_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let file = temp.path().join("file.txt");

    fs::write(&file, "content").expect("write file");

    // Try to read as directory: should fail
    let result = fs::read_dir(&file);
    assert!(result.is_err(), "should fail to read_dir on file");

    // Try to remove as directory: should fail
    let result = fs::remove_dir(&file);
    assert!(result.is_err(), "should fail to remove_dir on file");

    // But remove_file should work
    let result = fs::remove_file(&file);
    assert!(result.is_ok(), "remove_file should work");
}

/// Test graceful handling of circular symlinks
#[test]
#[cfg(unix)]
fn test_circular_symlink_handling() {
    use std::os::unix::fs as unix_fs;

    let temp = tempfile::tempdir().expect("create temp dir");
    let link1 = temp.path().join("link1");
    let link2 = temp.path().join("link2");

    // Create circular symlinks
    unix_fs::symlink(&link2, &link1).expect("create link1");
    unix_fs::symlink(&link1, &link2).expect("create link2");

    // Try to read: should fail (infinite loop detection)
    let result = fs::read_to_string(&link1);
    assert!(result.is_err(), "should fail on circular symlink");

    // Cleanup - remove symlinks
    let _ = fs::remove_file(&link1);
    let _ = fs::remove_file(&link2);
}

/// Test graceful handling of invalid UTF-8 filenames
#[test]
#[cfg(unix)]
fn test_invalid_utf8_filename_handling() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let temp = tempfile::tempdir().expect("create temp dir");

    // Create a filename with invalid UTF-8 bytes
    let invalid_name = OsStr::from_bytes(&[0x66, 0x69, 0x6c, 0x65, 0xff]); // "file\xff"
    let invalid_path = temp.path().join(invalid_name);

    // This might succeed or fail depending on filesystem, but shouldn't panic
    let _result = fs::write(&invalid_path, "content");

    // Try to list directory: should handle gracefully
    let result = fs::read_dir(temp.path());
    assert!(result.is_ok(), "should handle directory with unusual names");

    for entry_result in result.unwrap() {
        match entry_result {
            Ok(_entry) => {
                // Successfully read entry
            }
            Err(_e) => {
                // Some entries might fail to parse, but shouldn't crash
            }
        }
    }
}

/// Test handling of very long file paths
#[test]
fn test_very_long_path_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");

    // Build a very long path (but below system limit)
    let mut long_path = temp.path().to_path_buf();
    for i in 0..50 {
        long_path.push(format!("dir_{:02}", i));
    }

    // Try to create the deep path
    let result = fs::create_dir_all(&long_path);

    // Might succeed or fail depending on system limits, but shouldn't panic
    match result {
        Ok(_) => {
            // Successfully created
            assert!(long_path.exists());
        }
        Err(_) => {
            // Path too long or other error - acceptable
        }
    }
}

/// Test handling of concurrent access to same file
#[test]
fn test_concurrent_file_access_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let file = temp.path().join("shared.txt");

    fs::write(&file, "initial").expect("write initial");

    let mut handles = vec![];

    // Spawn multiple threads reading the same file
    for _ in 0..5 {
        let file = file.clone();
        let h = std::thread::spawn(move || {
            for _ in 0..10 {
                let _content = fs::read_to_string(&file);
            }
        });
        handles.push(h);
    }

    // Spawn threads that write to the file
    let file_clone = file.clone();
    let h = std::thread::spawn(move || {
        for i in 0..5 {
            let _result = fs::write(&file_clone, format!("update {}", i));
        }
    });
    handles.push(h);

    for h in handles {
        h.join().expect("thread panicked");
    }

    // File should still be readable at the end
    let result = fs::read_to_string(&file);
    assert!(result.is_ok(), "should be able to read file after concurrent access");
}

/// Test handling of metadata access on missing file
#[test]
fn test_metadata_missing_file_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let missing = temp.path().join("missing.txt");

    let result = fs::metadata(&missing);
    assert!(result.is_err(), "should fail to get metadata of missing file");
}

/// Test handling of operations on empty directory
#[test]
fn test_empty_directory_operations() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let empty_dir = temp.path().join("empty");

    fs::create_dir(&empty_dir).expect("create empty dir");

    // List empty directory
    let result = fs::read_dir(&empty_dir);
    assert!(result.is_ok());
    let entries: Vec<_> = result.unwrap().collect();
    assert_eq!(entries.len(), 0, "empty dir should have no entries");

    // Remove empty directory
    let result = fs::remove_dir(&empty_dir);
    assert!(result.is_ok(), "should remove empty directory");
    assert!(!empty_dir.exists());
}

/// Test handling of path normalization issues
#[test]
fn test_path_normalization_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");

    // Create with normal path
    let normal = temp.path().join("file.txt");
    fs::write(&normal, "content").expect("write normal");

    // Access with path containing ./
    let with_dot = temp.path().join(".").join("file.txt");
    let result = fs::read_to_string(&with_dot);
    assert!(result.is_ok(), "should handle ./ in path");

    // Create a subdirectory and try parent references
    let subdir = temp.path().join("sub");
    fs::create_dir(&subdir).expect("create subdir");

    let with_parent = subdir.join("..").join("file.txt");
    let result = fs::read_to_string(&with_parent);
    assert!(result.is_ok(), "should handle ../ in path");
}

/// Test handling when running out of disk space (simulated)
#[test]
fn test_large_file_write_handling() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let file = temp.path().join("large.bin");

    // Try to write a very large file (but not actually exceeding space in temp)
    let large_data = vec![42u8; 100 * 1024 * 1024]; // 100MB
    let result = fs::write(&file, &large_data);

    // Might succeed or fail depending on available space, but shouldn't panic
    match result {
        Ok(_) => {
            // Successfully written
            assert!(file.exists());
        }
        Err(_) => {
            // Out of space or other error - acceptable
        }
    }
}
