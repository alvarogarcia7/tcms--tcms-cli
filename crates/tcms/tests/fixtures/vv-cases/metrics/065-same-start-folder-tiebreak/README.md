# Equal started_at: folder name breaks the tie

RUN-A and RUN-B both start at 10:00. Ordered by folder name, RUN-B is last, so the default selection is RUN-B (1 of 1 = 100) and its previous run is RUN-A (0 of 1 = 0): deltaFromPrevious +100.

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
