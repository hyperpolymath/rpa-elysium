# SPDX-License-Identifier: MPL-2.0
# Owner/repository names are case-insensitive; git refs are not.
def norm:
  if type != "string" then error("uses/ref must be a string")
  elif startswith("./") or startswith("docker://") then empty
  elif test("^[^/@]+/[^/@]+(?:/[^@]+)?@[^@[:space:]]+$") then
    capture("^(?<owner>[^/@]+)/(?<repo>[^/@]+)(?:/[^@]+)?@(?<ref>[^@[:space:]]+)$")
    | ((.owner + "/" + .repo | ascii_downcase) + "@" + .ref)
  else error("invalid external uses/ref: " + .) end;
def refs:
  if type != "array" then error("lock refs must be arrays")
  else map(norm) | unique end;
def normal_dependencies:
  to_entries | map({key: (.key | norm), value: .value}) | from_entries;
# Fixed point, rather than recursive traversal: cycles terminate and missing
# nodes remain in the result for the separate dangling-edge check to reject.
def reachable($roots; $deps):
  {seen: ($roots | refs), prior: null} |
  until(.seen == .prior;
    .prior = .seen |
    .seen = (.seen + [.seen[] as $ref | ($deps[$ref].uses // [] | refs[])] | unique)) |
  .seen;
