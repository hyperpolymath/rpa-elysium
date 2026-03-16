<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
# rpa-fs-workflow

Filesystem workflow automation CLI for [RPA Elysium](../../README.adoc).

## Overview

`rpa-fs` watches directories for file system events (create, modify, delete, rename) and
executes configured actions — copy, move, archive, delete, rename, or plugin-based operations.

## Installation

```bash
cargo install --path .
# or from workspace root:
just install
```

## Usage

```bash
# Generate an example workflow config
rpa-fs init workflow.json

# Validate a config without running
rpa-fs validate workflow.json

# Run a workflow
rpa-fs run workflow.json

# Dry run — validate and display plan without executing
rpa-fs run --dry-run workflow.json

# Verbose output
rpa-fs -v run workflow.json

# Show version
rpa-fs --version
```

## Configuration

Workflow configs are JSON or [Nickel](https://nickel-lang.org/) files.

### JSON Example

```json
{
  "name": "backup-documents",
  "description": "Auto-backup PDFs and spreadsheets",
  "watch": [
    { "path": "/home/user/Downloads", "recursive": true }
  ],
  "rules": [
    {
      "name": "backup-pdfs",
      "patterns": ["*.pdf"],
      "events": ["created"],
      "actions": [
        { "type": "copy", "destination": "/home/user/Backups/pdf" }
      ]
    },
    {
      "name": "archive-old-logs",
      "patterns": ["*.log"],
      "events": ["modified"],
      "actions": [
        { "type": "archive", "destination": "/home/user/Archives", "format": "tar_gz" }
      ]
    }
  ]
}
```

### Nickel Example

See [`examples/workflow.ncl`](../../examples/workflow.ncl) for a Nickel configuration example.

## Action Types

| Action | Description |
|--------|-------------|
| `copy` | Copy file to destination (with optional overwrite and structure preservation) |
| `move` | Move file to destination |
| `archive` | Compress file to tar.gz or zip archive |
| `delete` | Delete the file (with optional trash support) |
| `rename` | Rename file using `{name}` and `{ext}` pattern substitution |
| `plugin` | Execute a WASM plugin action (requires rpa-plugin integration) |

## Event Types

| Event | Triggers when |
|-------|---------------|
| `created` | A new file appears in the watched directory |
| `modified` | An existing file is changed |
| `deleted` | A file is removed |
| `renamed` | A file is renamed or moved within the watch scope |

## Architecture

```
rpa-fs-workflow/
├── src/
│   ├── main.rs       # CLI entry point (clap)
│   ├── lib.rs         # Public API
│   ├── config.rs      # JSON/Nickel config parsing
│   ├── runner.rs      # Workflow execution engine
│   ├── watcher.rs     # Filesystem event watcher (notify)
│   └── actions/       # Action implementations
│       ├── copy.rs
│       ├── move_file.rs
│       ├── archive.rs
│       ├── delete.rs
│       ├── rename.rs
│       └── plugin.rs
```

## License

PMPL-1.0-or-later
