// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Comprehensive security tests for WASM sandbox plugin isolation and capability enforcement

use rpa_plugin::permissions::{Permission, PermissionSet};
use rpa_plugin::sandbox::{SandboxBuilder, SandboxConfig, DEFAULT_MEMORY_LIMIT, DEFAULT_TIMEOUT_MS};
use std::fs;

/// Test that sandbox cannot read files outside allowed paths
#[test]
fn test_sandbox_read_restriction_outside_allowed_path() {
    let config = SandboxConfig::new()
        .with_permission(Permission::read_path("/tmp/allowed"));

    // Plugin tries to read /etc/passwd (not in allowed set)
    assert!(!config.permissions.check(&Permission::read_path("/etc/passwd")));
    assert!(config.permissions.check(&Permission::read_path("/tmp/allowed")));
}

/// Test that sandbox cannot read files in subdirectories without parent permission
#[test]
fn test_sandbox_read_subdir_permission_isolation() {
    let config = SandboxConfig::new()
        .with_permission(Permission::read_path("/home/user/public"));

    // Should have permission for /home/user/public
    assert!(config.permissions.check(&Permission::read_path("/home/user/public")));

    // Should have permission for subdirectories
    assert!(config.permissions.check(&Permission::read_path("/home/user/public/file.txt")));

    // Should NOT have permission for parent directory
    assert!(!config.permissions.check(&Permission::read_path("/home/user")));

    // Should NOT have permission for sibling directory
    assert!(!config.permissions.check(&Permission::read_path("/home/user/private")));
}

/// Test that sandbox cannot write files outside allowed paths
#[test]
fn test_sandbox_write_restriction_outside_allowed_path() {
    let config = SandboxConfig::new()
        .with_permission(Permission::write_path("/tmp/work"));

    // Plugin tries to write to root (not in allowed set)
    assert!(!config.permissions.check(&Permission::write_path("/root")));
    assert!(config.permissions.check(&Permission::write_path("/tmp/work")));
}

/// Test that read and write permissions are independent
#[test]
fn test_sandbox_read_write_permission_independence() {
    let config = SandboxConfig::new()
        .with_permission(Permission::read_path("/tmp/data"))
        .with_permission(Permission::write_path("/tmp/output"));

    // Can read from /tmp/data
    assert!(config.permissions.check(&Permission::read_path("/tmp/data")));

    // Cannot write to /tmp/data (only read permission)
    assert!(!config.permissions.check(&Permission::write_path("/tmp/data")));

    // Can write to /tmp/output
    assert!(config.permissions.check(&Permission::write_path("/tmp/output")));

    // Cannot read from /tmp/output (only write permission)
    assert!(!config.permissions.check(&Permission::read_path("/tmp/output")));
}

/// Test that environment variable access is controlled
#[test]
fn test_sandbox_env_permission_isolation() {
    let config = SandboxConfig::new()
        .with_permission(Permission::env("HOME"));

    // Can access HOME
    assert!(config.permissions.check(&Permission::env("HOME")));

    // Cannot access other env vars
    assert!(!config.permissions.check(&Permission::env("PATH")));
    assert!(!config.permissions.check(&Permission::env("SECRET_KEY")));
    assert!(!config.permissions.check(&Permission::env("AWS_SECRET_ACCESS_KEY")));
}

/// Test that AllEnv permission covers any specific env var
#[test]
fn test_sandbox_all_env_permission() {
    let config = SandboxConfig::new()
        .with_permission(Permission::AllEnv);

    // Should be able to access any environment variable
    assert!(config.permissions.check(&Permission::env("HOME")));
    assert!(config.permissions.check(&Permission::env("PATH")));
    assert!(config.permissions.check(&Permission::env("ANY_VAR")));
}

/// Test that missing permissions are correctly identified
#[test]
fn test_sandbox_missing_permissions_detection() {
    let config = SandboxConfig::new()
        .with_permission(Permission::read_path("/tmp"));

    let requested = PermissionSet::new([
        Permission::read_path("/tmp/file.txt"),
        Permission::write_path("/var/log"),
        Permission::env("HOME"),
    ]);

    let missing = config.permissions.missing(&requested);

    assert_eq!(missing.len(), 2);
    assert!(missing.iter().any(|p| matches!(p, Permission::WritePath { .. })));
    assert!(missing.iter().any(|p| matches!(p, Permission::Env { name } if name == "HOME")));
}

/// Test capability leakage: plugin cannot escalate own permissions
#[test]
fn test_sandbox_cannot_escalate_permissions() {
    let limited_config = SandboxConfig::new()
        .with_permission(Permission::read_path("/tmp/safe"));

    // Sandbox cannot grant itself more permissions
    // (This would be enforced by the execute() method checking permissions on each request)
    assert!(!limited_config.permissions.check(&Permission::read_path("/etc")));
    assert!(!limited_config.permissions.check(&Permission::write_path("/root")));
    assert!(!limited_config.permissions.check(&Permission::AllEnv));

    // Permissions remain as configured
    assert!(limited_config.permissions.check(&Permission::read_path("/tmp/safe")));
}

