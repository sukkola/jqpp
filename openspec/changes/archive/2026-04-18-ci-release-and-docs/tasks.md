## 1. CI Workflow

- [x] 1.1 Create `.github/workflows/ci.yml` that triggers on push and pull_request
- [x] 1.2 Add job: checkout, install Rust stable, run `cargo clippy -- -D warnings`
- [x] 1.3 Add job: run `cargo test`

## 2. Release Workflow

- [x] 2.1 Create `.github/workflows/release.yml` that triggers on `v*` tag push
- [x] 2.2 Add Linux build matrix job using `cross` for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`
- [x] 2.3 Add macOS build jobs: `x86_64-apple-darwin` on `macos-13` runner, `aarch64-apple-darwin` on `macos-latest` runner
- [x] 2.4 Package each binary as `jqpp-{target}.tar.gz`
- [x] 2.5 Generate `SHA256SUMS` file from all four archives
- [x] 2.6 Upload all archives and `SHA256SUMS` to GitHub Release using `softprops/action-gh-release`

## 3. Homebrew Formula

- [x] 3.1 In `sukkola/homebrew-tap`, create `Formula/jqpp.rb` using `on_macos` block with `on_arm` / `on_intel` architecture selectors (Template created in `homebrew/jqpp.rb`)
- [x] 3.2 Set archive URLs pointing to the GitHub Release assets (`https://github.com/sukkola/jqpp/releases/download/v{version}/jqpp-{target}.tar.gz`)
- [x] 3.3 SHA-256 hashes are now filled automatically by the release pipeline
- [x] 3.3a Add `HOMEBREW_TAP_TOKEN` secret to the jqpp repo (GitHub PAT with `contents: write` on `sukkola/homebrew-tap`)
- [x] 3.4 Add `bin.install "jqpp"` and a `test` block (`system "#{bin}/jqpp", "--version"`)
- [x] 3.5 Run `brew audit --strict Formula/jqpp.rb` and fix any issues

## 4. Demo Recording

- [x] 4.1 Create `demo/demo.json` — nested JSON with an `orders` array; each order has `id`, `status`, `customer` (object with `name`, `email`), and `items` (array of objects with `sku`, `qty`, `price`)
- [x] 4.2 Create `demo/demo.tape` — set `Output demo/demo.gif`, `FontSize 16`, `Width 1200`, `Height 700`, dark theme; set `TypingSpeed 90ms`
- [x] 4.3 In the tape: launch `jqpp demo/demo.json`, `Sleep 1500ms` for TUI to render
- [x] 4.4 In the tape: type `.` and `Sleep 800ms` to show top-level field suggestions; press `Down` twice, `Tab` to accept `.orders`; `Sleep 600ms`
- [x] 4.5 In the tape: type `[]` then `.` (`Sleep 800ms`) to show nested object field suggestions from the items inside the array; navigate and accept `.customer`; `Sleep 600ms`
- [x] 4.6 In the tape: type `.` again to show `customer` sub-fields (`name`, `email`); accept `.name`; `Sleep 800ms` to show the result pane updating
- [x] 4.7 In the tape: clear query (select all, delete), type `.orders[] | .items[] | .` and `Sleep 800ms` to show item-level field completion
- [x] 4.8 In the tape: clear, type `.orders[] | sel` and `Sleep 600ms` to show `select` builtin suggestion; `Ctrl+C` to exit; `Sleep 500ms`
- [x] 4.9 Run `vhs demo/demo.tape` locally, review the GIF, adjust sleeps/steps until pacing feels natural
- [x] 4.10 Create `mise.toml` at repo root with `[tasks.demo]` that runs `vhs demo/demo.tape` from the repo root

## 5. README

- [x] 5.1 Open README with `# jq++` title and one-line tagline, then immediately embed `<img src="demo/demo.gif" alt="jqpp demo">` — nothing above it except the title
- [x] 5.2 Write value proposition below the GIF: jqpp = interactive jq like jqp + intellisense (field completions from live JSON, type-aware builtins, optional LSP diagnostics); explicitly call out that jqp does not provide completion
- [x] 5.3 Add installation section with Homebrew (`brew install sukkola/tap/jqpp`) and Cargo (`cargo install jqpp`) instructions
- [x] 5.4 Add jq-lsp section: what it is, `go install github.com/wader/jq-lsp@latest`, enable with `--lsp` flag, override binary with `JQPP_LSP_BIN`
- [x] 5.5 Add "Related projects" section: link to [jqp](https://github.com/noahgorstein/jqp) (interactive jq TUI inspiration) and [jq-lsp](https://github.com/wader/jq-lsp) (language server)
- [x] 5.6 Add basic usage section (keybindings, query bar, pane navigation)
