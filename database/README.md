# Database

**Nothing reads this directory.** It is the repository scaffold's
governance schema from 2026-08-16: `migrations/0001_initial.sql` defines
`projects`, `tasks`, `artifacts` and `notes` tables, the "database" layer of
ADR-0014's work model. No script, test or crate opens it, and no database
built from it exists in the repository.

It is **not** the application's metadata store. That is `bp-storage`
(ADR-0019): SQLite with its migrations embedded in
`crates/bp-storage/src/migrations.rs` (documents, tags and the join between
them), built and tested, and called by nothing in the application -- kept
unwired for RC1 by ADR-0030 (question D3 in
`project/DECISIONS_NEEDED.md`).

The rules this file used to list are kept by `bp-storage`, not here:

- migrations are append-only and numbered by their position; an applied one
  is never edited;
- no schema change without a migration;
- what is stored is governed by the document's security profile (ADR-0020);
- no plaintext secret fixtures;
- tests use disposable databases.
