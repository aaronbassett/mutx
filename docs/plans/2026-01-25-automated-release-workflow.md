# Automated Release Workflow Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Set up fully automated release pipeline with cargo-dist, cargo-release, and git-cliff for one-command releases that build cross-platform binaries and update Homebrew tap.

**Architecture:** cargo-release orchestrates local version bumping and git operations, git-cliff generates CHANGELOG from conventional commits, cargo-dist handles GitHub Actions workflow generation and cross-platform binary builds for 4 targets (x86_64/aarch64 Linux/macOS), automatic GitHub Releases, and Homebrew tap updates.

**Tech Stack:** cargo-dist, cargo-release, git-cliff, GitHub Actions, GitHub Releases, Homebrew

---

## Phase 1: Install and Initialize Tools

### Task 1: Install cargo-dist

**Files:**
- None (system-level installation)

**Step 1: Install cargo-dist**

Run:
```bash
cargo install cargo-dist
```

Expected: Successfully installed cargo-dist binary

**Step 2: Verify installation**

Run:
```bash
cargo dist --version
```

Expected: Version output (e.g., "cargo-dist 0.x.x")

---

### Task 2: Install cargo-release

**Files:**
- None (system-level installation)

**Step 1: Install cargo-release**

Run:
```bash
cargo install cargo-release
```

Expected: Successfully installed cargo-release binary

**Step 2: Verify installation**

Run:
```bash
cargo release --version
```

Expected: Version output (e.g., "cargo-release 0.x.x")

---

### Task 3: Install git-cliff

**Files:**
- None (system-level installation)

**Step 1: Install git-cliff**

Run:
```bash
cargo install git-cliff
```

Expected: Successfully installed git-cliff binary

**Step 2: Verify installation**

Run:
```bash
git cliff --version
```

Expected: Version output (e.g., "git-cliff 0.x.x")

---

## Phase 2: Configure cargo-dist

### Task 4: Initialize cargo-dist with interactive setup

**Files:**
- Modify: `Cargo.toml` (cargo-dist will add [workspace.metadata.dist] section)
- Create: `.github/workflows/release.yml`

**Step 1: Run cargo dist init**

Run:
```bash
cargo dist init
```

When prompted:
- "What kind of project is this?" → Select "Cargo workspace"
- "Select build targets:" → Select:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`
- "Select installers:" → Select "homebrew"
- "Homebrew tap:" → Enter `aaronbassett/homebrew-tap`
- "Enable GitHub CI?" → Yes

Expected: Creates `.github/workflows/release.yml` and updates `Cargo.toml`

**Step 2: Verify Cargo.toml changes**

Run:
```bash
grep -A 15 "\[workspace.metadata.dist\]" Cargo.toml
```

Expected: See dist configuration with targets, installers, tap, and CI settings

**Step 3: Verify GitHub Actions workflow created**

Run:
```bash
cat .github/workflows/release.yml | head -20
```

Expected: See workflow that triggers on tags matching `v*.*.*`

**Step 4: Commit cargo-dist initialization**

Run:
```bash
git add Cargo.toml .github/workflows/release.yml
git commit -m "chore: initialize cargo-dist for automated releases

Configure cargo-dist with 4 build targets (x86_64/aarch64 for Linux/macOS),
Homebrew installer, and GitHub Actions workflow generation.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Commit created successfully

---

## Phase 3: Configure git-cliff

### Task 5: Initialize git-cliff configuration

**Files:**
- Create: `cliff.toml`

**Step 1: Run git cliff init**

Run:
```bash
git cliff --init
```

Expected: Creates `cliff.toml` with default configuration

**Step 2: Verify cliff.toml created**

Run:
```bash
ls -la cliff.toml
```

Expected: File exists

**Step 3: Customize cliff.toml for Keep a Changelog format**

Edit: `cliff.toml`

Replace the entire file content with:

