# decmp

A multi-format archive utility with a CLI and a terminal-based file browser (TUI).

## Supported formats

| Format | List | Extract | Create | Encrypt |
|--------|------|---------|--------|---------|
| ZIP    | ✅   | ✅      | ✅     | ✅ (AES256) |
| 7z     | ✅   | ✅      | ✅     | ✅ |
| tar    | ✅   | ✅      | ✅     | — |
| tar.gz | ✅   | ✅      | ✅     | — |
| tar.xz | ✅   | ✅      | ✅     | — |
| tar.zst| ✅   | ✅      | ✅     | — |
| tar.bz2| ✅   | ✅      | ✅     | — |
| gz     | ✅   | ✅      | ✅     | — |
| xz     | ✅   | ✅      | ✅     | — |
| zst    | ✅   | ✅      | ✅     | — |
| bz2    | ✅   | ✅      | ✅     | — |

## Installation

```bash
cargo install --path crates/decmp
cargo install --path crates/decmp-tui
```

Or build from source:

```bash
cargo build --workspace --release
```

## CLI usage

### List

```bash
decmp list --file archive.zip
decmp list -f archive.tar.gz --encoding GBK
```

### Extract

```bash
decmp extract --file archive.zip --output ./out
decmp extract -f encrypted.7z -o ./out -p mypassword
```

### Create

```bash
decmp create --file archive.zip --sources file1.txt dir2/
decmp create -f archive.tar.gz -s src/ -s README.md
decmp create -f secure.zip -s data/ -p mypassword -F zip
decmp create -f archive.7z -s docs/ --level 9
```

Format (`-F`) defaults to `auto` and is detected from the output filename. In `auto` mode, single-file formats (`.gz`, `.xz`, `.zst`, `.bz2`) are treated as single-file archives — use `-F tar.gz` if you want a directory in a compressed tar.

## TUI

```bash
decmp-tui archive.zip
decmp-tui archive.tar.gz
```

| Key | Action |
|-----|--------|
| ↑ / ↓ / j / k | Navigate file list |
| Enter | Open directory / Preview file |
| Backspace | Go to parent directory |
| Esc | Back / Quit (at root) |
| Tab | Switch focus (file list / preview) |
| e | Extract selected file |
| E | Extract all files |
| p | Show file properties |
| ? | Toggle help |
| q | Quit |

The preview panel on the right shows:
- **Directories** — contents listed immediately
- **Text files** — press Enter to load; syntax highlighting via syntect (Rust, Python, Go, C, JS, CSS, YAML, SQL, TOML, Makefile, and more)
- **Binary files** — shown as non-previewable

Mouse is supported: click to select/enter, scroll wheel to navigate.

## Build from source

```bash
git clone https://github.com/anomalyco/decmp.git
cd decmp
cargo build --workspace --release
```

Requires Rust 1.85+ (edition 2024).

## Development

```bash
# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace

# Format (2-space indent)
cargo fmt --all
```

A pre-commit hook runs `fmt --check` → `clippy` → `check` on every commit.

## License

MIT — see [LICENSE](LICENSE).
