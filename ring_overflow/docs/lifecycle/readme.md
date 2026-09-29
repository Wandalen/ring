# lifecycle

Two lifecycles run through this crate and neither lasts as long as it looks. The
value it produces lives for one expression and is then gone. The work item that
says the crate should be built is still in the state that precedes being claimable,
and the crate has been built.

Both readings are useful in the same way: they say where to stop looking. There is
no concurrency story in the return value, because it does not survive long enough
to have one — the exposure is in the counter written on the way past. And there is
no state to recover from the task file, because it describes a crate that does not
exist.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_resolution_computed_matched_and_discarded.md) | A Resolution Computed, Matched, and Discarded | The value's span of existence, and the input that outlives it |
| [002](002_implemented_tested_and_still_planned.md) | Implemented, Tested, and Still Planned | The task's declared state, and the dependency list it shares with the readme |

## One Expression, Start to Finish

`would_resolve( self.overflow )` is a `match` scrutinee. The variant it returns is
tested against two patterns and is dead at the closing brace — never
named, never stored, never handed on. Nine sites in the workspace bind a
`Resolution` to a name and all nine are in the test file; across both crates' `src/`
the count is zero.

The input has the opposite shape. `Producer::overflow` is written once at
construction and has no setter, so the mapping's result is fixed for the producer's
whole life and recomputed on every full-ring event regardless. Caching it would cost
nothing in size — both types are one byte — and would put the evaluation at
construction, which is the one arrangement in which `would_resolve`'s `const`-ness
would finally be doing something.

## A Task That Describes a Crate That Is Already There

Task 113 reads `❓ Unverified` with a Scope section saying "Not yet worked out",
against 237 implemented lines and 15 passing tests. All 33 `ring_*` implement-tasks
read the same way and all 33 crates ship a `src/lib.rs`, so the tasks were produced
in one sweep and never walked forward.

The task also carries `**Depends on:** ring_types`, matching `readme.md:5` and
contradicting both the manifest and `src/lib.rs:5`. The file nearest the code is the
one that is right, which dates the omission: `ring_stats` was added after the readme
and the task were written, and only the module doc came along.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the value life in production, and the state its task declares --'
command grep -m1 -F '      Err( record ) => match would_resolve( self.overflow )' ring_core/src/lib.rs
command grep -m1 -A2 -F '# Task 113: Implement `ring_overflow`' ring_overflow/task/unverified/113_implement_ring_overflow.md | tail -n 1
echo '  -- implement-tasks in the first Open state, then crates shipping a lib --'
ls ring_*/task/unverified/*implement* 2>/dev/null | wc -l
ls ring_*/src/lib.rs 2>/dev/null | wc -l
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV45 | `ring_overflow` | n/a — observation | A `Resolution` in production is born as a `match` scrutinee and dead at the closing brace — nine name-bindings exist in the workspace and all nine are in the test file, zero across both crates' `src/` — so the type has no `Drop`, cannot be shared, torn, or read stale, and the crate's entire concurrency exposure sits instead in the counter `resolve` writes on the way past, which outlives every value the crate ever returns |
| OV46 | `ring_overflow` | n/a — observation | `Producer::overflow` is set once at construction and has no setter, so `would_resolve( self.overflow )` yields the same variant for the producer's whole life and is recomputed per full-ring event anyway — and since `Resolution` and `OverflowPolicy` are both one byte, storing the resolved variant instead would be size-free and would move the evaluation to construction, which is the one arrangement where the `const`-ness the crate maintains a duplicate mapping to preserve would actually be used |
| OV47 | `ring_overflow` | n/a — drift | Task 113 declares `❓ Unverified` — the state preceding claimable — with a Scope section reading "Not yet worked out" and no `## Verification Record`, against 237 implemented lines, 15 passing tests, and a thirteen-definition doc corpus; all 33 `ring_*` implement-tasks read the same way while all 33 crates ship a `src/lib.rs`, so the task set was produced in one sweep and never advanced, leaving task 113 useless as a record of what this crate needs |
| OV48 | `ring_overflow` | **wrong doc** | The manifest declares `ring_types` and `ring_stats`, and of the three prose dependency lists only `src/lib.rs:5` names both — `readme.md:5` and the task file's `**Depends on:**` line each omit `ring_stats` identically, dating the divergence to a pre-`ring_stats` design that only the module doc was updated from, and costing exactly the reader those two files exist for the fact that explains why the mapping is written twice |
