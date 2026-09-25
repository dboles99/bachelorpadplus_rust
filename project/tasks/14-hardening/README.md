# 14-hardening

Roadmap phases 19 (hardening, fuzzing, benchmarks) and 20 (packaging,
signing, release). **Partly done.** Hostile-input harnesses and Windows
measurements exist; packaging and the release pipeline are milestone M3, and
evidence on each OS is M4. Status checked against the code on 2026-09-25.

| Task | Status |
| --- | --- |
| fuzzing | Partly. `fuzz/` holds hostile-input harnesses (a checked-in corpus plus `proptest`) for `bp-crypto`, `bp-data`, `bp-files`, `bp-formats` and `bp-notebook`, run by the gate. It is not coverage-guided fuzzing, as `fuzz/README.md` says; one coverage-guided run on Linux is M4-05 |
| benchmarks | Partly. Startup and idle memory were measured on Windows 10 in August (`artifacts/benchmarks/`), typing latency on Windows (ADR-0017); the release binary's size was recorded after M1-13. Per-OS startup, open-time and memory measurements: M4-01 to M4-03 |
| threat-tests | Not started as a document: `docs/security/THREAT_MODEL.md` is M4-06. Security behaviour is covered by crate and cross-crate tests |
| packaging | Not started. Windows installer and portable zip: M3-01; Linux deb, rpm, tar.gz and PKGBUILD: M3-02; release script: M3-03; signing: M3-04 (the owner is obtaining the identity) |

Licensing files and `cargo deny` in the gate are M1-14, waiting on the
copyright holder's name.

The four task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).
