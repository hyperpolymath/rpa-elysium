# SPDX-License-Identifier: MPL-2.0
# Emit the official single-quoted, four-space-indented lockfile dialect. Native
# YAML parsers accept other styles, but Hypatia's deployed lock reader does not.
def quote: "'" + (tostring | gsub("'"; "''")) + "'";
if any(.dependencies[]; ((keys - ["hostname", "ref", "commit", "owner_id", "repo_id", "uses"]) | length) > 0)
then error("unknown dependency fields; review the lock schema before rendering")
else
  "# SPDX-License-Identifier: MPL-2.0",
  "# Resolved action graph; regenerate with scripts/update-actions-lock.sh.",
  ("version: " + (.version | quote)),
  ("workflows:" + (if (.workflows | length) == 0 then " {}" else "" end)),
  (.workflows | to_entries | sort_by(.key)[] |
    ("    " + (.key | quote) + ":" + (if (.value | length) == 0 then " []" else "" end)),
    (.value | sort[] | "        - " + quote)),
  ("dependencies:" + (if (.dependencies | length) == 0 then " {}" else "" end)),
  (.dependencies | to_entries | sort_by(.key)[] |
    ("    " + (.key | quote) + ":"),
    (.value as $d | ["hostname", "ref", "commit", "owner_id", "repo_id"][] as $field |
      select($d[$field] != null) |
      "        " + $field + ": " + ($d[$field] | if type == "number" then tostring else quote end)),
    (if (.value.uses // [] | length) > 0 then
      "        uses:", (.value.uses | sort[] | "            - " + quote)
    else empty end))
end
