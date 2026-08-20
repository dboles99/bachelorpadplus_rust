# 05-formats

Roadmap phases 5 (format registry) and 6 (structured formats). **Started.**

| Task | Status |
| --- | --- |
| [format-registry](01-format-registry.md) | Done — `bp-formats` |
| [markdown](02-markdown.md) | Partial — outline only, via `bp-semantic` |
| [json-yaml-toml](03-json-yaml-toml.md) | JSON and TOML done; YAML not started |
| [jsonl-csv](04-jsonl-csv.md) | JSONL done; CSV reports shape only |
| [source-formats](05-source-formats.md) | Detection only — no syntax highlighting |

## What exists

**Detection** covers the 21 first-class formats from specs.md section 7,
grouped into profiles. Extension first, content second: an extension is the
user's stated intent and overriding it is how an editor ends up "helpfully"
treating a `.txt` note as source code. Sniffing is bounded to 1 KB and counts
characters rather than bytes, so it neither scales with file size nor panics
on a multi-byte boundary.

**Operations** (`bp-data`) are pure `&str -> Result<String>`: JSON validate,
format, minify, sort keys; JSON ↔ JSON Lines; TOML validate and format;
delimiter detection and a shape report for CSV/TSV.

Two rules run through all of it. Operations **refuse rather than repair** — a
format that quietly fixes as it goes would silently rewrite the user's data.
And errors carry **line and column**, because "invalid JSON" is not
actionable.

JSONL validation streams line by line and reports *every* malformed record,
not just the first: a bad export usually has a pattern, and one error at a
time hides it.

## Outstanding

- **YAML.** Needs a maintained parser; `serde_yaml` is deprecated, so this is
  a dependency decision rather than a coding task.
- **CSV with quoted fields.** The shape report splits on the delimiter and
  does not understand quoting, so a field containing a comma is miscounted.
  Documented in `bp-data`; needs a real CSV reader.
- **Markdown preview and Table of Contents.** The outline exists; rendering
  does not. Slint has no HTML view, so a preview means either drawing styled
  text ourselves or converting to something displayable.
- **Syntax highlighting.** Detection knows a file is Rust; nothing colours it.
- **Table and tree views** for tabular and structured data.
