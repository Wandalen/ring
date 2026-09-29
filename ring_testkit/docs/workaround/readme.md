# Workaround Doc Definition

### Scope

- **Purpose**: Record the external constraints `ring_testkit` absorbs on behalf of its consumers — each with whose constraint it actually is, what the crate pays, and the condition under which the workaround can be deleted.
- **Responsibility**: The constraint and its owner, what is absorbed and for whom, the cost, the deletion condition, and the evidence that the absorbed shape works.
- **In Scope**: Shapes this crate carries *because a tool cannot do something*, and would drop the moment the tool could.
- **Out of Scope**: Traps discovered in use rather than imposed from outside (→ [`pitfall/`](../pitfall/readme.md)); questions parked without a chosen shape (→ [`decisions/`](../decisions/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Two Allocations Loom Cannot Avoid](001_two_allocations_loom_cannot_avoid.md) | Why `leak` and `leak_ends` exist, what they cost, and the two events that delete them | 🔄 |
| 002 | [A Suite Split In Two By A Global Cfg](002_a_suite_split_in_two_by_a_global_cfg.md) | Why the tests live in two mutually exclusive files, and what nothing running the second half costs | 🔄 |

**One constraint, two shapes.** Both instances trace to the same origin — loom is
a whole-build substitution with no scoped threads — and they are separate because
what each one costs, and what would delete it, have nothing in common. `001` costs
memory and is deleted by a loom API or a `ring_core` constructor. `002` costs one
direction of compile checking and is deleted by an automated command, which
neither of those two changes would provide.

They also sit on opposite sides of the crate. `001` is a public API decision — two
exported functions a consumer calls. `002` is invisible from outside: no signature
changes, no item is added or removed, and a consumer reading `docs/api/` would
never learn that half the suite exists.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/workaround
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each names a deletion condition: %s\n' "$( command grep -licE 'deletion condition|deleted by|would delete' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'evidence rows, both files: %s\n' "$( command grep -hcE '^\| (W-E[0-9]|X[0-9]) \| ' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
printf 'loom mentions in src:     %s\n' "$( command grep -c 'loom' ../../src/lib.rs || true )"
printf 'cfg attributes in src:    %s\n' "$( command grep -c '#\[ cfg' ../../src/lib.rs || true )"
printf 'file-level cfg in tests:  %s\n' "$( cat ../../tests/*.rs | command grep -c '^#!\[ cfg' || true )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
each instance has a recipe: 2
each names a deletion condition: 2
evidence rows, both files: 8
loom mentions in src:     19
cfg attributes in src:    0
file-level cfg in tests:  2
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK51 | three of four open questions resolved elsewhere | n/a — doc gap | Of the four Pending items in this crate's decision log, three name another `ring_*` crate as where the answer lives — Pending 2 in `ring_shutdown`, Pending 3 in `ring_core`, Pending 4 in `ring_spsc` — and `pitfall/002` TK44 measures the one case in detail: `ring_shutdown`'s own decision log has never heard the question and one file in that whole crate names `ring_testkit`, so nothing in the corpus carries an entry from the crate that observed a cost to the crate that could remove it. |
| TK52 | the cost dismissed by unmeasured comparison | n/a — doc gap | `leak`'s doc comment and this definition both retire the leak's cost as *"loom's own per-execution bookkeeping dwarfs it"* with no number on either side — the ring is checkably tiny at `CAPACITY = 2`, but nothing states loom's per-execution allocation, the executions the three models run, or the total for one `--cfg loom` invocation, and the leak is unbounded in exactly the dimension left uncounted. |
| TK53 | the cfg no automated command sets | n/a — coverage | `verb/test` sets `RUSTFLAGS="-D warnings"` and the loom cfg nowhere, no file under `verb/` sets it, and `RUSTFLAGS="--cfg loom"` appears at six sites across four files — manifest comment, both test files' module docs, and `tests/manual/readme.md` twice — so the family's only `#![ cfg( loom ) ]` file, holding three models over six threads, runs when a human remembers. |
| TK54 | a placement rationale that states only its benefit | **misleading doc** | The manifest justifies models living in `tests/` because *"the scripted fixture stays measurable by a coverage run that does not set the cfg"*, which is true and does not mention that the same run does not compile the loom half at all — so a rename or signature change that breaks it yields a fully green `verb/test`, and the asymmetry is real: the scripted file's exclusion under the cfg is a soundness requirement, the exhaustive file's exclusion without it is a build consequence with no automated compile check anywhere. |

### Two things that look like workarounds and are not

**`ring_tls::flush_into` being unreachable** is not absorbed — it is simply not
used. This crate stages into a `TlsBuffer` and publishes into a
`ring_core::Ring` one record at a time because the amortised path claims against
a cursor a `ring_core::Ring` does not have. Nothing is worked around; a gap is
recorded. → [`pitfall/003`](../pitfall/003_the_amortised_flush_has_no_ring.md),
[`decisions/readme.md`](../decisions/readme.md) Pending 3.

**`ring_types` as a dev-dependency** is a two-crate import to set one field:
`RingConfig::with_overflow` takes an `OverflowPolicy` that `ring_config` does not
re-export. It is a finding about the family's surface rather than a constraint
this crate absorbs for anyone — the extra import is paid by this crate's own
tests, and a consumer of `ring_testkit` never meets it, because `Script::run`
takes a ring the caller has already built.
→ [`integration/001`](../integration/001_the_three_edges_and_the_one_that_is_missing.md).
