# BachelorPad+ Decision Log

This file is the human-readable decision register. Every row has a formal ADR
in `docs/decisions/` under the same number: `BP-ADR-0007` is `ADR-0007.md`.
Adding a decision means adding both.

| ID | Date | Decision | Status | ADR |
| --- | --- | --- | --- | --- |
| BP-ADR-0001 | 2026-08-16 | Target Windows 10, Windows 11 and Linux | Accepted | [ADR-0001](docs/decisions/ADR-0001.md) |
| BP-ADR-0002 | 2026-08-16 | Rust is the primary implementation language | Accepted | [ADR-0002](docs/decisions/ADR-0002.md) |
| BP-ADR-0003 | 2026-08-16 | Default semantic filename uses `Title_DDMMMYYYY.ext` | Accepted | [ADR-0003](docs/decisions/ADR-0003.md) |
| BP-ADR-0004 | 2026-08-16 | Creation date is the default filename date | Accepted | [ADR-0004](docs/decisions/ADR-0004.md) |
| BP-ADR-0005 | 2026-08-16 | Status bar exposes last successful disk save and full path | Accepted | [ADR-0005](docs/decisions/ADR-0005.md) |
| BP-ADR-0006 | 2026-08-16 | Core editing must work without semantic/cloud features | Accepted | [ADR-0006](docs/decisions/ADR-0006.md) |
| BP-ADR-0007 | 2026-08-16 | Add full Rust performance/safety feature set | Accepted | [ADR-0007](docs/decisions/ADR-0007.md) |
| BP-ADR-0008 | 2026-08-16 | Add structured formats and Jupyter-style notebook capability | Accepted | [ADR-0008](docs/decisions/ADR-0008.md) |
| BP-ADR-0009 | 2026-08-16 | Add Light, Dark, Organic and Green themes | Accepted | [ADR-0009](docs/decisions/ADR-0009.md) |
| BP-ADR-0010 | 2026-08-16 | Add clipboard history and format-aware clipboard behavior | Accepted | [ADR-0010](docs/decisions/ADR-0010.md) |
| BP-ADR-0011 | 2026-08-16 | Add comprehensive security and cryptography subsystem | Accepted | [ADR-0011](docs/decisions/ADR-0011.md) |
| BP-ADR-0012 | 2026-08-16 | Use OS-supported default-editor registration, never patch Notepad | Accepted | [ADR-0012](docs/decisions/ADR-0012.md) |
| BP-ADR-0013 | 2026-08-16 | Use original retro-futurist appliance branding without copying existing IP | Accepted | [ADR-0013](docs/decisions/ADR-0013.md) |
| BP-ADR-0014 | 2026-08-16 | Use project→task→step→prompts/rosettas→artifacts→metadata→database→note governance | Accepted | [ADR-0014](docs/decisions/ADR-0014.md) |
| BP-ADR-0015 | 2026-08-17 | Adopt Slint as the UI toolkit | Accepted | [ADR-0015](docs/decisions/ADR-0015.md) |
| BP-ADR-0016 | 2026-08-17 | CI is local-first; GitHub Actions is dormant | Accepted | [ADR-0016](docs/decisions/ADR-0016.md) |
| BP-ADR-0017 | 2026-08-17 | Default to the software renderer; split the startup target | Accepted | [ADR-0017](docs/decisions/ADR-0017.md) |
| BP-ADR-0018 | 2026-08-17 | Write a custom editor view; make the rope the storage; ship the view opt-in | Accepted | [ADR-0018](docs/decisions/ADR-0018.md) |
| BP-ADR-0019 | 2026-08-17 | SQLite via bundled `rusqlite`; append-only migrations; metadata never content | Accepted | [ADR-0019](docs/decisions/ADR-0019.md) |
| BP-ADR-0020 | 2026-08-19 | Security profiles resolve to a policy; named profiles are monotonic | Accepted | [ADR-0020](docs/decisions/ADR-0020.md) |
| BP-ADR-0021 | 2026-08-19 | `.bpadx`: two AEADs versioned in the envelope, chunked, position authenticated | Accepted | [ADR-0021](docs/decisions/ADR-0021.md) |
| BP-ADR-0022 | 2026-08-19 | The recovery journal is sealed with the document's passphrase and recovered at unlock | Accepted | [ADR-0022](docs/decisions/ADR-0022.md) |
| BP-ADR-0023 | 2026-08-19 | YAML uses `saphyr`; nesting, alias expansion and duplicate keys are bounded before a tree exists | Accepted | [ADR-0023](docs/decisions/ADR-0023.md) |
| BP-ADR-0024 | 2026-08-19 | An audit event is `Copy`, so it cannot carry a secret; the log's destination resolves from the existing policy axes | Accepted | [ADR-0024](docs/decisions/ADR-0024.md) |
| BP-ADR-0025 | 2026-08-19 | Notebook kinds ride in namespaced `.ipynb` metadata; a run needs a `UserGesture` no parsed file can produce | Accepted | [ADR-0025](docs/decisions/ADR-0025.md) |
| BP-ADR-0026 | 2026-08-19 | A `.sig` sidecar appends to the whole file name and records the signer's key; Windows enforces no key-file permission | Accepted | [ADR-0026](docs/decisions/ADR-0026.md) |
| BP-ADR-0027 | 2026-08-19 | Large-file thresholds are measured; the line index is sparse; memory mapping is declined with reasons | Accepted | [ADR-0027](docs/decisions/ADR-0027.md) |
| BP-ADR-0028 | 2026-08-19 | Redaction merges overlapping and adjacent spans, resolves every offset against the original, and states what verification cannot prove | Accepted | [ADR-0028](docs/decisions/ADR-0028.md) |
| BP-ADR-0029 | 2026-09-25 | The recent list, recovery journal and security history leave the roaming directory, and an old location is adopted -- or kept -- rather than abandoned | Accepted | [ADR-0029](docs/decisions/ADR-0029.md) |
| BP-ADR-0030 | 2026-09-25 | RC1 scope: notebook execution, semantic organisation, personality modes and highlighting deferred; mmap declined for good; Sign ships with a sealed seed | Accepted | [ADR-0030](docs/decisions/ADR-0030.md) |
| BP-ADR-0031 | 2026-09-25 | Slint is used under its Royalty-free Licence 2.0, with `AboutSlint` in the About window | Accepted | [ADR-0031](docs/decisions/ADR-0031.md) |
| BP-ADR-0032 | 2026-09-25 | App id `io.github.dboles99.BachelorPad`; the installer owns Windows registration; DejaVu Sans Mono is bundled | Accepted | [ADR-0032](docs/decisions/ADR-0032.md) |
| BP-ADR-0033 | 2026-09-25 | `bp-platform-windows` is the one crate allowed `windows-sys` and confined `unsafe` | Accepted | [ADR-0033](docs/decisions/ADR-0033.md) |
| BP-ADR-0034 | 2026-09-26 | Nothing leaves the machine in v1; `Network::Allowed` means "not forbidden, and no consumer exists" | Accepted | [ADR-0034](docs/decisions/ADR-0034.md) |

