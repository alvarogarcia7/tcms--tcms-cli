---
id: TCMS-39.1
title: >-
  Release pipeline: publish SHA256SUMS for binaries and log them for
  verification
status: To Do
assignee: []
created_date: '2026-10-29T03:09:00Z'
labels: []
dependencies: []
references:
  - TCMS-39
parent_task_id: TCMS-39
type: enhancement
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The binary release pipeline (parent TCMS-39) produces per-architecture binaries/zips that users download, but gives them no way to confirm the files are intact and unmodified. Nothing in the job output records what was built either, so another agent or reviewer cannot later check from the pipeline logs that the published artifacts match what CI produced. Checksums must be generated in the pipeline and echoed into the job log so they can be cross-checked independently of the uploaded files.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The release pipeline generates a SHA256SUMS file covering the generated binaries (and release zip files) for each built architecture (macos arm64, linux amd64)
- [ ] #2 SHA256SUMS uses the standard format so it can be verified with `sha256sum -c` (or `shasum -a 256 -c` on macOS)
- [ ] #3 SHA256SUMS is included in the release artifacts and uploaded to the Gitlab release alongside the binaries
- [ ] #4 The job log prints the full contents of each SHA256SUMS file (hash and filename per line), clearly delimited, so another agent can read the log and compare the hashes against downloaded release files
- [ ] #5 The pipeline verifies the generated SHA256SUMS against the files (checksum check passes) before uploading, and the job fails if verification fails
- [ ] #6 Checksum generation and log output run only where the release is produced (tagged commits in master), consistent with parent TCMS-39
- [ ] #7 Checksum generation is portable across the BSD/GNU tool differences described in the project's shell compatibility rules
- [ ] #8 Documentation explains how users and agents verify a download against SHA256SUMS and against the pipeline log
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
