# 03-file-safety

Roadmap phase 3. **Done.**

| Task | Status |
| --- | --- |
| [atomic-save](01-atomic-save.md) | Done — `bp-files` |
| [file-watcher](02-file-watcher.md) | Done — polling, `bp-files::watch` |
| [recovery-journal](03-recovery-journal.md) | Done — `bp-history` |
| [revision-history](04-revision-history.md) | Not started — belongs with phase 9 |

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
and offered back on the next launch. Checkpoints are written to a temporary
file and renamed, so a crash *during* a checkpoint leaves the previous one
intact — a half-written journal is worse than none, because it looks
recoverable. A corrupt entry is skipped rather than failing the scan.

The status bar shows specs.md section 3's real unsaved form —
`● Unsaved │ Recovery 8:05 PM │ Last disk save 8:01 PM` — and `bp-core`'s
distinct `CheckpointTime` and `DiskSaveTime` types make it structurally
impossible for a checkpoint to render as a save.

## Outstanding

- **The journal writes unsaved text to disk in plaintext.** Inherent to crash
  recovery, but ADR-0011 requires the document's security profile to govern
  it. `bp-history`'s module documentation names it as the place encryption
  lands. Blocked on phase 14.
- **The watcher polls rather than subscribing.** Deliberate: no dependency, no
  background thread pushing events into the UI loop, identical on both
  platforms, and the comparison is a pure function. A real watcher would
  replace `check`, not the decision logic around it.
- **Revision history** (task 04) has not started. Content-addressed history
  belongs alongside the metadata store in phase 9.