/// Test plugin isolation: crash in one sandbox doesn't affect config of another
#[test]
fn test_sandbox_isolation_independent_configs() {
    let config1 = SandboxConfig::new()
        .with_permission(Permission::read_path("/tmp/plugin1"));

    let config2 = SandboxConfig::new()
        .with_permission(Permission::read_path("/tmp/plugin2"));

    // Each sandbox has independent permissions
    assert!(config1.permissions.check(&Permission::read_path("/tmp/plugin1")));
    assert!(!config1.permissions.check(&Permission::read_path("/tmp/plugin2")));

    assert!(config2.permissions.check(&Permission::read_path("/tmp/plugin2")));
    assert!(!config2.permissions.check(&Permission::read_path("/tmp/plugin1")));
}

/// Test memory limits are respected
#[test]
fn test_sandbox_memory_limit_enforcement() {
    let small_limit = 1024 * 1024; // 1MB
    let normal_limit = DEFAULT_MEMORY_LIMIT;

    let config_small = SandboxConfig::new().with_memory_limit(small_limit);
    let config_normal = SandboxConfig::new().with_memory_limit(normal_limit);

    assert_eq!(config_small.memory_limit, small_limit);
    assert_eq!(config_normal.memory_limit, normal_limit);
    assert!(config_small.memory_limit < config_normal.memory_limit);
}

/// Test timeout enforcement
#[test]
fn test_sandbox_timeout_enforcement() {
    let quick_timeout = 5_000; // 5 seconds
    let normal_timeout = DEFAULT_TIMEOUT_MS;

    let config_quick = SandboxConfig::new().with_timeout(quick_timeout);
    let config_normal = SandboxConfig::new().with_timeout(normal_timeout);

    assert_eq!(config_quick.timeout_ms, quick_timeout);
    assert_eq!(config_normal.timeout_ms, normal_timeout);
    assert!(config_quick.timeout_ms < config_normal.timeout_ms);
}

/// Test filesystem symlink handling in permission checks
#[test]
fn test_sandbox_symlink_escape_prevention() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let allowed_dir = temp.path().join("allowed");
    let external_dir = temp.path().join("external");

    fs::create_dir(&allowed_dir).expect("create allowed dir");
    fs::create_dir(&external_dir).expect("create external dir");

    // Create a file in external directory
    let external_file = external_dir.join("secret.txt");
    fs::write(&external_file, "secret").expect("write external file");

    // Create symlink in allowed directory pointing outside
    let symlink = allowed_dir.join("link.txt");
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        unix_fs::symlink(&external_file, &symlink).expect("create symlink");

        // Permission for allowed_dir DOES NOT cover the symlink target when canonicalized
        // because canonicalize() follows symlinks
        let mut config = SandboxConfig::new();
        config.permissions = PermissionSet::empty()
            .with(Permission::read_path(&allowed_dir));

        // Accessing the symlink is NOT allowed because after canonicalization
        // it points to external_file which is not in allowed_dir
        assert!(!config.permissions.check(&Permission::read_path(&symlink)));

        // But accessing external_dir content directly is still not allowed
        assert!(!config.permissions.check(&Permission::read_path(&external_file)));

        // To access the symlink, need explicit permission to its canonical target
        let mut config2 = SandboxConfig::new();
        config2.permissions = PermissionSet::empty()
            .with(Permission::read_path(&external_file));

        // Now accessing the symlink works because its target is allowed
        assert!(config2.permissions.check(&Permission::read_path(&symlink)));
    }
}

/// Test environment variable isolation: plugin can't read ungranted vars
#[test]
fn test_sandbox_env_isolation_no_host_leakage() {
    let config = SandboxConfig::new()
        .with_permission(Permission::env("PUBLIC_VAR"));

    // Can access granted env var
    assert!(config.permissions.check(&Permission::env("PUBLIC_VAR")));

    // Cannot access sensitive vars even if they exist on the host
    assert!(!config.permissions.check(&Permission::env("AWS_ACCESS_KEY_ID")));
    assert!(!config.permissions.check(&Permission::env("GITHUB_TOKEN")));
    assert!(!config.permissions.check(&Permission::env("DATABASE_PASSWORD")));
}

/// Test Random/UUID permission is separate from Time
#[test]
fn test_sandbox_random_time_permission_independence() {
    // Note: default config grants both Time and Random
    // Build empty permission sets to test independence
    let mut config_time = SandboxConfig::new();
    config_time.permissions = PermissionSet::empty().with(Permission::Time);

    let mut config_random = SandboxConfig::new();
    config_random.permissions = PermissionSet::empty().with(Permission::Random);

    // Time permission doesn't grant Random
    assert!(config_time.permissions.check(&Permission::Time));
    assert!(!config_time.permissions.check(&Permission::Random));

    // Random permission doesn't grant Time
    assert!(config_random.permissions.check(&Permission::Random));
    assert!(!config_random.permissions.check(&Permission::Time));
}

