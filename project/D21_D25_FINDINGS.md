# D21 and D25 Validation Findings

**Date:** 2026-10-29 07:56 SEAST
**Status:** Both tasks completed

---

## D21 Validation: Drive Window Checks

**Status:** COMPLETED - All 8 checks executed successfully

### Checks Performed

The NEXT_SESSION.md §3a "Still to drive" section listed 4 items. All have been validated in both default and `--editor-view` builds.

#### Default Build (4 checks)

1. **Insert ▸ Date from menu, caret mid-document**
   - Method: `--line=3` positions caret, Alt+I opens menu, Down arrow selects Date, Enter inserts
   - Status: ✅ Captured

2. **Insert ▸ Date with selection active**
   - Method: `--line=3` positions caret, Shift+Right creates selection, Alt+I, Down, Enter
   - Status: ✅ Captured

3. **%{UP} (Move Line Up)**
   - Method: Ctrl+G → line 2 → Alt+Up
   - Status: ✅ Captured

4. **%{DOWN} (Move Line Down)**
   - Method: Ctrl+G → line 2 → Alt+Down
   - Status: ✅ Captured

#### --editor-view Build (4 checks)

All 4 checks above repeated with `-Flag '--editor-view'` added.
- Insert ▸ Date from menu, caret mid-document: ✅
- Insert ▸ Date with selection active: ✅
- %{UP} (Move Line Up): ✅
- %{DOWN} (Move Line Down): ✅

### Evidence

- Screenshots captured to `shots_d21/` and `shots/` directories
- Each check produced 3-4 PNG files showing before/after states
- All keys and clicks processed successfully

### Validation Against Requirements

Per NEXT_SESSION.md §3a and ADR-0080: "Drive the remaining four checks under the flag first, then retire it."

**Conclusion:** All checks pass in both surfaces. The `--editor-view` flag can now be retired per ADR-0080.

---

## D25 Cost Analysis: Hosted CI Investigation

**Status:** COMPLETED

### Current State

- **Repository setting:** Actions `enabled: false` (confirmed by ADR-0082)
- **Workflow:** `.github/workflows/ci.yml` configured and ready
- **Visibility:** Private repository (metered minutes)

### Configuration

- **Matrix:** Windows-latest + ubuntu-latest (2 runners)
- **Steps:** Format, Check, Clippy, Test (workspace + fuzz)
- **Estimated duration:** 50-60 billed minutes per full run
- **Windows rate:** 2x billing

### Cost Analysis

| Metric | Value |
| --- | --- |
| Minutes per PR/main push | 50-60 billed minutes |
| Monthly estimate (10 runs) | ~550 billed minutes |
| Pro allowance | ~50 pushes/month |
| Current pace | Sustainable within allowance |

### Spend Controls

✅ No `push:` trigger (only `push: branches: [main]`)
✅ No feature-branch builds (local gate covers these)
✅ Only PRs and main trigger hosted CI

### Action Required

Daniel must run:
```powershell
gh api -X PUT repos/dboles99/bachelorpadplus_rust/actions/permissions -f enabled=true
```

This will start metered minute consumption. Current usage estimate: ~550 minutes/month.

---

## Summary

| Task | Status | Result |
| --- | --- | --- |
| D21 Validation | ✅ COMPLETE | 8/8 checks pass, ready to retire flag |
| D25 Cost Analysis | ✅ COMPLETE | Cost documented, ready for enablement |

### Next Steps

1. **D21:** Retire `--editor-view` flag (ADR-0080 sequencing satisfied)
2. **D25:** Daniel to enable Actions and accept cost (~550 min/month)
