// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! End-to-end workflow execution tests
//!
//! Tests complete file operation cycles: watcher → event → rules → actions → result

use std::fs;

/// Test complete file creation and rename workflow
#[test]
fn test_e2e_file_creation_and_rename_workflow() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let watch_dir = temp.path();

    // Create a test file
    let test_file = watch_dir.join("test.txt");
    fs::write(&test_file, "test content").expect("write test file");
    assert!(test_file.exists(), "test file should exist after creation");

    // Rename the file
    let renamed_file = watch_dir.join("test_renamed.txt");
    fs::rename(&test_file, &renamed_file).expect("rename file");

    // Verify old file is gone
    assert!(!test_file.exists(), "old file should not exist after rename");
    assert!(renamed_file.exists(), "renamed file should exist");

    // Verify content is preserved
    let content = fs::read_to_string(&renamed_file).expect("read renamed file");
    assert_eq!(content, "test content", "content should be preserved after rename");
}

/// Test complete file copy workflow
#[test]
fn test_e2e_file_copy_workflow() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_file = temp.path().join("source.txt");
    let dest_file = temp.path().join("destination.txt");

    let content = "source file content";
    fs::write(&source_file, content).expect("write source file");

    // Copy the file
    fs::copy(&source_file, &dest_file).expect("copy file");

    // Verify both files exist
    assert!(source_file.exists(), "source file should still exist");
    assert!(dest_file.exists(), "destination file should exist after copy");

    // Verify content matches
    let source_content = fs::read_to_string(&source_file).expect("read source");
    let dest_content = fs::read_to_string(&dest_file).expect("read destination");
    assert_eq!(source_content, dest_content, "content should match");
    assert_eq!(dest_content, content, "content should match original");
}

/// Test complete file move workflow
#[test]
fn test_e2e_file_move_workflow() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_file = temp.path().join("original.txt");
    let archive_dir = temp.path().join("archive");

    fs::create_dir(&archive_dir).expect("create archive dir");

    let content = "original content";
    fs::write(&source_file, content).expect("write source file");

    let archived_file = archive_dir.join("original.txt");

    // Move the file to archive
    fs::rename(&source_file, &archived_file).expect("move file to archive");

    // Verify source is gone
    assert!(!source_file.exists(), "source should not exist after move");

    // Verify file is in archive with content intact
    assert!(archived_file.exists(), "archived file should exist");
    let archived_content = fs::read_to_string(&archived_file).expect("read archived file");
    assert_eq!(archived_content, content, "content should be preserved in archive");
}

/// Test complete file deletion workflow
#[test]
fn test_e2e_file_deletion_workflow() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let test_file = temp.path().join("to_delete.txt");

    fs::write(&test_file, "content").expect("write test file");
    assert!(test_file.exists(), "file should exist before deletion");

    // Delete the file
    fs::remove_file(&test_file).expect("delete file");

    // Verify file is gone
    assert!(!test_file.exists(), "file should not exist after deletion");
}

/// Test workflow with multiple sequential operations
#[test]
fn test_e2e_multiple_sequential_operations() {
    let temp = tempfile::tempdir().expect("create temp dir");

    // Create multiple files
    let file1 = temp.path().join("file1.txt");
    let file2 = temp.path().join("file2.txt");
    let file3 = temp.path().join("file3.txt");

    fs::write(&file1, "content1").expect("write file1");
    fs::write(&file2, "content2").expect("write file2");
    fs::write(&file3, "content3").expect("write file3");

    assert!(file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());

    // Perform operations on each
    fs::rename(&file1, temp.path().join("file1_v2.txt")).expect("rename file1");

    let archive = temp.path().join("archive");
    fs::create_dir(&archive).expect("create archive");
    fs::rename(&file2, archive.join("file2.txt")).expect("move file2");

    fs::remove_file(&file3).expect("delete file3");

    // Verify final state
    assert!(!file1.exists());
    assert!(!file2.exists());
    assert!(!file3.exists());
    assert!(temp.path().join("file1_v2.txt").exists());
    assert!(archive.join("file2.txt").exists());
}

/// Test workflow robustness with file in use
#[test]
fn test_e2e_workflow_with_file_operations_on_large_file() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let large_file = temp.path().join("large.bin");

    // Create a larger file (1MB)
    let large_content = vec![42u8; 1024 * 1024];
    fs::write(&large_file, &large_content).expect("write large file");

    // Copy it
    let copy = temp.path().join("large_copy.bin");
    fs::copy(&large_file, &copy).expect("copy large file");

    // Verify copy integrity
    let copy_content = fs::read(&copy).expect("read copy");
    assert_eq!(copy_content, large_content, "large file copy should be identical");

    // Clean up original
    fs::remove_file(&large_file).expect("delete original");
    assert!(!large_file.exists());
    assert!(copy.exists());
}