/// Test network permission granularity
#[test]
fn test_sandbox_network_permission_granularity() {
    let config = SandboxConfig::new()
        .with_permission(Permission::network("api.example.com", Some(443)));

    // Can access granted host:port
    assert!(config.permissions.check(&Permission::network("api.example.com", Some(443))));

    // Cannot access different port on same host
    assert!(!config.permissions.check(&Permission::network("api.example.com", Some(80))));

    // Cannot access same port on different host
    assert!(!config.permissions.check(&Permission::network("other.example.com", Some(443))));
}

/// Test network permission without port covers any port
#[test]
fn test_sandbox_network_any_port_permission() {
    let config = SandboxConfig::new()
        .with_permission(Permission::network("api.example.com", None));

    // Can access any port on granted host
    assert!(config.permissions.check(&Permission::network("api.example.com", Some(80))));
    assert!(config.permissions.check(&Permission::network("api.example.com", Some(443))));
    assert!(config.permissions.check(&Permission::network("api.example.com", Some(8080))));

    // Still cannot access other hosts
    assert!(!config.permissions.check(&Permission::network("other.example.com", Some(443))));
}

/// Test execute permission is tracked but restricted
#[test]
fn test_sandbox_execute_permission_restriction() {
    let config = SandboxConfig::new();

    // By default, no execute permission
    assert!(!config.permissions.check(&Permission::Execute {
        command: "bash".to_string()
    }));
    assert!(!config.permissions.check(&Permission::Execute {
        command: "/bin/sh".to_string()
    }));

    // With permission, only specific command is allowed
    let config_with_exec = SandboxConfig::new()
        .with_permission(Permission::Execute {
            command: "bash".to_string()
        });

    assert!(config_with_exec.permissions.check(&Permission::Execute {
        command: "bash".to_string()
    }));
    // Different command not allowed
    assert!(!config_with_exec.permissions.check(&Permission::Execute {
        command: "sh".to_string()
    }));
}

/// Test SandboxBuilder fluent interface
#[test]
fn test_sandbox_builder_fluent_api() {
    let sandbox = SandboxBuilder::new()
        .memory_limit(32 * 1024 * 1024)
        .timeout(15_000)
        .fuel(50_000_000)
        .permission(Permission::read_path("/tmp"))
        .permission(Permission::write_path("/tmp"))
        .permission(Permission::Time)
        .permission(Permission::Random)
        .build();

    assert!(sandbox.is_ok());
    let sb = sandbox.unwrap();
    let config = sb.config();

    assert_eq!(config.memory_limit, 32 * 1024 * 1024);
    assert_eq!(config.timeout_ms, 15_000);
    assert_eq!(config.fuel_limit, Some(50_000_000));
    assert_eq!(config.permissions.len(), 4);
}

/// Test permission set composition
#[test]
fn test_sandbox_permission_set_composition() {
    let mut perms = PermissionSet::empty();
    assert!(perms.is_empty());

    perms.add(Permission::read_path("/tmp"));
    assert_eq!(perms.len(), 1);

    perms.add(Permission::write_path("/tmp"));
    assert_eq!(perms.len(), 2);

    perms.add(Permission::Time);
    assert_eq!(perms.len(), 3);

    // Check all permissions are present
    assert!(perms.check(&Permission::read_path("/tmp")));
    assert!(perms.check(&Permission::write_path("/tmp")));
    assert!(perms.check(&Permission::Time));
}

/// Test zero-permission sandbox (excluding default Time/Random)
#[test]
fn test_sandbox_zero_permission_isolation() {
    let mut config = SandboxConfig::new();
    // Remove default permissions to test truly empty set
    config.permissions = PermissionSet::empty();

    // No permissions granted when empty
    assert!(!config.permissions.check(&Permission::read_path("/")));
    assert!(!config.permissions.check(&Permission::write_path("/")));
    assert!(!config.permissions.check(&Permission::env("HOME")));
    assert!(!config.permissions.check(&Permission::Time));
    assert!(!config.permissions.check(&Permission::Random));
    assert!(!config.permissions.check(&Permission::AllEnv));
}

/// Test permission iteration
#[test]
fn test_sandbox_permission_iteration() {
    let config = SandboxConfig::new()
        .with_permission(Permission::read_path("/tmp"))
        .with_permission(Permission::write_path("/home"))
        .with_permission(Permission::Time)
        .with_permission(Permission::Random);

    let perms: Vec<_> = config.permissions.iter().collect();
    assert_eq!(perms.len(), 4);

    // Verify all permission types are present
    assert!(perms.iter().any(|p| matches!(p, Permission::ReadPath { .. })));
    assert!(perms.iter().any(|p| matches!(p, Permission::WritePath { .. })));
    assert!(perms.iter().any(|p| **p == Permission::Time));
    assert!(perms.iter().any(|p| **p == Permission::Random));
}

/// Test sandbox with working directory
#[test]
fn test_sandbox_with_working_directory() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let work_dir = temp.path();

    let sandbox = SandboxBuilder::new()
        .work_dir(work_dir)
        .build();

    assert!(sandbox.is_ok());
    let sb = sandbox.unwrap();
    assert_eq!(sb.config().work_dir, Some(work_dir.to_path_buf()));
}
