---
name: capstone-topic
description: Drive one capstone topic from docs/capstone/ to a deliverable phrased as a check that fails on a planted violation and passes on the fix.
disable-model-invocation: true
argument-hint: "<topic number 1-7>"
---

# Capstone topic

## Select

1. Read `docs/capstone/0<N>_*.md` and `docs/capstone/readme.md`.
2. Check nobody else owns it: `git branch -a`, open PRs, the team's assignment. One topic, one
   owner.
3. Restate the gap and the deliverables in three lines. Name the files you will touch and the
   ones owned by other topics you must not (CI → 1, fuzz → 2, `deny.toml`/audit → 3,
   MSRV/metadata/`CHANGELOG` → 4, rustdoc → 5, `no_std` → 6, bench reporting → 7).

## Verify the premise

The topic docs were written at extraction time; some claims have since gone stale. Check each
one the plan depends on against the tree before building on it. Known drift (2026-09-30):

- `ring_bench` is not criterion and there is no `crossbeam-channel`; the direct external
  dependencies are `crossbeam-queue` (optional), `loom` (cfg), `trybuild` (dev).
- Only `ring_spsc` and `ring_mpsc` contain `unsafe`; `ring_core` is a stale allowlist entry.
- Every crate already has `#![deny(missing_docs)]`.
- Five crates are `publish = false`: `bench_harness`, `ring_bench`, `ring_debug`,
  `ring_testkit`, `ring_trace`.
- The trybuild case named as failing has been re-blessed.
- Loom models run with `./verb/loom`; nothing ran them automatically before.

## Define done as a check

Phrase every deliverable as something that can fail: a gate, a CI job, a verb, a test, a
command with expected output. For each, write down the planted violation that must turn it red.
New gate → `new-gate` skill.

## Build

- Wire the check into a `verb/` script so it runs the same locally and in CI. `key::val`
  parameters, `dry::1`, reject unknown parameters (`CODESTYLE.md` § Scripts).
- Keep the diff to the topic. Findings outside it become issues, not detours.
- Document the policy the topic asks for (e.g. what happens when audit goes red) where the
  check lives, briefly.

## Demonstrate

1. Plant the violation → red, with the output.
2. Remove it → green, with the output.
3. Both in the PR description; that pair is the demo for the final session.
