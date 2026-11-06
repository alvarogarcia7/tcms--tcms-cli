---
id: TCMS-51
title: Speed up the GitLab pipeline by reducing Docker image size
status: To Do
assignee: []
created_date: '2026-11-06T00:36:00Z'
labels: []
dependencies: []
priority: medium
type: spike
ordinal: 93400
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Some Docker images used by the pipeline are over 7 GB, and pulling them is a significant part of pipeline run time. Investigate why the images are this large and whether slimming them would meaningfully shorten pipeline runs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each Docker image used by the GitLab pipeline is listed with its current size and the layers or packages that contribute most to it
- [ ] #2 The time spent pulling images in a representative pipeline run is measured and compared with total pipeline duration
- [ ] #3 A written recommendation states which image size reductions are worth pursuing and the expected pipeline time savings
- [ ] #4 Any follow-up implementation work is captured as separate backlog tasks linked to this one
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
