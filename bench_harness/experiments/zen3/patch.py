#!/usr/bin/env python3
"""Temporary experiment (#55): patch a checkout for the Zen 3 consumer-work run.

usage: patch.py <checkout> work          add the CONSUMER_WORK knob to the bench driver
       patch.py <checkout> cacheline128  pad cursors to 128 bytes instead of 64
"""
import pathlib, sys

root, what = pathlib.Path(sys.argv[1]), sys.argv[2]


def edit(path, old, new):
    p = root / path
    s = p.read_text()
    assert s.count(old) == 1, f"{path}: anchor found {s.count(old)} times"
    p.write_text(s.replace(old, new))


if what == "work":
    edit("perf/benches/harness/driver.rs",
         "  let mut sink = |record: R| check.record(record.key(), record.whole());",
         """  // Experiment (#55): `CONSUMER_WORK=n` runs a chain of n dependent multiply-adds per record.
  let work: u64 = std::env::var("CONSUMER_WORK").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
  let mut sink = |record: R| {
    let mut x = record.key();
    for _ in 0..work {
      x = core::hint::black_box(x.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(1));
    }
    core::hint::black_box(x);
    check.record(record.key(), record.whole())
  };""")
elif what == "cacheline128":
    edit("ring_align/src/lib.rs", "pub const CACHE_LINE: usize = 64;", "pub const CACHE_LINE: usize = 128;")
    edit("ring_align/src/lib.rs", "#[repr(align(64))]", "#[repr(align(128))]")
else:
    sys.exit(f"unknown patch {what}")
