#!/usr/bin/env python3
"""List the test and bin executables cargo produced, and count them per package.

Each executable is a separate link step, so the count per package is a direct measure of link cost.

Usage: parse_executables.py <cargo-test-json> <out.tsv>
"""
import collections
import json
import os
import sys


def package_name(package_id: str) -> str:
    # path packages: "path+file:///.../crates/NAME#VERSION"
    # registry packages: "registry+...#NAME@VERSION"
    if "#" in package_id:
        head, tail = package_id.split("#", 1)
        if "@" in tail:
            return tail.split("@", 1)[0]
        return os.path.basename(head.rstrip("/"))
    return package_id


def main() -> int:
    json_path, out_path = sys.argv[1], sys.argv[2]
    rows = []
    seen = set()
    with open(json_path, encoding="utf-8") as f:
        for line in f:
            try:
                msg = json.loads(line)
            except ValueError:
                continue
            if msg.get("reason") != "compiler-artifact" or not msg.get("executable"):
                continue
            exe = msg["executable"]
            if exe in seen:
                continue
            seen.add(exe)
            size = os.path.getsize(exe) if os.path.exists(exe) else 0
            rows.append((
                package_name(msg["package_id"]),
                msg["target"]["name"],
                ",".join(msg["target"]["kind"]),
                size,
                exe,
                "true" if msg.get("profile", {}).get("test") else "false",
            ))

    with open(out_path, "w", encoding="utf-8") as f:
        f.write("package\ttarget\tkind\tbytes\tpath\tis_test_harness\n")
        for row in rows:
            f.write("\t".join(str(x) for x in row) + "\n")

    per_pkg_count = collections.Counter(r[0] for r in rows)
    per_pkg_bytes = collections.Counter()
    for r in rows:
        per_pkg_bytes[r[0]] += r[3]
    print(f"test_executables: {len(rows)}")
    print(f"total_executable_bytes: {sum(per_pkg_bytes.values())}")
    print("executables per package (each is a separate link step):")
    for pkg, count in sorted(per_pkg_count.items(), key=lambda kv: (-kv[1], kv[0]))[:25]:
        print(f"  {count:4d} exe  {per_pkg_bytes[pkg]:14d} bytes  {pkg}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
