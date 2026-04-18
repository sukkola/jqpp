# readme-docs Specification

## Purpose
TBD - created by archiving change ci-release-and-docs. Update Purpose after archive.
## Requirements
### Requirement: README demo video is the first element after the title
Immediately after the `# jq++` title and one-line tagline, the README SHALL embed `demo/demo.gif` as an `<img>` tag (not a markdown image link) so it renders inline on GitHub. No installation instructions, badges, or prose SHALL appear above it.

#### Scenario: Demo GIF renders on GitHub
- **WHEN** a developer opens the GitHub repository page
- **THEN** the first thing they see below the title is the animated demo GIF playing

### Requirement: README explains the value proposition
The `README.md` SHALL explain — directly below the demo GIF — that jqpp provides interactive jq exploration (like jqp) plus built-in intellisense: field-name completion from live JSON, type-aware jq builtin suggestions, and optional LSP-powered diagnostics. The key differentiator over jqp SHALL be stated explicitly: jqpp gives you IDE-quality completion while you explore.

#### Scenario: Reader understands positioning
- **WHEN** a developer reads the README introduction
- **THEN** they understand jqpp is jqp-like interactive jq exploration with intellisense on top, and that jqp does not provide this

### Requirement: README attributes jqp
The README SHALL include a "See also" or "Related" section that mentions [jqp by Noah Gorstein](https://github.com/noahgorstein/jqp) as an alternative interactive jq TUI and credits it as an inspiration.

#### Scenario: jqp attribution present
- **WHEN** a user reads the README
- **THEN** they find a link to `https://github.com/noahgorstein/jqp` with a brief description

### Requirement: README documents jq-lsp setup and usage
The README SHALL include a dedicated section explaining what [jq-lsp](https://github.com/wader/jq-lsp) is, how to install it, and how to enable it in jqpp via the `--lsp` flag and `JQPP_LSP_BIN` environment variable. It SHALL clarify that jq-lsp is optional and what additional features it enables (diagnostics).

#### Scenario: jq-lsp setup documented
- **WHEN** a user wants to enable LSP support
- **THEN** the README tells them to install jq-lsp (`go install github.com/wader/jq-lsp@latest`) and run jqpp with `--lsp`

#### Scenario: JQPP_LSP_BIN documented
- **WHEN** a user has jq-lsp at a non-default path
- **THEN** the README explains they can set `JQPP_LSP_BIN=/path/to/jq-lsp`

### Requirement: README includes installation instructions
The README SHALL document two install methods: Homebrew (`brew install sukkola/tap/jqpp`) and Cargo (`cargo install jqpp`).

#### Scenario: Homebrew install documented
- **WHEN** a macOS user reads the install section
- **THEN** they find the `brew install sukkola/tap/jqpp` command

#### Scenario: Cargo install documented
- **WHEN** a Rust toolchain user reads the install section
- **THEN** they find `cargo install jqpp`

