# 10-research

Roadmap phase 13. **Built and unwired, with no target release.** ADR-0030
keeps `bp-research` out of RC1 and the release plan hides its menu; no later
release has been named for it. Status checked against the code on
2026-09-25.

| Task | Status |
| --- | --- |
| research-profile | Built in `bp-research` (`profile.rs`); unwired |
| citation-model | Built in `bp-research`: BibTeX and CSL read and write, a model of a cited work, citation styles; unwired |
| paper-metadata | Built in `bp-research` (`paper.rs`); unwired |

The three task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

Nothing in the application or any other crate depends on `bp-research`.
Everything in it is offline by design (ADR-0006): DOI lookup, reference
completion and retraction checks are absent, not stubbed. The Research menu
is hidden unless `show_planned_menus = true` (M1-05). Its BibTeX and CSL
parsers have no fuzz harness (ADR-0008); that matters only once the crate is
reachable.
