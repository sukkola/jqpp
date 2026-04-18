## Context

No CI or release infrastructure exists today. The `openapiv3-filter` repo (same owner) has a working reference pipeline using `cross` for cross-compilation and `softprops/action-gh-release` for publishing. The Homebrew tap at `sukkola/homebrew-tap` already hosts one formula built from source; we will add a binary-based formula for faster installs. The tool is a single Rust binary with no runtime dependencies.

## Goals / Non-Goals

**Goals:**
- Automated lint/test on every push and PR
- Tag-triggered cross-platform release builds (Linux x86_64, Linux ARM64, macOS x86_64, macOS ARM64)
- GitHub Release with `jqpp-{target}.tar.gz` archives and `SHA256SUMS` file
- Homebrew formula in `sukkola/homebrew-tap` that downloads the pre-built macOS binary
- README that clearly explains why jqpp exists (intellisense + interactive jq in one tool), how to install it, and how to configure jq-lsp

**Non-Goals:**
- Windows builds (no demand yet; can be added later)
- Docker images
- Automated formula bump PRs (manual update for v1, automate later)
- Changelogs / release notes generation

## Decisions

**Use `cross` for Linux cross-compilation**
Same pattern as `openapiv3-filter`. Avoids needing custom runner images. macOS targets use native runners (`macos-latest`) since `cross` doesn't support macOS well.

**Binary archive naming: `jqpp-{target}.tar.gz`**
Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`. Consistent with Rust target triple conventions, easy to script against in the Homebrew formula.

**Homebrew formula installs pre-built binary**
Faster than `cargo install` from source. Formula selects the correct archive by CPU architecture. SHA-256 checksums in the formula come from the release's `SHA256SUMS` file.

**Single `SHA256SUMS` file in the release**
One file lists all four archive checksums. Homebrew formula references individual hashes inline (standard practice). The CI job generates this with `sha256sum`.

**CI and release are separate workflow files**
CI (`ci.yml`) runs on every push/PR. Release (`release.yml`) triggers only on `v*` tags. Keeps concerns separate and avoids accidental releases.

## Risks / Trade-offs

- [Homebrew formula requires manual hash update per release] → Acceptable for now; automate with a release-triggered PR bot in a follow-up
- [macOS ARM runner availability] → `macos-latest` on GitHub Actions is ARM (M1) as of 2024; x86 uses `macos-13`
- [`cross` version pinning] → Pin to a known-good version to avoid surprise breakage

## Open Questions

- Should the release workflow also update the Homebrew formula automatically (via a cross-repo PR)? Deferred — do manually first.
