---
id: TCMS-39
title: Create a binary release for rust code
status: To Do
assignee: []
created_date: '2026-10-24 06:41'
labels: []
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
- Compile in CI/CD pipeline the {macos arm64, linux amd64} binaries for the rust code. 
- Package them in a zip file, one per architecture, so that a user can download it and use directly on their computer.
- Only for tagged commits (e.g. v1.01) in master.
- Upload the release to Gitlab release
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
- [ ] #2 Download the release from gitlab, make sure it works.
<!-- DOD:END -->
