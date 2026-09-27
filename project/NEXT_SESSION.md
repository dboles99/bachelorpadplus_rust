# Next session plan

Rewritten 2026-09-27, at the end of the seventh build session. Everything
below is on `main` (merged as `65a13c3`, PR #3). The history of how it got
there is in `git log` and in the dated "Progress" notes under each milestone
of [`RELEASE_PLAN.md`](RELEASE_PLAN.md); this file keeps only what the next
session needs.

## Where things stand

25 crates, **2,313 tests passing on Linux** (one ignored on purpose: the
fuzz workspace's cost measurement, which pays a 256 MiB Argon2 derivation).
The version is `1.0.0-rc.0`, used for release dry runs only; nothing has
been published. **Nothing built since the sixth session has run on
Windows.**

| Milestone | State |
| --- | --- |
| M0: data safety, privacy, honesty | Done, except M0-12, the Windows gate run |
| M1: the shipping view, an honest gate | Built. Its exit also needs the Windows gate and a 1 GB file opened in the viewer |
| M2: platform integration | Done in code. The Windows half (M2-02/03/04) has never run on Windows |
| M3: packaging and the release pipeline | Done as far as this machine can go. Waiting: a Windows dry run, a Debian 12 container build, Fedora and Arch package tests, a signing identity |
| M4: evidence on each OS | Not started |
| M5: RC1 per OS | Not started |

Landed in the seventh session, in outline: in-app dialogs, About and
Settings; Ln/Col and the line moves in the default view; the large-file
read-only viewer (`--readonly`, `--large-file`); the D6 line rule (LF, CRLF
and a bare CR end a line everywhere); the editor context menu with
snapshot-on-open clipboard rows (U5); the Argon2 cost cap (decision 15);
nothing leaves the machine (ADR-0034); Windows resources, the `.reg` pair
and the Settings deep link; Linux Set as Default; the approved icon;
licences and generated third-party notices; `cargo deny` in the gate; the
Inno Setup installer, the portable zip, `.deb`, `.rpm`, `tar.gz` and
`PKGBUILD`; `scripts/Invoke-Release.ps1` with a recorded Linux dry run; the
signing step (wired, never run); and the user guide, CHANGELOG, SECURITY
and the RC checklist.

## Before the next session: the owner, on the Lenovo

[`docs/release/WINDOWS_BRINGUP.md`](../docs/release/WINDOWS_BRINGUP.md) is the
run-book: install the tools, run the gate, use the app for ten minutes, run
the release dry run and `Test-Install.ps1`, and bring the results back. Also
waiting on the owner, all listed in [`DECISIONS_NEEDED.md`](DECISIONS_NEEDED.md):

- **3a, the signing identity.** Azure Artifact Signing takes individual
  developers in the USA and Canada only; an OV certificate is the fallback,
  and `Signing.ps1` already takes either.
- **D11, bare CRs in line-ending conversion.** Recommended: yes, rewrite
  them, and label a CR-only file "CR".
- **The package mirrors.** Allow `deb.debian.org`, `security.debian.org`,
  `dl.fedoraproject.org`, `mirrors.fedoraproject.org`,
  `geo.mirror.pkgbuild.com` and `archlinux.org` in this environment's
  network settings.

## The next session, in order

1. **Start clean.** The session branch's pull request is merged, so restart
   the branch from `main`: `git fetch origin main && git checkout -B
   <branch> origin/main`. Check the disk (`df -h /`); `target/` from an
   earlier session can be 15 GB.
2. **Triage whatever the Windows run brought back.** A failing gate stage
   comes before everything else, because every other item assumes Windows
   builds and runs. Commit the Windows dry run's `manifest.json` and
   `SHA256SUMS` beside the Linux ones. M0-12 closes when the gate is green
   on Windows.
3. **If the mirrors are open** (check with `curl -sI https://deb.debian.org`):
   `pwsh scripts/Invoke-Release.ps1 -Tag v1.0.0-rc.0 -DryRun -Container`,
   which is also the first build of the pinned Debian 12 image, then
   `packaging/linux/test-packages.sh` on Fedora and Arch. Commit the
   record; the manifest's glibc figure should drop to 2.36 or below.
4. **Fix before rc.1:** a `.bpadx` opened from Recent Files, the command
   line or a Find in Files hit goes through `AppState::open` and never gets
   the passphrase flow. Route them through `open_maybe_encrypted`
   (`crates/bp-ui/src/state.rs`), with a test per entry point.
5. **D11, once answered:** a `LineEnding::Cr` in `bp-files`, conversion that
   rewrites bare CRs, and the status bar label.
6. **M4's automatable parts** (plan §5, M4):
   - M4-01 to M4-03: startup, open-time, typing latency and idle memory
     on Linux, and a 1 GB file in the viewer (M1-06's exit check);
   - M4-05: a coverage-guided fuzz run, if nightly and crates.io are
     reachable;
   - M4-06: `docs/security/THREAT_MODEL.md`.
7. **Smaller follow-ups** in `WORK_QUEUE.md` "Follow-ups from M3": the
   context menu ignores the theme; whether the default `TextInput` view
   breaks lines at U+2028 or a form feed; the `artifacts/test-evidence`
   ignore rule against the RC checklist's layout; `SLINT_BACKEND=winit-x11`
   in risk R8.
8. **When a signing identity exists:** the first `-Sign` run on Windows
   (M3-04), then M3-08.
9. **Then M5:** cut `1.0.0-rc.1` (the version, the CHANGELOG heading, the
   release notes with the Windows 10 statement) once M4's evidence is in.

Not for RC1, and not to be started: notebook execution, semantic
organisation (`bp-storage`, `bp-organize`), personality modes, syntax
highlighting, Markdown preview (ADR-0030; U4 and D3 are open for 1.x).

## Working with agents in this repository

Seven sessions of evidence:

- **Each agent gets its own worktree, cut from the working branch's HEAD**
  (not `main`; `isolation: worktree` cuts from `main`), and its own
  `CARGO_TARGET_DIR`. Sharing one target directory between trees makes cargo
  reuse one tree's build of a workspace crate in another. It also bakes a
  deleted worktree's path into test binaries through `CARGO_MANIFEST_DIR`:
  after M1-18 the gate failed two drift tests looking for files in a
  worktree that no longer existed. If a shared target is unavoidable
  because disk is short, `touch` every `.rs` file in the workspace before
  the gate.
- **The container can restart and kill a running agent.** Its uncommitted
  work survives in the worktree. Tell agents to commit each coherent slice
  as they go, and when one is lost, resume from its diff.
- **One crate per agent; serialise anything touching `bp-ui/src/state.rs`
  or `ui/app.slint`.** Two agents in parallel on different crates merged
  cleanly all session; the conflicts were always in `state.rs`.
- **Review the substance, not the report.** Merging the viewer and the line
  rule found a column tracker that counted only `\n`, which neither agent
  could have seen alone.
- **Ask for a test that fails without the fix**, and a property where one
  exists. Every defect found by review was found that way.
- **Tell agents to report problems outside their scope rather than fix
  them**, and keep `scripts/Invoke-LocalCI.ps1` changes in their own commit.
- **Code taking a `Platform` parameter must not use `std::path` to split
  anything.** Use `bp_platform::paths::file_name` and `paths::join`; this has
  been sprung twice.
- **Disk:** a release build is about 12 GB and the full debug target about
  15 GB. Build with `CARGO_INCREMENTAL=0`, and `CARGO_PROFILE_DEV_DEBUG=0`
  when two builds share the disk.

## The `--editor-view` checklist, partly answered by a machine

```powershell
cargo run --release -- --editor-view
```

A person has still not done this. The sixth session drove the binary on a
virtual X display with synthetic input, which answers some rows and cannot
answer the rest.

| Check | What would be wrong | Sixth session, Xvfb, Linux |
| --- | --- | --- |
| Type a character | Keys not reaching the `FocusScope` | **Works** |
| Click mid-line | Caret left or right of the pointer: the measured advance is off | Was wrong on Linux (proportional fallback font). The bundled DejaVu Sans Mono (ADR-0032) removes the cause; **not re-checked** |
| Click at the end of a long line | Advance error accumulating | Same cause, same fix; **not re-checked** |
| Arrow down through a short line and back | The goal column lost | Page Down kept the column; the round trip not run |
| Drag to select | Selection misplaced | Not run |
| Scroll the wheel | Wrong direction, or snapping back to the caret | Not run |
| Ctrl+S | The surface swallowing chords | **Works** |
| Ctrl+D, Alt+Up, Alt+Down | Duplicate and move line | Covered by window tests since; not run by hand |
| Right-click, Menu key | The context menu (M1-18) | **Works** under Xvfb in both views |
| Status bar | Ln/Col tracking the caret | **Works**, shows `OVR` |
| A tab-indented file | Tab stops wrong | Not run |
| Resize the window | Row count not following | Not run |
| Home, End, Page Up/Down, Ctrl+Home/End, Insert | Keys unnamed or landing wrongly | **Works** |

Write down what feels wrong, not only what breaks. The default is still
`TextInput`, so nothing is lost if the answer is bad.

## Housekeeping still open

- **Native Linux timing.** WSLg's compositor makes startup numbers
  unrepresentative; ADR-0015's evidence is Windows-only. M4-01 is the fix.
- **Scroll smoothness under CPU rasterisation** is ADR-0017's standing
  revert condition and needs a capture rig.
- **Idle memory drifted from ~20 MB to ~32 MB** after dialogs, observed
  twice, never investigated. M1-01 replaced those dialogs; M4-03 measures.
- **`expect_used`, `unreachable` and `indexing_slicing`** are not linted.
  Each is a code change rather than a policy change and wants its own pass.
- **`fuzz/corpus/yaml/` crashes editor tooling** that scans for `*.yaml`
  (Google's Cloud Code YAML server). Disable that extension for this
  workspace; details in `fuzz/README.md`.

## How to work in this repository

- `./scripts/Invoke-LocalCI.ps1` is the gate: on Windows add `-Linux` for
  the WSL leg; on Linux run `pwsh -NoProfile -File
  scripts/Invoke-LocalCI.ps1`. There is no hosted CI (ADR-0016):
  `.github/workflows/ci.yml` runs only on `workflow_dispatch`, and runs the
  gate script itself.
- `./scripts/Install-GitHooks.ps1` once per clone; the hooks run the gate.
- Releases are made by `scripts/Invoke-Release.ps1` on the owner's machines
  (decision 19, the ADR-0016 amendment), never by CI.
- `docs/decisions/` is the register (34 ADRs); add an ADR when a decision has
  consequences beyond one file. `project/DECISIONS_NEEDED.md` is the queue of
  questions only the owner can answer: ask them all at once, at the start.
- **Do not edit files while the gate is running.** The formatting stage
  re-reads the tree, and a mid-run edit fails it for no reason connected to
  the change.
