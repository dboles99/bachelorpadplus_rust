# 04-large-files

Roadmap phase 4. **Partly done: the engine is built and tested, and no user
can reach it yet.** Status checked against the code on 2026-09-25.

| Task | Status |
| --- | --- |
| mmap-reader | Declined for good: chunked reads instead (ADR-0027; ADR-0030, S3). Memory mapping would need `unsafe` |
| streaming-search | Built: `bp_buffer::WindowReader` (`bp-buffer/src/stream.rs`). Not reachable until the viewer lands |
| large-file-ui | Not built. Release-plan item M1-06 (read-only viewer tab with a banner, `--readonly`, `--large-file`), waiting on the owner's answer to U3 |

The three task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

What is reachable today: `bp_files::load` refuses a file past the 32 MiB
editable limit before reading it (`LoadError::TooLarge`, M0-04), and the
status bar says why; a file between 8 MiB and that limit opens with a notice
that whole-document tools may be slow (`SizeClass::Large`).
`bp_buffer::LargeFile` (`large.rs`) is used nowhere outside its own crate.
