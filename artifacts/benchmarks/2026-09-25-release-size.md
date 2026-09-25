# Release binary size -- M1-13 build hygiene

`cargo build --release -p bachelorpad`, 2026-09-25, Linux x86_64 host
(target `x86_64-unknown-linux-gnu`), rustc 1.94.1 (e408947bf 2026-03-25).
Size is `target/release/bachelorpad` in bytes.

| Configuration | Size (bytes) | Stripped? |
|---|---:|---|
| Before: cargo's default release profile (no `[profile.release]`), Slint and arboard default features | 38,281,520 | No -- debuginfo removed by cargo's default, symbol table kept |
| Before, symbol table removed afterwards with `strip` (for comparison only) | 30,522,184 | Yes (symbols) |
| After: M1-13 profile (`lto = "thin"`, `codegen-units = 1`, `debug = "line-tables-only"`, `strip = "symbols"`, `panic = "unwind"`), Slint without `system-tray`, arboard without `image-data` | 26,154,328 | Yes (symbols) |

Change: -12,127,192 bytes (-31.7 %). Of that, about 7.8 MB is the symbol
table and 4.4 MB is thin LTO, one codegen unit and the dropped features.

Both binaries pass `xvfb-run -a bachelorpad --self-check`
(`BACHELORPAD_SELF_CHECK_OK`) and print `bachelorpad 0.1.0` for `--version`.

Not measured here: the Windows (`x86_64-pc-windows-msvc`) binary, which needs
a Windows host to link. There `strip` is a no-op (rustc always links with
`/DEBUG`), the exe carries no symbols either way, and the line tables go to
`bachelorpad.pdb` beside it.
