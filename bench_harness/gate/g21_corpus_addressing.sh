#!/usr/bin/env bash
# G21 — no recipe extracts source by absolute line number.
#
# A recipe that says `sed -n '210,222p' src/lib.rs` is correct exactly until
# somebody edits line 209. After that it still runs, still exits zero, and still
# prints Rust — just the wrong Rust. G15 catches the resulting disagreement, so
# nothing rots silently; what the coupling costs instead is that every source
# correction invalidates every recipe addressing a line below it, across every
# crate in the family.
#
# That cost was measured, not assumed: one five-line change to
# `ring_core/src/lib.rs` left 61 recipes stale across 18 crates, and not one of
# the 61 was a real disagreement about the source. Every one was a line number
# that had moved. S7 asks for 458 such corrections, so at that coupling the
# stage is a treadmill and a corpus that punishes correcting the defects it
# finds gets left uncorrected.
#
# The gate knows both spellings of the address, `sed -n 'N,Mp'` and
# `awk 'NR >= N && NR <= M'`, because it shipped knowing only the first,
# reported the family clean, and was believed — the 28 awk addresses it could
# not see surfaced out of G15 instead, four of them already printing different
# code. A gate that names its rule in prose and implements one syntax for it
# enforces the syntax, and the difference is invisible from a clean report.
set -uo pipefail
GATE=G21
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist
run_corpus_checker addressing.py "no recipe cites source by absolute line number"
