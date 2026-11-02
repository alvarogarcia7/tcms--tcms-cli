#!/usr/bin/env python3
"""Extract per-unit and per-crate compile times from cargo's HTML timing report.

cargo embeds the unit table as a JavaScript literal (`const UNIT_DATA = [...];`).

Usage: parse_timing.py <cargo-timing.html> <out_dir>
Writes <out_dir>/unit-times.tsv and <out_dir>/crate-times.tsv and prints the top units.
"""
import collections
import json
import os
import re
import sys


def main() -> int:
    html_path, out_dir = sys.argv[1], sys.argv[2]
    with open(html_path, encoding="utf-8") as f:
        html = f.read()
    match = re.search(r"const UNIT_DATA = (\[.*?\]);", html, re.S)
    if not match:
        print(f"could not find UNIT_DATA in {html_path}")
        return 1
    units = json.loads(match.group(1))

    with open(os.path.join(out_dir, "unit-times.tsv"), "w", encoding="utf-8") as f:
        f.write("name\tversion\tmode\tduration_s\n")
        for u in units:
            f.write(f"{u['name']}\t{u.get('version', '')}\t{u.get('mode', '')}\t{round(u['duration'], 3)}\n")

    per_crate = collections.defaultdict(float)
    for u in units:
        per_crate[u["name"]] += u["duration"]
    with open(os.path.join(out_dir, "crate-times.tsv"), "w", encoding="utf-8") as f:
        f.write("name\ttotal_duration_s\n")
        for name, seconds in sorted(per_crate.items(), key=lambda kv: -kv[1]):
            f.write(f"{name}\t{round(seconds, 3)}\n")

    total = sum(u["duration"] for u in units)
    print(f"compilation_units: {len(units)}  sum_of_unit_durations_s: {round(total, 2)}")
    print("top 15 units by duration:")
    for u in sorted(units, key=lambda u: -u["duration"])[:15]:
        print(f"  {u['duration']:8.2f}s  {u['name']} {u.get('version', '')} ({u.get('mode', '')})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