```toml
[changelog]
# changelog header
header = """
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

"""
# template for the changelog body
body = """
{% for group, commits in commits | group_by(attribute="group") %}
    ### {{ group | upper_first }}
    {% for commit in commits %}
        - {{ commit.message | upper_first | trim }}\
    {% endfor %}
{% endfor %}

"""
# remove the leading and trailing whitespace from the template
trim = true

[git]
# parse the commits based on https://www.conventionalcommits.org
conventional_commits = true
# filter out the commits that are not conventional
filter_unconventional = true
# process each line of a commit as an individual commit
split_commits = false
# regex for parsing and grouping commits
commit_parsers = [
  { message = "^feat", group = "Added" },
  { message = "^fix", group = "Fixed" },
  { message = "^doc", group = "Documentation" },
  { message = "^perf", group = "Performance" },
  { message = "^refactor", group = "Refactored" },
  { message = "^style", group = "Styling" },
  { message = "^test", group = "Testing" },
  { message = "^chore\\(release\\): prepare for", skip = true },
  { message = "^chore\\(deps\\)", skip = true },
  { message = "^chore\\(pr\\)", skip = true },
  { message = "^chore\\(pull\\)", skip = true },
  { message = "^chore|^ci", group = "Miscellaneous Tasks" },
  { body = ".*security", group = "Security" },
  { message = "^revert", group = "Reverted" },
]
# protect breaking changes from being skipped due to matching a skipping commit_parser
protect_breaking_commits = false
# filter out the commits that are not matched by commit parsers
filter_commits = false
# glob pattern for matching git tags
tag_pattern = "v[0-9]*"

# regex for skipping tags
skip_tags = "v0.1.0-beta.1"
# regex for ignoring tags
ignore_tags = ""
# sort the tags topologically
topo_order = false
# sort the commits inside sections by oldest/newest order
sort_commits = "oldest"
# limit the number of commits included in the changelog.
# limit_commits = 42
```

**Step 4: Test git-cliff on existing commits**

Run:
```bash
git cliff --unreleased
```

Expected: Generates changelog entries for commits since last tag (should be empty since no tags exist yet)

**Step 5: Commit cliff.toml**

Run:
```bash
git add cliff.toml
git commit -m "chore: configure git-cliff for changelog generation

Set up git-cliff with Keep a Changelog format, conventional commit parsing,
and grouping by commit type (Added, Fixed, Documentation, etc.).

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Commit created successfully

---

## Phase 4: Configure cargo-release

### Task 6: Create cargo-release configuration

**Files:**
- Create: `release.toml`

**Step 1: Create release.toml**

Create file at: `release.toml`

```toml
# cargo-release configuration
# https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md

# Pre-release checks
[pre-release-checks]
# Run tests before release
test = true
# Run cargo fmt check
format = true
# Run clippy
clippy = true

# Release configuration
[release]
# Don't publish to crates.io (using Homebrew instead)
publish = false
# Push to remote
push = true
# Create and push git tag
tag = true
# Tag prefix
tag-prefix = "v"
# Tag message
tag-message = "chore: release {{crate_name}} version {{version}}"
# Sign commits
sign-commit = false
# Sign tags
sign-tag = false

# Pre-release replacements and hooks
[[pre-release-replacements]]
file = "CHANGELOG.md"
search = "## \\[Unreleased\\]"
replace = "## [Unreleased]\n\n## [{{version}}] - {{date}}"
exactly = 1

[[pre-release-replacements]]
file = "CHANGELOG.md"
search = "\\[Unreleased\\]: https://github.com/aaronbassett/mutx/compare/v.*\\.\\.\\.HEAD"
replace = "[Unreleased]: https://github.com/aaronbassett/mutx/compare/v{{version}}...HEAD\n[{{version}}]: https://github.com/aaronbassett/mutx/compare/v{{prev_version}}...v{{version}}"
exactly = 1

# Hooks
[[pre-release-hook]]
# Regenerate CHANGELOG.md with git-cliff before release
command = ["git", "cliff", "--tag", "v{{version}}", "-o", "CHANGELOG.md"]
```

**Step 2: Verify configuration syntax**

Run:
```bash
cargo release --help
```

Expected: Help output showing cargo-release is working

**Step 3: Commit release.toml**

Run:
```bash
git add release.toml
git commit -m "chore: configure cargo-release for automated version bumping

Set up cargo-release with pre-release checks (tests, fmt, clippy),
git-cliff integration for CHANGELOG generation, and disabled crates.io
publishing (using Homebrew distribution instead).

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Commit created successfully

---

## Phase 5: Set up GitHub Repository and Secrets

### Task 7: Create GitHub repository and configure remote

**Files:**
- None (git configuration)

**Step 1: Create GitHub repository**

Manual action required:
1. Go to https://github.com/new
2. Repository name: `mutx`
3. Owner: `aaronbassett`
4. Visibility: Public
5. Do NOT initialize with README (we have existing code)
6. Click "Create repository"

**Step 2: Add GitHub remote**

Run:
```bash
git remote add origin https://github.com/aaronbassett/mutx.git
```

Expected: Remote added successfully

**Step 3: Verify remote**

Run:
```bash
git remote -v
```

Expected: Shows origin pointing to https://github.com/aaronbassett/mutx.git

**Step 4: Push existing code to GitHub**

