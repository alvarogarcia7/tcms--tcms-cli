# --run takes the folder name, not run_id

Folder RUN-FOLDER-1 contains test-run.yaml with run_id RUN-INSIDE-9. --run RUN-INSIDE-9 must fail (TCMS-50.6: folder name only). exit 1.

**Expected:** exit 1 (refused; nothing valid on stdout).

Arguments and expected values are in `case.json`; see the index `../README.md`.
