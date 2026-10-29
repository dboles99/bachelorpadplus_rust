# 06-search

Roadmap phase 7. **Started.**

| Task | Status |
| --- | --- |
| [exact-regex](01-exact-regex.md) | Done — `bp-search` |
| [recursive-search](02-recursive-search.md) | Done — `bp-search::files` |
| [replace-preview](03-replace-preview.md) | Replace All done; preview not started |
| [fuzzy-search](04-fuzzy-search.md) | Not started |

## What exists

**In-document find and replace.** Ctrl+F opens the bar, Escape closes it,
Enter steps forward, and matches are selected in the editor so it scrolls to
them. Literal by default with regex opt-in, case sensitivity, whole-word.

A literal query is escaped, so searching for `a.b` finds `a.b` and not `axb` —
that is the whole difference between a find box and a footgun. Offsets are
character indices, because they are handed straight to the widget to select
and a byte offset would land inside a multi-byte character.

**Recursive cross-file search.** "In Folder…" walks a directory and lists
hits; clicking one opens the file with the match selected.

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

- **Fuzzy search.** Not started.
- **Search within a selection**, and **replace in files** across a directory.
- **Streaming search** for files too large to hold in memory. Belongs with the
  large-file engine in phase 4.
- **Performance is unmeasured.** The recursive walk is single-threaded and
  correctness-first; specs.md section 5 asks for high-performance search, and
  nothing here has been benchmarked against that.
