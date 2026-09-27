# 14-hardening

Roadmap phases 19 (hardening, fuzzing, benchmarks) and 20 (packaging,
signing, release). **Partly done.** Hostile-input harnesses and Windows
measurements exist; packaging and the release pipeline are built and wired
(M3), with a Linux dry run recorded; evidence on each OS is M4. Status
checked against the code on 2026-09-27.

| Task | Status |
| --- | --- |
| fuzzing | Partly. `fuzz/` holds hostile-input harnesses (a checked-in corpus plus `proptest`) for `bp-crypto`, `bp-data`, `bp-files`, `bp-formats` and `bp-notebook`, run by the gate. It is not coverage-guided fuzzing, as `fuzz/README.md` says; one coverage-guided run on Linux is M4-05 |
| benchmarks | Partly. Startup and idle memory were measured on Windows 10 in August (`artifacts/benchmarks/`), typing latency on Windows (ADR-0017); the release binary's size was recorded after M1-13. Per-OS startup, open-time and memory measurements: M4-01 to M4-03 |
| threat-tests | Not started as a document: `docs/security/THREAT_MODEL.md` is M4-06. Security behaviour is covered by crate and cross-crate tests |
| packaging | Built and wired. Windows installer and portable zip (M3-01) and signing (M3-04, `packaging/windows/Signing.ps1`) have never run on Windows, and signing has no identity yet (decision 3, question 3a). Linux deb, rpm, tar.gz and PKGBUILD (M3-02) are tested on Ubuntu 22.04 and 24.04 only; the pinned Debian 12 build image has never been built. The release script (M3-03) has a native Linux dry run recorded in `artifacts/releases/v1.0.0-rc.0/linux/` |

Licensing files (holder Daniel Boles), the generated
`THIRD-PARTY-NOTICES.txt` and a `cargo deny` gate stage are done (M1-14).

The four task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).