## Decisions needed before the work they block

The one list of open questions is
[`project/DECISIONS_NEEDED.md`](project/DECISIONS_NEEDED.md). It is kept
current as work happens, and an answered row moves from there into an ADR and
the table above. This section used to carry its own subset, which went stale
the moment the queue moved; it no longer duplicates it.
[`project/RELEASE_PLAN.md`](project/RELEASE_PLAN.md) §4 orders the same
questions by the milestone each one blocks. Of that list, decisions 1, 2, 4,
5, 6, 7 and 16 were answered on 2026-09-25 (ADR-0030 to ADR-0033), which
settles D5, D8, D9 and U2. On 2026-09-26 the owner answered 17 (nothing
leaves the machine, ADR-0034, settling D1), 18 (package formats), 19
(releases built by a local script), 20 (RC1 ships saying the encryption
envelope is unreviewed, settling D4 for RC1) and decision 2's copyright
holder (Daniel Boles). Later on 2026-09-26 the owner answered 3 (Azure
Trusted Signing, since renamed Artifact Signing), 15 and D10 (ask above four
times the default Argon2 cost, refuse above 256 MiB or 8 passes), 21 (the
Windows 10 support statement), U3 (a read-only viewer tab for large files),
U5 (clipboard rows snapshot on open), D6 (a line ends at LF, CRLF or a bare
CR, everywhere) and approved the icon. What decision 3 still lacks is the
signing identity itself, which the owner has yet to obtain. Two questions
were added on 2026-09-27: D11 (whether line-ending conversion should rewrite
bare CRs) and 3a (whether the owner can get Artifact Signing or needs an OV
certificate).

