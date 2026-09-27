# 06-search

Roadmap phase 7. **Done for RC1.** Find in Files and the editor now agree on
what a line is (D6, release-plan M1-09), and search inside the large-file
viewer is reachable (M1-06). Fuzzy search and replace in files are deferred
to 1.x (release plan §3). Status checked against the code on 2026-09-27.

| Task | Status |
| --- | --- |
| exact-regex | Done — `bp-search` |
| recursive-search | Done — `bp-search::files`, on a worker thread with a stop (M1-07) |
| replace-preview | Done — Replace All shows what it would change and asks first |
| fuzzy-search | Not started; deferred to 1.x |

The four task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

## What exists

**In-document find and replace.** Ctrl+F opens the bar, Escape closes it,
Enter steps forward, and matches are selected in the editor so it scrolls to
them. Literal by default with regex opt-in, case sensitivity, whole-word.

A literal query is escaped, so searching for `a.b` finds `a.b` and not `axb` —
that is the whole difference between a find box and a footgun. Offsets are
character indices, because they are handed straight to the widget to select
and a byte offset would land inside a multi-byte character.

**Recursive cross-file search.** "In Folder…" walks a directory and lists
hits; clicking one opens the file with the match selected. The walk runs on a
worker thread and closing the results panel stops it (M1-07).

The guards there are the substance, not an afterthought. A naive recursive
grep in a home directory reads gigabytes of `target/`, chokes on a binary, and
returns fifty thousand hits nobody scrolls through:

- build and dependency trees skipped, as is anything starting with a dot;
- files over 5 MB skipped;
- binaries detected by a NUL byte in the first 8 KB;
- symlinks not followed and depth capped, because a link pointing back up the
  tree turns a walk into an infinite one;
- the walk stops at 500 hits **and says so** — "500 results" and "at least 500
  results" mean different things to someone deciding whether to narrow their
  query.

## Outstanding

- ~~Replace preview.~~ **Done.** Replace All shows what it would change and
  asks before doing it (`bp-ui/src/dispatch.rs`).
- ~~Which characters end a line (D6).~~ **Done** (M1-09). A line ends at
  LF, CRLF or a bare CR in the buffer, `bp-search` and the viewer alike, and
  the test that was `#[ignore]`d on it is un-ignored. Line-ending
  *conversion* still treats a bare CR as data: D11 in
  `project/DECISIONS_NEEDED.md`.
- **Fuzzy search**, **search within a selection** and **replace in files**.
  Not started; fuzzy search and replace in files are deferred to 1.x.
- ~~Streaming search~~ **Done** (M1-06): Find in the large-file viewer runs
  on a background job over the file, not a copy of it in memory.
- **Performance is unmeasured.** The recursive walk is single-threaded and
  correctness-first; specs.md section 5 asks for high-performance search, and
  nothing here has been benchmarked against that.
