#!/usr/bin/env python3
"""Temporary experiment (#55): results.log -> markdown, median of rounds per cell, variant and consumer work."""
import collections, re, statistics, sys

vals = collections.defaultdict(list)
spins = collections.defaultdict(list)
cur = {}
for line in open(sys.argv[1]):
    p = line.split(None, 3)
    if len(p) < 4:
        continue
    key = tuple(p[:3])
    rest = p[3].strip()
    m = re.match(r"^(spsc\S*/\S+)", rest)
    if m:
        cur[key] = m.group(1)
    if key not in cur:
        continue
    cell, (rnd, work, variant) = cur[key], key
    t = re.search(r"thrpt:\s+\[\S+ \S+ (\S+) (\S+)", rest)
    if t:
        vals[(cell, variant, work)].append(float(t.group(1)) * (1000 if t.group(2).startswith("G") else 1))
    s = re.search(r"full (\S+)/record, empty (\S+)/record", rest)
    if s:
        spins[(cell, variant, work)].append(float(s.group(1)))

variants = ["master-64", "pr41-64", "master-128", "pr41-128"]
works = sorted({w for _, _, w in vals}, key=lambda w: int(w[1:]))
for cell in sorted({c for c, _, _ in vals}):
    print(f"\n### {cell} (M records/s, median of rounds; full spins per record)\n")
    print("| consumer work | " + " | ".join(variants) + " | #41 vs master (64) | #41 vs master (128) |")
    print("|---|" + "---|" * (len(variants) + 2))
    for w in works:
        med = {v: statistics.median(vals[(cell, v, w)]) if vals[(cell, v, w)] else None for v in variants}
        row = [f"{med[v]:.1f} (f{statistics.median(spins[(cell, v, w)]):.2f})" if med[v] else "—" for v in variants]
        rel = []
        for a, b in (("master-64", "pr41-64"), ("master-128", "pr41-128")):
            rel.append(f"{100 * (med[b] / med[a] - 1):+.0f}%" if med[a] and med[b] else "—")
        print(f"| {w[1:]} | " + " | ".join(row) + " | " + " | ".join(rel) + " |")
