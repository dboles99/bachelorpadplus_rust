# 05-formats

Roadmap phases 5 (format registry) and 6 (structured formats). **Done for
RC1.** Syntax highlighting is deferred to 1.x (ADR-0030, S6) and Markdown
preview waits on U4, which the release plan recommends deferring. Status
checked against the code on 2026-09-25.

| Task | Status |
| --- | --- |
| format-registry | Done — `bp-formats` |
| markdown | Outline only, via `bp-semantic`; fenced and indented code kept out of it (M1-21). Preview and TOC: U4 |
| json-yaml-toml | Done. JSON keeps the author's key order, Sort Keys is its own action, TOML Format warns (M0-05); YAML via `saphyr` (ADR-0023) |
| jsonl-csv | Done. RFC 4180 CSV/TSV shape report, conversion to JSON and JSON Lines, column types |
| source-formats | Detection and the format label only — no syntax highlighting (ADR-0030, S6: 1.x) |

The five task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

## What exists

**Detection** covers the 21 first-class formats from specs.md section 7,
grouped into profiles. Extension first, content second: an extension is the
user's stated intent and overriding it is how an editor ends up "helpfully"
treating a `.txt` note as source code. Sniffing is bounded to 1 KB and counts
characters rather than bytes, so it neither scales with file size nor panics
on a multi-byte boundary.

**Operations** (`bp-data`) are pure `&str -> Result<String>`: JSON validate,
format, minify, sort keys; JSON ↔ JSON Lines; TOML validate and format; YAML
validate, format, minify and conversion to and from JSON; delimiter
detection, a shape report, column types and conversion to JSON or JSON Lines
for CSV/TSV. The Data menu reaches all of them.

Two rules run through all of it. Operations **refuse rather than repair** — a
format that quietly fixes as it goes would silently rewrite the user's data.
And errors carry **line and column**, because "invalid JSON" is not
actionable.

JSONL validation streams line by line and reports *every* malformed record,
not just the first: a bad export usually has a pattern, and one error at a
time hides it.

## Outstanding

- ~~YAML~~ **Done** (ADR-0023 chose `saphyr`). Sort keys and an outline for
  YAML are not built; YAML sort keys is deferred to 1.x (release plan §3).
- ~~CSV with quoted fields~~ **Done.** `bp_data::delimited_rows` honours
  RFC 4180 quoting.
- **Markdown preview and Table of Contents.** The outline exists; rendering
  does not. Slint has no HTML view. This is question U4 in
  `project/DECISIONS_NEEDED.md`.
- **Syntax highlighting.** Detection knows a file is Rust; nothing colours it.
  Deferred to 1.x (ADR-0030, S6).
- **Table and tree views** for tabular and structured data. Deferred to 1.x
  (release plan §3).
- **Windows-1252** (M1-10) opens and saves legacy files; it has not yet been
  run on Windows.