specs.md section 22's warm-start target (75 ms) is a measurement still owed,
not a decision; it is under *Open items* below with the rest of ADR-0017's
unmeasured half.

## Open items

- **Fixed in the sixth session, and worth knowing how.** Two of the
  cross-crate tests' defects: `bp-notebook` ran a cell whose stated language
  it could not map through the kernel's interpreter (it now falls to `Raw`
  with a warning, as ADR-0025 said all along), and `bp-files` never applied
  `\\?\`. The second is the instructive one. `std::fs` *does* lengthen a
  long path on Windows by itself, which is why creating and reading such a
  file always worked; `tempfile`'s `persist` calls `MoveFileExW` with the path
  as given, so only the rename -- the step that makes a save atomic -- failed.
  A dependency that bypasses `std` for one call inherits none of `std`'s
  platform fixes for that call.
- **"A flipped bit anywhere in a `.sig` never verifies" is false, correctly.**
  `Sidecar::parse` tolerates trailing whitespace so a signature survives being
  mailed and pasted, so flipping bit 0 of the closing newline yields a
  vertical tab and the file still verifies -- the document really is
  unaltered and the signature really does hold. The property with teeth, and
  the one now tested over every byte and every bit, is that a damaged sidecar
  verifies *if and only if* it parses back to the identical key and
  signature. Recorded because the naive version is the one somebody will
  write next.

- **~~The recent-files list roams on Windows~~ -- settled by ADR-0029.**
  The list, the recovery journal and the security history all leave the
  configuration directory together, and an old location is adopted rather
  than abandoned. The journal went to `Data`, not `State`, which is what
  `bp-platform`'s own docs said.
- **`bp-platform-windows` was created late, and `bp-platform-linux` not at
  all.** At first neither was, and that was a decision rather than an
  omission. specs §20 and the crate map
  name them. Everything they would hold is either already a parameterised
  function in `bp-platform` -- tested on both legs -- or blocked on D8 and
  the signing-key question. ADR-0001's own warning about `cfg`-gated code
  argues against two crates whose contents each CI leg never compiles.
  Revisit when something genuinely platform-specific has to be linked.
  Since then D8 and the signing-key question are answered, and ADR-0033
  (2026-09-25) names `bp-platform-windows` as the one crate allowed
  `windows-sys` and confined `unsafe`, for when Win32 calls are needed. It
  now exists and is wired into the application (release plan M2-03): the
  parent console for `--help`, the crash message box, and the default-editor
  report and Settings link. None of it has yet been run on Windows.
  `bp-platform-linux` has still not been created.

- **What a line is: answered (D6, 2026-09-26), and the pinned test runs.**
  `bp-search` used to count `'\n'` while `bp-buffer`'s rope, built with
  ropey's default `unicode_lines`, also broke on a bare `\r`, `\x0B`,
  `\x0C`, U+0085, U+2028 and U+2029, so a find-in-files result scrolled the
  editor to the wrong line in any file carrying one. The owner chose the
  Notepad rule: a line ends at LF, CRLF or a bare CR, and nothing else,
  everywhere. The rope is built with `cr_lines`; `bp_buffer::breaks` states
  the rule for the rope, `line_count`, the large-file index and viewer and
  the stream windows; and `bp-search`, `bp-secrets`, `bp-semantic` and
  `bp-redaction`, which do not depend on `bp-buffer`, restate it and are
  held to it by `tests/integration/tests/search_over_buffer.rs`. Three
  places still see lines their own way, each deliberately: the default
  `TextInput` view draws breaks as Slint does (it may draw a form feed or
  U+2028 as a new line); `bp-files` converts line endings without touching
  a bare `\r`, because `LineEnding` cannot name a classic Mac file and a
  conversion that rewrote one would rewrite a `\r` that was data (whether it
  should is open question D11); and the
  JSON, YAML, TOML and CSV parsers report line numbers by their own rules.
- **Time to first interaction is unmeasurable under the software renderer.**
  Slint exposes no rendering notifier there, so half of the ADR-0017 startup
  target has no measurement behind it. A hole, not a pass.
- **Scroll smoothness under CPU rasterisation is unmeasured**, and needs a
  capture rig. This is the standing revert condition on ADR-0017. The
  application's own typing-path latency *is* now measured and is not a
  problem: 376 µs p50 at 1 MB against a ~16 ms frame budget.
- **Comparative startup timing on Linux is unmeasured.** The shell builds and
  runs there, but WSLg's compositor makes timing unrepresentative. Needs a
  native Linux machine.
- ~~Keyboard shortcut delivery is unverified.~~ **Confirmed working**
  (2026-08-17, manual test). `KeyBinding` in a wrapping `FocusScope` matches
  during the capture phase, so the focused `TextInput` no longer swallows
  Ctrl+S.
- **The three dependants now read the security profile.** The recovery
  journal refuses rather than writing plaintext, and deletes what a looser
  profile already wrote; clipboard history stops recording and is cleared;
  and `bp-storage`'s `record_document` drops the title under `PathOnly` and
  records nothing under `Disabled`. Privacy Mode clamps all of it for the
  session without replacing the document's profile, so leaving it restores
  what the document had rather than the default.

  What ADR-0020 required to fail loudly no longer has to. The two profiles
  wanting an encrypted recovery journal are honoured, because `bp-crypto`
  ships and ADR-0022 seals the journal with the document's own passphrase.
  The remaining hole is narrower, and is the next item: a *plaintext*
  document under those profiles has no key, so it gets no journal at all.
- **The custom editor view has been driven end to end, but not by a
  person.** Its rules are covered by some 200 tests in `bp-editor`. The sixth
  session drove the binary under `--editor-view` on a virtual X display with
  synthetic input: keys arrive, Ctrl+S saves every edit, Home/End/Page
  Up/Page Down land where they should, and Ln/Col tracks the caret. The Linux
  caret drift it found (a proportional fallback for "Consolas") was fixed by
  bundling DejaVu Sans Mono (ADR-0032, M0-03). Whether the caret lands under
  the pointer since that fix, and which way the wheel scrolls, are **not yet
  verified**: the headless window tests send keys, not pointer placement, and
  no person has run the checklist in `project/NEXT_SESSION.md`.

  This used to say that caret features depended on the custom view's answer.
  **That is no longer true.** Go to Line shipped (W5), and since release-plan
  item M1-04 it, Duplicate Line, Move Line Up/Down, Insert Date/Time and
  Ln/Col all work in the default `TextInput` view too, which reads the
  widget's caret. The default is still `TextInput`, so a bad answer here
  affects only those who opt in.
- **A clipboard menu row could act on a different entry than the one it
  named. Fixed** (release-plan item M1-18, U5 answered "snapshot on open").
  Row ids were decoded against the history as it stood at the click, and the
  1.2 s poll could add an entry between the menu being built and the click.
  Now opening Edit or the editor context menu freezes a snapshot of the
  history as one fingerprint per entry (`bp_clipboard::Snapshot`, hashes
  only, so it keeps no text the history has forgotten). A row's id is a slot
  in that snapshot, and a click finds the entry the slot names wherever the
  history has moved it; if it has gone, the status bar says so and nothing is
  pasted, pinned or cleared. A copy made while a menu is open appears the
  next time one opens. What is left: an entry *removed* while a menu is open
  (a stricter profile clearing the history) takes its row with it, so the
  rows below move up. That is deliberate -- the alternative is keeping a
  preview of forgotten text on screen -- and every row still acts on the
  entry it shows.
- **Most of the product has not been used.** Over 2,000 tests cover the
  pieces and, since the window tests, some of the seams. One manual pass
  found two defects no test caught: a menu bar where twelve of fourteen menus
  swallowed clicks, and Save As defaulting to the process working directory,
  which wrote real documents into a git checkout. Integrated behaviour needs
  exercising, not more unit tests.
