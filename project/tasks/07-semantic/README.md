# 07-semantic

Roadmap phases 8 (semantic foundation), 9 (organization) and 10 (hybrid
search). **Phase 8 done for RC1; phase 9 built as far as a metadata store and
kept unwired; phase 10 not started.** ADR-0030 (S2) defers 9 and 10 to 1.2,
and D3 in `project/DECISIONS_NEEDED.md` holds the question of wiring
`bp-storage`. Status checked against the code on 2026-09-25.

| Task | Status |
| --- | --- |
| deterministic-analysis | Done — `bp-semantic`: title, summary, keywords, outline, statistics |
| semantic-naming | Done — feeds Save As and Note ▸ Semantic Rename |
| metadata | `bp-storage` is built and tested (SQLite, its own migrations) and nothing in the application calls it: ADR-0019, D3, and ADR-0030 for RC1 |
| related-notes | Not started; 1.2 (ADR-0030, S2) |
| hybrid-search | Not started; 1.2 (ADR-0030, S2) |

The five task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

## What exists

Layer one of specs.md section 10, and **only** layer one: title, keywords,
summary and outline, each a pure function of the text in front of it.

That constraint is ADR-0006, not modesty. Core behaviour has to work with
every semantic and cloud feature disabled, so naming a note cannot depend on a
model being loaded or a network being up. It also means every rule is testable
and every answer explainable — the user can see *why* their note got that
title.

- **Title** from the first Markdown heading, else the first non-empty line,
  with comment and quote markers stripped. Truncates on a word boundary, and a
  truncated title is still a prefix of its source. An empty document has no
  title; inventing one would be worse than admitting there is none.
- **Outline** from ATX headings. `#hashtag` is not a heading — the space is
  what makes it one.
- **Keywords** by frequency, minus stopwords and bare numbers, sorted
  deterministically so equal frequencies do not reorder between runs.
- **Summary** is the first paragraph rather than the first N characters,
  because cutting mid-sentence produces something that reads like a fault.

**Semantic naming** is wired: Save As proposes a filename built from the
document's own content in the ADR-0003 grammar, and Note ▸ Semantic Rename
routes through the save dialog — a physical rename needs explicit approval
(PROJECT_MEMORY), and a file dialog is that approval.

## Outstanding

- **Layers two and three** — local statistical models and optional generative
  providers. Both must stay optional (ADR-0006), and layer three needs a
  decision about which providers and what leaves the machine (ADR-0011).
- **Metadata store.** `bp-storage` exists: documents and tags, with its
  migrations embedded in `crates/bp-storage/src/migrations.rs`. Topics,
  entities and relationships from `DATA_MODEL.md` are not in it. Nothing in
  the application calls it (D3; ADR-0030 keeps it out of RC1).
  `database/migrations/0001_initial.sql` is a different, older schema that
  nothing reads (see `database/README.md`).
- **Related notes, duplicate detection, folder suggestions, semantic diff** —
  all of phase 9, all needing the metadata store wired first. Deferred to 1.2
  (ADR-0030, S2).
- **Hybrid lexical + semantic search** (phase 10) needs embeddings, which need
  a security decision before anything is indexed (D1). Deferred to 1.2
  (ADR-0030, S2).
