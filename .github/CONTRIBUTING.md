# Contributing to RPA Elysium

The canonical contributing guide is [`CONTRIBUTING.adoc`](../CONTRIBUTING.adoc)
at the repository root (estate policy: AsciiDoc by default).

This `.github/` copy exists only so GitHub surfaces a pointer on the
issue/PR templates; edit the root AsciiDoc guide, not this file.

## Signed Commits

Every commit that reaches the default branch must be signed; a ruleset refuses
unsigned pushes. Estate policy:
[SIGNING-POLICY](https://github.com/hyperpolymath/standards/blob/main/docs/SIGNING-POLICY.adoc).

- **People and interactive agents** sign with an SSH key registered on GitHub
  as a *signing* key (`gpg.format=ssh`, `commit.gpgsign=true`). The committer
  email must be verified on that account.
- **Apps, bots and workflows** never `git push` local commits. They write
  through the API (`createCommitOnBranch`, the estate `signed-push` action, or a
  squash merge) so that GitHub signs the commit.
- Merge PRs with **squash**. Rebase-merge replays commits unsigned and is
  disabled.

