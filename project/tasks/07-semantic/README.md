# 07-semantic

Roadmap phases 8 (semantic foundation), 9 (organization) and 10 (hybrid
search). **Phase 8 started; 9 and 10 not.**

| Task | Status |
| --- | --- |
| [deterministic-analysis](01-deterministic-analysis.md) | Done — `bp-semantic` |
| [semantic-naming](02-semantic-naming.md) | Done — feeds Save As |
| [metadata](03-metadata.md) | Not started — needs `bp-storage` |
| [related-notes](04-related-notes.md) | Not started |
| [hybrid-search](05-hybrid-search.md) | Not started |

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
- **Metadata store** (`bp-storage`): documents, topics, entities, tags,
  relationships. `database/migrations/0001_initial.sql` exists but nothing
  reads it, and the schema covers governance tables rather than the document
  model in `DATA_MODEL.md`.
- **Related notes, duplicate detection, folder suggestions, semantic diff** —
  all of phase 9, all needing the metadata store first.
- **Hybrid lexical + semantic search** (phase 10) needs embeddings, which need
  a security decision before anything is indexed.
