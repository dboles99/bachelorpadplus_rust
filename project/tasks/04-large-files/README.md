# 04-large-files

Roadmap phase 4. **Done for RC1: the viewer is built and reachable**
(release-plan M1-06). What remains is measuring a 1 GB file in a release
build (M4-02), and trying it on Windows. Status checked against the code on
2026-09-27.

| Task | Status |
| --- | --- |
| mmap-reader | Declined for good: chunked reads instead (ADR-0027; ADR-0030, S3). Memory mapping would need `unsafe` |
| streaming-search | Done: Find in the viewer runs on a background job (M1-06). Find Previous reads forwards to the caret, so it is slower |
| large-file-ui | Done (M1-06, U3 answered): a read-only viewer tab with a banner giving both sizes; `--readonly` and `--large-file` open it |

The three task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

What is reachable today: a file past the 32 MiB editable limit opens in the
viewer (`ui/viewer.slint`, `bp-ui/src/viewer.rs`, over `bp_buffer::LargeFile`)
instead of being refused as it was after M0-04. The viewer has no buffer, so
no save, journal or data-conversion path can reach the file. Lines are
indexed on a background job. Its limits, all stated in the user guide: no
selection or copy, no UTF-16, and 64 KiB per displayed line. A file between
8 MiB and the limit opens editable, with a notice that whole-document tools
may be slow (`SizeClass::Large`).