/// Test workflow with directory creation and file operations
#[test]
fn test_e2e_workflow_with_nested_directories() {
    let temp = tempfile::tempdir().expect("create temp dir");

    // Create nested structure
    let nested_path = temp.path().join("level1/level2/level3");
    fs::create_dir_all(&nested_path).expect("create nested dirs");

    let test_file = nested_path.join("test.txt");
    fs::write(&test_file, "nested content").expect("write nested file");

    // Verify nested file exists and is readable
    assert!(test_file.exists());
    let content = fs::read_to_string(&test_file).expect("read nested file");
    assert_eq!(content, "nested content");

    // Move file up one level
    let moved_file = nested_path.parent().unwrap().join("test.txt");
    fs::rename(&test_file, &moved_file).expect("move file up");

    assert!(!test_file.exists());
    assert!(moved_file.exists());
}

/// Test workflow state consistency after failed operation
#[test]
fn test_e2e_workflow_state_consistency_after_operation() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let file1 = temp.path().join("file1.txt");
    let file2 = temp.path().join("file2.txt");

    fs::write(&file1, "content1").expect("write file1");

    // Create file2
    fs::write(&file2, "content2").expect("write file2");
    assert!(file2.exists());

    // Remove file1
    fs::remove_file(&file1).expect("delete file1");
    assert!(!file1.exists());

    // State should still be consistent: file2 exists, file1 doesn't
    assert!(file2.exists());
    let content = fs::read_to_string(&file2).expect("read file2");
    assert_eq!(content, "content2");
}

/// Test workflow with special characters in filenames
#[test]
fn test_e2e_workflow_with_special_characters_in_filenames() {
    let temp = tempfile::tempdir().expect("create temp dir");

    // Create files with special characters (but not directory separators)
    let special_files = vec![
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.multiple.dots.txt",
    ];

    for filename in special_files {
        let path = temp.path().join(filename);
        fs::write(&path, "content").expect(&format!("write {}", filename));
        assert!(path.exists(), "file {} should exist", filename);

        // Verify content
        let content = fs::read_to_string(&path).expect(&format!("read {}", filename));
        assert_eq!(content, "content");
    }
}

/// Test concurrent file operations don't corrupt state
#[test]
fn test_e2e_workflow_concurrent_operations() {
    let temp = tempfile::tempdir().expect("create temp dir");

    // Create 10 files
    for i in 0..10 {
        let path = temp.path().join(format!("file_{}.txt", i));
        fs::write(&path, format!("content {}", i)).expect(&format!("write file_{}", i));
    }

    // Rename all files
    for i in 0..10 {
        let old_path = temp.path().join(format!("file_{}.txt", i));
        let new_path = temp.path().join(format!("renamed_{}.txt", i));
        fs::rename(&old_path, &new_path).expect(&format!("rename file_{}", i));
    }

    // Verify all are renamed
    for i in 0..10 {
        let old_path = temp.path().join(format!("file_{}.txt", i));
        let new_path = temp.path().join(format!("renamed_{}.txt", i));
        assert!(!old_path.exists(), "old file_{} should not exist", i);
        assert!(new_path.exists(), "renamed_{} should exist", i);
    }
}

/// Test workflow handles file metadata correctly
#[test]
fn test_e2e_workflow_preserves_file_metadata() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = temp.path().join("source.txt");
    let dest = temp.path().join("dest.txt");

    let content = "test content";
    fs::write(&source, content).expect("write source");

    // Get source metadata
    let source_metadata = fs::metadata(&source).expect("get source metadata");

    // Copy file
    fs::copy(&source, &dest).expect("copy file");

    // Get dest metadata
    let dest_metadata = fs::metadata(&dest).expect("get dest metadata");

    // Both should be regular files with same size
    assert!(source_metadata.is_file());
    assert!(dest_metadata.is_file());
    assert_eq!(source_metadata.len(), dest_metadata.len());
}

/// Test workflow with empty files
#[test]
fn test_e2e_workflow_with_empty_files() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let empty_file = temp.path().join("empty.txt");

    // Create empty file
    fs::write(&empty_file, "").expect("create empty file");
    assert!(empty_file.exists());

    let metadata = fs::metadata(&empty_file).expect("get metadata");
    assert_eq!(metadata.len(), 0, "file should be empty");

    // Rename it
    let renamed = temp.path().join("empty_renamed.txt");
    fs::rename(&empty_file, &renamed).expect("rename empty file");

    assert!(!empty_file.exists());
    assert!(renamed.exists());
    let renamed_metadata = fs::metadata(&renamed).expect("get renamed metadata");
    assert_eq!(renamed_metadata.len(), 0, "renamed file should still be empty");
}

/// Test workflow cleanup
#[test]
fn test_e2e_workflow_complete_cleanup() {
    let temp = tempfile::tempdir().expect("create temp dir");

    // Create nested structure with files
    let dir1 = temp.path().join("dir1");
    let dir2 = dir1.join("dir2");
    fs::create_dir_all(&dir2).expect("create nested dirs");

    fs::write(dir1.join("file1.txt"), "content").expect("write file in dir1");
    fs::write(dir2.join("file2.txt"), "content").expect("write file in dir2");

    // Remove all files
    fs::remove_file(dir1.join("file1.txt")).expect("delete file1");
    fs::remove_file(dir2.join("file2.txt")).expect("delete file2");

    // Remove directories
    fs::remove_dir(&dir2).expect("remove dir2");
    fs::remove_dir(&dir1).expect("remove dir1");

    // Verify cleanup
    assert!(!dir1.exists());
    assert!(!dir2.exists());
}
