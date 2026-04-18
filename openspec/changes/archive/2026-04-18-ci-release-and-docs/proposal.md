## Why

jqpp has no automated release pipeline, no Homebrew distribution, and no README that explains its value proposition. Adding GitHub Actions CI/release workflows and a polished README makes the tool installable in one command and positions it clearly relative to the existing ecosystem (jqp for interactive queries, jq-lsp for language server) — jqpp is the tool that brings both together with built-in intellisense on top of jqp-like functionality.

## What Changes

- New GitHub Actions CI workflow: lint + test on every push/PR
- New GitHub Actions release workflow: cross-compile for Linux x86_64, Linux ARM64, macOS x86_64, macOS ARM64 (Apple Silicon); create GitHub Release with tarballs and SHA-256 checksums
- New Homebrew formula in `sukkola/homebrew-tap` repo (separate repo, documented as a task)
- Updated/new `README.md`: value proposition, install instructions (Homebrew + cargo), jq-lsp setup and usage documentation, attribution to jqp and jq-lsp
- New `demo/demo.tape` VHS script + `demo/demo.json` example data that records the intellisense features in action; the resulting video is the first visual in the README
- New `mise.toml` task `demo` that runs `vhs demo/demo.tape` to regenerate the recording

## Capabilities

### New Capabilities

- `ci-pipeline`: Automated lint/test checks on every push and pull request
- `release-pipeline`: Cross-platform binary builds, GitHub Release creation, and checksum generation on version tags
- `homebrew-formula`: Homebrew formula in `sukkola/homebrew-tap` enabling `brew install sukkola/tap/jqpp`
- `readme-docs`: Project README covering purpose, install, jq-lsp setup, jqp attribution, and intellisense rationale; demo video is the first element under the title
- `demo-recording`: VHS tape script, example JSON fixture, and mise task that produce a reproducible terminal demo video showing intellisense in action

### Modified Capabilities

<!-- No existing spec-level requirements are changing -->

## Impact

- New: `.github/workflows/ci.yml`
- New: `.github/workflows/release.yml`
- New/updated: `README.md`
- New: `demo/demo.tape`, `demo/demo.json`, `demo/demo.gif` (committed output)
- New: `mise.toml` with `demo` task
- External: `sukkola/homebrew-tap` — new formula file (out-of-repo task)
- No changes to application source code
