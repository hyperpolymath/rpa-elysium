# SPDX-License-Identifier: PMPL-1.0-or-later
# Example RPA Elysium Filesystem Workflow (Nickel).
#
# This module defines an automated filesystem management policy using 
//! the RPA Elysium framework. It uses Nickel's record merging and 
//! functions to create a type-safe automation specification.

let home = "/home/user" in

{
  name = "nickel-workflow",
  description = "Standardized filesystem automation for project silos.",
  enabled = true,
  version = "1.0.0",

  # TRIGGERS: Directories to monitor for filesystem events.
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

  # AUTOMATION RULES: Define 'pattern -> event -> action' mappings.
  rules = [
    {
      name = "organize-images",
      patterns = ["*.jpg", "*.jpeg", "*.png", "*.gif", "*.webp"],
      events = ["created"],
      actions = [
        {
          # ACTION: Moves new images to a central sorting directory.
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
          # ACTION: Copies documents to an 'Incoming' queue while 
          # preserving the source file.
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
          # ACTION: Compresses large video files into a ZIP archive 
          # before moving to long-term storage.
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
