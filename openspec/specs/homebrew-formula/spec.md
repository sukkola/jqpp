# homebrew-formula Specification

## Purpose
TBD - created by archiving change ci-release-and-docs. Update Purpose after archive.
## Requirements
### Requirement: Homebrew formula installs jqpp from pre-built binary
A Homebrew formula at `Formula/jqpp.rb` in the `sukkola/homebrew-tap` repository SHALL allow users to install jqpp with `brew install sukkola/tap/jqpp`. The formula SHALL download the pre-built binary for the user's architecture rather than compiling from source.

#### Scenario: Install on Apple Silicon Mac
- **WHEN** a user runs `brew install sukkola/tap/jqpp` on an ARM Mac
- **THEN** Homebrew downloads the `jqpp-aarch64-apple-darwin.tar.gz` archive and places `jqpp` in the PATH

#### Scenario: Install on Intel Mac
- **WHEN** a user runs `brew install sukkola/tap/jqpp` on an x86_64 Mac
- **THEN** Homebrew downloads the `jqpp-x86_64-apple-darwin.tar.gz` archive and places `jqpp` in the PATH

### Requirement: Formula verifies binary integrity
The Homebrew formula SHALL include the SHA-256 hash for each architecture's archive so Homebrew can verify the download.

#### Scenario: Checksum verification
- **WHEN** Homebrew downloads the archive
- **THEN** it verifies the SHA-256 hash matches the formula's declared value before installing

### Requirement: Formula passes brew audit
The formula SHALL pass `brew audit --strict Formula/jqpp.rb` with no errors.

#### Scenario: Audit passes
- **WHEN** `brew audit --strict Formula/jqpp.rb` is run
- **THEN** no errors are reported

