# Metadata

Provenance records for generated artefacts (ADR-0014): one JSON file per
artefact, named by its id, written by `scripts/New-ArtifactMetadata.ps1`:

```powershell
./scripts/New-ArtifactMetadata.ps1 -Project BachelorPad+ -Task M4-04 -ArtifactPath artifacts/test-evidence/<file>
```

The script records the project, task, optional step and rosetta, creation
time, the artefact's path and SHA-256, and a status. `artifact.schema.json`
describes that shape; nothing validates records against it yet.

**No record has been committed yet** (2026-09-25). The benchmark JSON under
`artifacts/benchmarks/` carries its own time, host and compiler instead.
Release-plan item M4-04 is the next planned use, for per-OS test evidence.

`repository_manifest.json`, a list of the scaffold's files with sizes and
hashes from 2026-08-16, was deleted on 2026-09-25 (release-plan M2-13):
nothing generated it, nothing read it, and every hash of an edited file was
wrong. Git already records what it tried to.
