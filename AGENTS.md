## Pre-commit hook

`.git/hooks/pre-commit` runs: `cargo fmt --check` → `cargo clippy --workspace` → `cargo check --workspace`. All must pass.

## Code style

`rustfmt.toml` sets `tab_spaces = 2`. Run `cargo fmt --all` before committing.

## Build & Test

```bash
# Build all crates
cargo build --workspace

# Run all tests (unit + integration)
cargo test --workspace

# Run only the core library tests
cargo test -p decmp-core

# Run only the CLI integration tests (shell-created archives)
cargo test -p decmp --test integration

# Run only TUI tests
cargo test -p decmp-tui

# Lint (clippy must pass with zero warnings)
cargo clippy --workspace

# Format (2-space indent)
cargo fmt --all
```

## Workspace structure

```
crates/
├── decmp-core/      # Library: ArchiveHandler trait + 7 format handlers
├── decmp/           # CLI binary (clap)
└── decmp-tui/       # TUI binary (ratatui + crossterm + syntect)
```

Three crates, two binaries. `cargo run` needs `--bin decmp` or `--bin decmp-tui`.

## Key architecture

### decmp-core

`ArchiveHandler` trait in `crates/decmp-core/src/archive/mod.rs`:

```rust
fn list(&self, path, password, encoding) -> Result<Vec<ArchiveEntry>>;
fn extract(&self, path, dest, password, encoding) -> Result<()>;
fn extract_entries(&self, path, entries, dest, password, encoding) -> Result<()>;
fn read_entry(&self, path, entry_name, password, encoding) -> Result<Vec<u8>>;
fn create(&self, sources, dest, password, level) -> Result<()>;
```

7 handlers: Zip, SevenZ, Tar (handles tar.gz/xz/zst/bz2/lzma), Gzip, Zstd, Xz, Bzip2.

Single-file formats (Gzip/Zstd/Xz/Bzip2) only compress one file — directories need tar.* variants.

`ArchiveEntry` fields: `name`, `size`, `compressed_size`, `is_dir`, `method`, `modified: Option<String>`.

`Password::from(&str)` for sevenz-rust (not `Password::new()`).

Entry names with `./` prefix (common in tar archives) are normalized at three levels:
- `normalize_entry_names()` in `app/mod.rs` strips from `ArchiveEntry.name` on load
- `DirTree::from_entries()` in `tree.rs` strips `./` before trimming trailing `/`
- Tar handler `extract_entries`/`read_entry` strip `./` from decoded archive names before comparison

### decmp-tui

`App` struct uses sub-structs for grouping:

| Sub-struct | Fields |
|---|---|
| `ArchiveState` | `path`, `handler`, `entries`, `tree` |
| `NavState` | `current_path`, `list_state`, `focus` |
| `PreviewState` | `content`, `scroll`, `horizontal_scroll`, `cache`, `scrollbar`, `area` |

Remaining fields on `App`: `mode`, `password`, `password_input`, `extract_dest_input`, `status_msg`, `should_quit`, `properties_entry`, `help_scroll`, `file_list_area`, `file_list_scroll`, `pending_extract_entries`, `pending_action`.

Code split into `src/app/` — `mod.rs` + `navigation.rs`, `preview.rs`, `extract.rs`, `password.rs`, `scroll.rs`. Each is a separate `impl App` block.

Mode enum: Browse/Password/ExtractDest/Properties/Help (no Preview mode — preview is inline in the right panel).

### Syntax highlighting

`src/highlight.rs` uses **syntect** (v5) with bundled Sublime Text grammars and Base16 Ocean Dark theme. Lazy-initialized globals (`LazyLock<SyntaxSet>`, `LazyLock<ThemeSet>`). Language detected by file extension via `find_syntax_by_extension()`. Highlighting runs once when a file is previewed; spans are cached in `SidePreview.highlighted`.

### CLI argument handling

Format detection is purely filename-based (no magic bytes). `detect_format()` in `archive/mod.rs`.

## Common gotchas

- `cargo run` in workspace root needs `--bin decmp` or `--bin decmp-tui` — there are two binaries.
- ZIP encryption: `entry.by_index_decrypt(i, pw.as_bytes())` but `options.with_aes_encryption(AesMode::Aes256, pw)` takes `&str`, not `&[u8]`. Inconsistent API across the zip crate.
- The `sevenz-rust` `FileTime(u64)` field `0` is **private** — no public way to extract raw timestamp.
- TUI password input has no horizontal scroll — just `"*"` repeated, truncated with `<` prefix if overlong.
- TUI preview for files is lazy: Enter loads it, clicking selects but doesn't load until Enter is pressed. Directory contents show automatically in the right panel.
- Rust edition 2024 changes pattern binding: `let Some((name, _)) = expr` may give `&str` instead of `String` in some contexts. Use `&name == ".."` or store the value explicitly.
- No `#[allow(dead_code)]` annotations anywhere — dead code should be removed, not suppressed.
- `rustfmt.toml` enforces 2-space indent. Hook enforces `cargo fmt --check`, `cargo clippy`, `cargo check` on commit.
