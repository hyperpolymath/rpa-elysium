# SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>
#
# Example RPA Elysium Filesystem Workflow in Nickel
# Run with: rpa-fs run workflow.ncl

let home = "/home/user" in

{
  name = "nickel-workflow",
  description = "Filesystem workflow defined in Nickel for type-safe configuration",
  enabled = true,
  version = "1.0.0",

  watch = [
    {
      path = "%{home}/Downloads",
      recursive = false,
    },
    {
      path = "%{home}/Documents/Inbox",
      recursive = true,
    },
  ],

  rules = [
    {
      name = "organize-images",
      patterns = ["*.jpg", "*.jpeg", "*.png", "*.gif", "*.webp"],
      events = ["created"],
      actions = [
        {
          type = "move",
          destination = "%{home}/Pictures/Unsorted",
          overwrite = false,
        },
      ],
      enabled = true,
    },
    {
      name = "organize-documents",
      patterns = ["*.pdf", "*.doc", "*.docx", "*.odt"],
      events = ["created"],
      actions = [
        {
          type = "copy",
          destination = "%{home}/Documents/Incoming",
          overwrite = false,
          preserve_structure = false,
        },
      ],
      enabled = true,
    },
    {
      name = "archive-videos",
      patterns = ["*.mp4", "*.mkv", "*.avi", "*.mov"],
      events = ["created"],
      actions = [
        {
          type = "archive",
          destination = "%{home}/Archives/Videos",
          format = "zip",
          delete_source = false,
        },
      ],
      enabled = true,
    },
  ],
}
