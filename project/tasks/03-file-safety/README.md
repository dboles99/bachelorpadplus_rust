# 03-file-safety

Roadmap phase 3. **Done** for RC1; content-addressed revision history is
deferred to 1.x (release plan §3). Status checked against the code on
2026-09-25.

| Task | Status |
| --- | --- |
| atomic-save | Done — `bp-files` |
| file-watcher | Done — polling, `bp-files::watch` |
| recovery-journal | Done — `bp-history`; per-instance folders under an OS lock (M0-02), sealed for encrypted documents (ADR-0022) |
| revision-history | Deferred to 1.x. File ▸ Save as New Revision (M1-17) saves a new, suffixed file; it is not a history |

The four task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

## What exists

**Atomic save.** Temp file in the target's directory, flushed to the device,
read back, then renamed over the target. A save either fully succeeds or
leaves the previous file untouched; a test proves a failed save leaves the
original intact.

**External-change detection.** A stamp of modification time *and* length,
polled every 2 seconds. Length alone misses an edit that preserves size —
which is exactly what fixing a typo does. Deleted is reported as *Missing*,
not *Modified*: a different problem needing a different answer. Save asks
before overwriting somebody else's changes, and that refusal is reported as
"needs a path" rather than "saved", so no caller mistakes it for a write.

**Recovery journal.** Every unsaved document is checkpointed every 5 seconds
and offered back on the next launch. Since M1-08 only a document that changed
since its last checkpoint is written, and since M1-07 the write happens off
the UI thread. Checkpoints are written to a temporary
file and renamed, so a crash *during* a checkpoint leaves the previous one
intact — a half-written journal is worse than none, because it looks
recoverable. A corrupt entry is skipped rather than failing the scan.

The status bar shows specs.md section 3's real unsaved form —
`● Unsaved │ Recovery 8:05 PM │ Last disk save 8:01 PM` — and `bp-core`'s
distinct `CheckpointTime` and `DiskSaveTime` types make it structurally
impossible for a checkpoint to render as a save.

## Outstanding

- ~~The journal writes unsaved text to disk in plaintext.~~ **Done.** The
  document's security profile governs it (phase 14, ADR-0020): Standard
  writes plaintext; Private and Confidential seal each checkpoint with the
  document's passphrase (ADR-0022), so a plaintext document under those two
  gets no journal, and the status bar says so; Maximum keeps no journal.
- **The watcher polls rather than subscribing.** Deliberate: no dependency, no
  background thread pushing events into the UI loop, identical on both
  platforms, and the comparison is a pure function. A real watcher would
  replace `check`, not the decision logic around it.
- **Revision history** has not started, and is deferred to 1.x by the
  release plan (§3). Content-addressed history belongs alongside the
  metadata store in phase 9, which ADR-0030 keeps out of RC1.
