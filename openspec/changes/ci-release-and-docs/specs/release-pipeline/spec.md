## ADDED Requirements

### Requirement: Release workflow triggers on version tags
A GitHub Actions workflow at `.github/workflows/release.yml` SHALL trigger when a tag matching `v*` is pushed.

#### Scenario: Tag push triggers release
- **WHEN** a tag like `v0.2.0` is pushed
- **THEN** the release workflow runs and produces a GitHub Release

### Requirement: Cross-platform binaries are built
The release workflow SHALL build `jqpp` binaries for the following targets:
- `x86_64-unknown-linux-gnu` (via `cross`)
- `aarch64-unknown-linux-gnu` (via `cross`)
- `x86_64-apple-darwin` (native macOS runner)
- `aarch64-apple-darwin` (native macOS ARM runner)

#### Scenario: All platform binaries produced
- **WHEN** the release workflow completes successfully
- **THEN** four binary archives exist, one per target

### Requirement: Release archives are named with target triple
Each binary SHALL be packaged as `jqpp-{target}.tar.gz` (e.g. `jqpp-x86_64-unknown-linux-gnu.tar.gz`).

#### Scenario: Archive naming
- **WHEN** the Linux x86_64 binary is packaged
- **THEN** the archive is named `jqpp-x86_64-unknown-linux-gnu.tar.gz`

### Requirement: SHA-256 checksums are published
The release workflow SHALL generate a `SHA256SUMS` file listing the SHA-256 hash of each archive and upload it alongside the archives to the GitHub Release.

#### Scenario: Checksums file present
- **WHEN** a GitHub Release is created
- **THEN** `SHA256SUMS` is attached as a release asset containing one `hash  filename` line per archive

### Requirement: GitHub Release is created automatically
The release workflow SHALL create a GitHub Release for the tag, attaching all four archives and the `SHA256SUMS` file.

#### Scenario: Release assets uploaded
- **WHEN** the release workflow succeeds
- **THEN** the GitHub Release contains four `.tar.gz` files and one `SHA256SUMS` file