Run:
```bash
git push -u origin main
```

Expected: All commits pushed to GitHub main branch

---

### Task 8: Create GitHub Personal Access Token for Homebrew tap

**Files:**
- None (GitHub web UI action)

**Step 1: Create PAT**

Manual action required:
1. Go to https://github.com/settings/tokens/new
2. Note: "mutx cargo-dist Homebrew tap updates"
3. Expiration: No expiration (or 1 year, your choice)
4. Select scopes:
   - `repo` (Full control of private repositories)
   - `workflow` (Update GitHub Action workflows)
5. Click "Generate token"
6. **IMPORTANT:** Copy the token immediately (you won't see it again)

Expected: Token copied to clipboard (format: `ghp_...`)

**Step 2: Add token as GitHub Actions secret**

Manual action required:
1. Go to https://github.com/aaronbassett/mutx/settings/secrets/actions
2. Click "New repository secret"
3. Name: `HOMEBREW_TAP_TOKEN`
4. Value: Paste the PAT from Step 1
5. Click "Add secret"

Expected: Secret `HOMEBREW_TAP_TOKEN` appears in secrets list

---

### Task 9: Verify Homebrew tap repository exists

**Files:**
- None (GitHub repository check)

**Step 1: Check if homebrew-tap exists**

Run:
```bash
curl -s -o /dev/null -w "%{http_code}" https://github.com/aaronbassett/homebrew-tap
```

Expected: `200` (repository exists) or `404` (needs to be created)

**Step 2: If 404, create homebrew-tap repository**

Manual action required (only if Step 1 returned 404):
1. Go to https://github.com/new
2. Repository name: `homebrew-tap`
3. Owner: `aaronbassett`
4. Visibility: Public
5. Initialize with README: Yes
6. Click "Create repository"

Expected: Repository `aaronbassett/homebrew-tap` exists and is public

---

## Phase 6: Update CHANGELOG for v1.0.0 Release

### Task 10: Prepare CHANGELOG for git-cliff

**Files:**
- Modify: `CHANGELOG.md`

**Step 1: Add Unreleased section header**

Edit: `CHANGELOG.md`

Add after line 7 (after "and this project adheres to..."):

```markdown

## [Unreleased]

```

So the file looks like:
```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-01-25
...
```

**Step 2: Add comparison links at bottom**

Edit: `CHANGELOG.md`

Add at the very end:

```markdown

[Unreleased]: https://github.com/aaronbassett/mutx/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/aaronbassett/mutx/releases/tag/v1.0.0
```

**Step 3: Verify CHANGELOG format**

Run:
```bash
head -15 CHANGELOG.md && tail -5 CHANGELOG.md
```

Expected: Shows Unreleased section header and comparison links

**Step 4: Commit CHANGELOG updates**

Run:
```bash
git add CHANGELOG.md
git commit -m "docs: prepare CHANGELOG for automated releases

Add Unreleased section header and version comparison links to enable
git-cliff and cargo-release automated changelog updates.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Commit created successfully

---

## Phase 7: Test Local Build

### Task 11: Test cargo-dist local build

**Files:**
- None (build artifacts in target/distrib/)

**Step 1: Run cargo dist plan**

Run:
```bash
cargo dist plan
```

Expected: Shows release plan with 4 targets, Homebrew installer, GitHub CI

**Step 2: Build locally for current platform**

Run:
```bash
cargo dist build --artifacts local
```

Expected: Builds binary for current platform, creates tarball in target/distrib/

**Step 3: Verify build artifacts**

Run:
```bash
ls -lh target/distrib/
```

Expected: See tarball file (e.g., `mutx-x86_64-apple-darwin.tar.gz` or similar)

**Step 4: Test binary from tarball**

Run:
```bash
tar -xzf target/distrib/mutx-*.tar.gz -C /tmp/
/tmp/mutx --version
```

Expected: Shows version `mutx 1.0.0`

**Step 5: Clean up test artifacts**

Run:
```bash
rm -rf target/distrib/ /tmp/mutx
```

Expected: Artifacts cleaned up

---

## Phase 8: Create v1.0.0 Release

### Task 12: Push to GitHub and create first release tag

**Files:**
- None (git operations)

**Step 1: Push all commits to GitHub**

Run:
```bash
git push origin main
```

Expected: All commits pushed successfully

**Step 2: Create and push v1.0.0 tag**

Run:
```bash
git tag -a v1.0.0 -m "chore: release mutx version 1.0.0"
git push origin v1.0.0
```

Expected: Tag pushed to GitHub, triggers GitHub Actions workflow

**Step 3: Monitor GitHub Actions workflow**

Manual action:
1. Go to https://github.com/aaronbassett/mutx/actions
2. Click on the "Release" workflow run for tag v1.0.0
3. Watch the build progress for all 4 targets

Expected: Workflow runs successfully (green checkmark) after ~5-10 minutes

**Step 4: Verify GitHub Release created**

Manual action:
1. Go to https://github.com/aaronbassett/mutx/releases
2. Check that v1.0.0 release exists

Expected: Release with 4 binary artifacts (.tar.gz files) and checksums

**Step 5: Verify Homebrew tap updated**

Run:
```bash
curl -s https://raw.githubusercontent.com/aaronbassett/homebrew-tap/main/Formula/mutx.rb | head -20
```

Expected: Shows Homebrew formula with version 1.0.0 and URLs pointing to GitHub release

---

## Phase 9: Test End-to-End Homebrew Installation

### Task 13: Test Homebrew installation from tap

**Files:**
- None (Homebrew system test)

**Step 1: Add tap (if not already added)**

Run:
```bash
brew tap aaronbassett/tap
```

Expected: Tap added successfully

**Step 2: Install mutx from tap**

Run:
```bash
brew install aaronbassett/tap/mutx
```

Expected: Downloads binary from GitHub release, installs to Homebrew cellar

**Step 3: Verify installation**

Run:
```bash
which mutx
mutx --version
```

Expected:
- Path shows Homebrew location (e.g., `/opt/homebrew/bin/mutx`)
- Version shows `mutx 1.0.0`

**Step 4: Test basic functionality**

Run:
```bash
echo "test content" | mutx /tmp/test-mutx-install.txt
cat /tmp/test-mutx-install.txt
rm /tmp/test-mutx-install.txt
```

Expected: File created with "test content", then deleted

**Step 5: Uninstall test installation (keep for actual use or remove)**

Run (optional):
```bash
brew uninstall mutx
```

Expected: mutx uninstalled from Homebrew

---

## Phase 10: Test cargo-release Workflow

### Task 14: Test cargo-release dry-run for future releases

**Files:**
- None (dry-run test)

**Step 1: Make a trivial change to test release flow**

Create file: `docs/test-release.md`

```markdown
# Test Release Workflow

This file is used to test the cargo-release workflow.
It will be removed after successful test.
```

**Step 2: Commit with conventional commit message**

Run:
```bash
git add docs/test-release.md
git commit -m "docs: add test file for release workflow verification"
```

Expected: Commit created successfully

**Step 3: Run cargo release dry-run**

Run:
```bash
cargo release patch --dry-run
```

Expected: Shows what would happen:
- Version would bump to 1.0.1
- CHANGELOG would be updated
- Commit would be created
- Tag v1.0.1 would be created
- Changes would be pushed

**Step 4: Review dry-run output**

Expected output should show:
- Pre-release checks (tests, fmt, clippy) would run
- git-cliff would regenerate CHANGELOG
- Version bump: 1.0.0 → 1.0.1
- Tag: v1.0.1
- Push to origin/main

**Step 5: Remove test file**

Run:
```bash
git reset HEAD~1
git clean -fd docs/test-release.md
```

Expected: Test commit removed, working directory clean

---

## Phase 11: Documentation

### Task 15: Update README with release workflow documentation

**Files:**
- Modify: `README.md`

**Step 1: Add Release section to README**

Edit: `README.md`

Add before the "License" section:

```markdown
## Release Process

This project uses automated releases with cargo-dist, cargo-release, and git-cliff.

### For Maintainers

**Making a release:**

```bash
# For bug fixes (1.0.0 -> 1.0.1)
cargo release patch --execute

# For new features (1.0.0 -> 1.1.0)
cargo release minor --execute

# For breaking changes (1.0.0 -> 2.0.0)
cargo release major --execute
```

The release process automatically:
1. Runs tests, clippy, and fmt checks
2. Regenerates CHANGELOG.md from conventional commits
3. Bumps version in Cargo.toml
4. Creates git tag and pushes to GitHub
5. Triggers GitHub Actions to build binaries for 4 platforms
6. Creates GitHub Release with binaries
7. Updates Homebrew tap formula

**Conventional commits:**

Use standardized commit prefixes for automatic changelog generation:
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `chore:` - Build process, dependencies
- `test:` - Test changes
- `refactor:` - Code refactoring

```

**Step 2: Update Installation section**

Edit: `README.md`

Update the "From crates.io" section to:

```markdown
### From Homebrew (macOS/Linux)

```bash
brew install aaronbassett/tap/mutx
```
```

Remove or comment out the crates.io installation since it's not published there:

```markdown
### From source

```bash
cargo install --path .
```
```

**Step 3: Commit README updates**

Run:
```bash
git add README.md
git commit -m "docs: document automated release workflow

Add release process documentation for maintainers including cargo-release
usage and conventional commit guidelines. Update installation instructions
to prioritize Homebrew tap.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Commit created successfully

**Step 4: Push README update**

Run:
```bash
git push origin main
```

Expected: Pushed successfully

---

## Phase 12: Final Validation

### Task 16: Validate complete setup

**Files:**
- None (validation checks)

**Step 1: Verify all configuration files exist**

Run:
```bash
ls -la Cargo.toml release.toml cliff.toml .github/workflows/release.yml
```

Expected: All files exist

**Step 2: Verify git remote configured**

Run:
```bash
git remote -v
```

Expected: Shows origin pointing to github.com/aaronbassett/mutx

**Step 3: Verify GitHub release exists**

Run:
```bash
curl -s https://api.github.com/repos/aaronbassett/mutx/releases/latest | grep tag_name
```

Expected: Shows `"tag_name": "v1.0.0"`

**Step 4: Verify Homebrew tap formula exists**

Run:
```bash
curl -s https://raw.githubusercontent.com/aaronbassett/homebrew-tap/main/Formula/mutx.rb | grep 'version "'
```

Expected: Shows `version "1.0.0"`

**Step 5: Verify tools installed**

Run:
```bash
cargo dist --version && cargo release --version && git cliff --version
```

Expected: All three tools show version numbers

**Step 6: Document completion**

Create file: `docs/release-workflow-complete.md`

```markdown
# Release Workflow Setup Complete

**Date:** $(date +%Y-%m-%d)

## Verification Checklist

- [x] cargo-dist installed and configured
- [x] cargo-release installed and configured
- [x] git-cliff installed and configured
- [x] GitHub repository created and pushed
- [x] GitHub Actions workflow generated
- [x] GitHub Personal Access Token created and stored
- [x] Homebrew tap repository configured
- [x] v1.0.0 release successfully built and published
- [x] Homebrew formula updated automatically
- [x] Installation tested from Homebrew tap
- [x] Documentation updated

## Next Steps

For future releases, simply run:

\`\`\`bash
cargo release [patch|minor|major] --execute
\`\`\`

The entire pipeline will execute automatically.

## Monitoring

- GitHub Actions: https://github.com/aaronbassett/mutx/actions
- Releases: https://github.com/aaronbassett/mutx/releases
- Homebrew tap: https://github.com/aaronbassett/homebrew-tap
```

**Step 7: Commit completion documentation**

Run:
```bash
git add docs/release-workflow-complete.md
git commit -m "docs: document release workflow setup completion

Add verification checklist and next steps for future releases.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push origin main
```

Expected: Commit created and pushed successfully

---

## Success Criteria

The implementation is complete when:

1. ✅ cargo-dist, cargo-release, and git-cliff are installed
2. ✅ Cargo.toml contains dist metadata for 4 build targets
3. ✅ cliff.toml configured for Keep a Changelog format
4. ✅ release.toml configured with pre-release checks and git-cliff integration
5. ✅ .github/workflows/release.yml exists and triggers on version tags
6. ✅ GitHub repository created with all code pushed
7. ✅ HOMEBREW_TAP_TOKEN secret configured in GitHub
8. ✅ v1.0.0 tag pushed and GitHub Actions workflow completed successfully
9. ✅ GitHub Release created with 4 binary artifacts
10. ✅ Homebrew tap formula updated with v1.0.0
11. ✅ Installation from Homebrew tap tested and working
12. ✅ Documentation updated with release process
13. ✅ cargo release dry-run validated for future releases

## Troubleshooting

**GitHub Actions fails:**
- Check workflow logs in Actions tab
- Verify HOMEBREW_TAP_TOKEN has correct permissions
- Ensure homebrew-tap repository exists and is public

**Homebrew tap not updated:**
- Check Actions logs for "Update Homebrew tap" step
- Verify PAT has `repo` and `workflow` scopes
- Check homebrew-tap repository for commits from github-actions bot

**Binary doesn't work:**
- Verify build target matches platform (x86_64 vs aarch64)
- Check GitHub Release has all 4 platform binaries
- Test local build with `cargo dist build --artifacts local`

**cargo release fails pre-checks:**
- Run `cargo test` to verify tests pass
- Run `cargo clippy` to check for linting errors
- Run `cargo fmt` to format code
- Ensure CHANGELOG.md has Unreleased section
