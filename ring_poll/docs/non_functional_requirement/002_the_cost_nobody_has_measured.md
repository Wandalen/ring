# Non-Functional Requirement: The Cost Nobody Has Measured

### Scope

- **Purpose**: Record what quantitative evidence exists for this crate's cost claims, where it lives, and what runs it.
- **Responsibility**: The single measured figure, the ceiling the suite actually enforces, and the family's benchmarking arrangement.
- **In Scope**: The 500 ms assertion, the manual probe behind it, and `ring_bench`'s dependency set.
- **Out of Scope**: What the costs are (→ [`001`](001_what_a_tick_may_cost.md)); the correctness the suite does enforce (→ [`../invariant/002`](../invariant/002_a_budget_bounds_attempts_not_time.md)).

### What is measured, and by what

| Claim | Where stated | What checks it | When it last ran |
|---|---|---|---|
| Spinning beats parking on wall clock | `invariant/002`, module doc | `a_large_budget_spins_rather_than_sleeping` | every suite run |
| …by roughly two orders of magnitude | `tests/manual/readme.md` P3 | a hand-run command | 2026-08-28, once |
| One attempt is cheap | nowhere quantitative | nothing | — |
| A batch attempt is bounded by free space | `001` / PL33 | nothing | — |
| `out.push` may reallocate | `001` / PL34 | nothing | — |

The one figure the crate owns is P3's: **20 000 attempts in ≤ 8 ms**, against a
500 ms assertion — a 62× margin, recorded by hand on one machine on one day.

### What the automated assertion is worth

`a_large_budget_spins_rather_than_sleeping` asserts
`elapsed < Duration::from_millis( 500 )` for 20 000 attempts. Its failure message
names the parking cost it is separating from — *"a sleeping pause would cost
about 1.0 s"* — and it is right about that: 20 000 × 50 µs is a second, and no
parking implementation passes.

It is a *category* test, not a *cost* test. At 62× margin, the crate would have
to become sixty-two times slower before the suite noticed → PL35.

### What the family has instead of benchmarks

| Artifact | Count |
|---|---|
| `ring_*` crates with `tests/manual/` | 33 of 33 |
| `ring_*` crates with a `benches/` directory | 0 |
| Manifests declaring `[[bench]]` or criterion | 0 |
| Runnable performance artifacts in the family | 1 — `ring_bench/examples/comparison.rs` |
| Family crates `ring_bench` depends on | 9 |
| …of which are tick-path helpers | 0 → PL36 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'crates in the family:       %s\n' "$( for d in ring_*/; do echo x; done | wc -l )"
printf 'with tests/manual:          %s\n' "$( for d in ring_*/tests/manual; do [ -d "$d" ] && echo x; done | wc -l )"
printf 'with a benches dir:         %s\n' "$( for d in ring_*/benches; do [ -d "$d" ] && echo x; done | wc -l )"
printf 'manifests with bench/crit:  %s\n' "$( command grep -lE '\[\[bench\]\]|criterion' ring_*/Cargo.toml 2>/dev/null | wc -l )"
printf 'examples in the family:     %s\n' "$( for d in ring_*/examples; do [ -d "$d" ] && ls "$d"; done | tr '\n' ' ' )"
printf 'ring_bench depends on:      %s\n' "$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_bench/Cargo.toml | command grep -oE '^ring_[a-z_]+' | wc -l )"
printf 'and names ring_poll:        %s\n' "$( command grep -c 'ring_poll' ring_bench/Cargo.toml || true )"
printf 'the timing assertion:       %s\n' "$( command grep -oE 'Duration::from_millis\( [0-9]+ \)' ring_poll/tests/poll_test.rs )"
printf 'attempts it spends:         %s\n' "$( awk '/fn a_large_budget_spins/{f=1} f&&/^\}$/{exit} f' ring_poll/tests/poll_test.rs | command grep -oE 'Budget::new\( [0-9_]+ \)' )"
printf 'measured cost, from P3:     %s\n' "$( command grep -oE '0\.[0-9]+ s, including test setup' ring_poll/tests/manual/readme.md )"
printf 'margin under the bound:     %s\n' "$( command grep -oE '[0-9]+. under the bound' ring_poll/tests/manual/readme.md )"
printf 'margin under parking cost:  %s\n' "$( command grep -oE '[0-9]+. under the parking cost' ring_poll/tests/manual/readme.md )"
printf 'the parking side, arith:    %s\n' "$( command grep -oE '~1 000 ms|about 1\.0 s' ring_poll/tests/manual/readme.md | head -1 )"
printf 'dates in the manual plan:   %s\n' "$( command grep -oE '2026-[0-9]{2}-[0-9]{2}' ring_poll/tests/manual/readme.md | sort -u | tr '\n' ' ' )"
printf 'timing assertions in suite: %s\n' "$( command grep -c 'Duration::from' ring_poll/tests/poll_test.rs || true )"
```

Live output:

```
crates in the family:       33
with tests/manual:          33
with a benches dir:         0
manifests with bench/crit:  0
examples in the family:     comparison.rs 
ring_bench depends on:      9
and names ring_poll:        0
the timing assertion:       Duration::from_millis( 500 )
attempts it spends:         Budget::new( 20_000 )
measured cost, from P3:     0.008 s, including test setup
margin under the bound:     62× under the bound
margin under parking cost:  125× under the parking cost
the parking side, arith:    about 1.0 s
dates in the manual plan:   2026-08-28 
timing assertions in suite: 1
```

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_what_a_tick_may_cost.md](001_what_a_tick_may_cost.md) | The costs this file establishes are unmeasured |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | PL24 — the same 500 ms bound, read as a stated-margin problem |

### Items

| File | Relationship |
|------|--------------|
| [`../item/002_what_the_crate_does_not_declare.md`](../item/002_what_the_crate_does_not_declare.md) | PL28 — the inlining question this absence leaves open |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/001_non_parking_is_not_bounded_latency.md`](../pitfall/001_non_parking_is_not_bounded_latency.md) | The caller the missing measurement would have informed |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The cost claims in the module doc, none of them quantitative |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `a_large_budget_spins_rather_than_sleeping` — the crate's only timing assertion |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | P3 — the only measured figure the crate owns, hand-run once |

### PL35 — the only automated cost check has 62× of margin, by design

`a_large_budget_spins_rather_than_sleeping` is the crate's sole timing
assertion: 20 000 attempts, `elapsed < Duration::from_millis( 500 )`. The manual
probe P3 measured what it actually costs — **0.008 s including test setup** — and
states both ratios itself: 62× under the bound, 125× under the parking cost the
bound was placed to exclude.

That margin is correct for the job the test was written to do. It separates two
categories, spinning and parking, whose costs differ by three orders of
magnitude; a tight bound would flake on a loaded CI machine and prove nothing
extra. The manual plan says so plainly, and PL24 already records the asymmetry in
which margin the failure message quotes.

The consequence is what this file is for: the crate has no regression detector.
A change that makes the tick path five times slower — a lost inline, a bounds
check reintroduced, an extra atomic in `ring_core::try_push` — passes this
assertion with room to spare, and passes every other test in the suite, because
every other test checks behaviour. Nothing in `verb/test` would report it.

For most crates that is fine. For this one it is the specific thing being
claimed: the crate exists so that a system's turn on the ring costs a known,
bounded amount, and the module doc reasons about frame budgets. Between "does not
park" (enforced structurally, by the dependency graph) and "is fast" (asserted
nowhere) there is a gap the test cannot close, and the 62× is not a flaw in the
test — it is the measurement of how wide that gap is.

### PL36 — the family has a benchmarking crate and no benchmarks

`ring_bench` is a real crate with a considered surface — `Workload`, `Candidate`,
`Outcome`, `Comparison`, and a `run` function — and one worked example,
`examples/comparison.rs`. It is the only runnable performance artifact anywhere in
the thirty-three-crate family.

Around it, the counts are zero. No crate has a `benches/` directory. No manifest
declares a `[[bench]]` target or depends on criterion. No crate but `ring_bench`
has an `examples/` directory at all. So the family's performance evidence is one
example program, run when somebody runs it.

`ring_bench` depends on nine family crates — `ring_factory`, `ring_tls`,
`ring_flush`, `ring_stats`, `ring_spsc`, `ring_mpsc`, `ring_core`, `ring_slot`,
`ring_types` — which is a coherent set: it compares ring *backends*, which is the
choice most worth measuring in a ring family. `ring_poll` is not among them, and
neither is any other tick-path crate.

The gap is narrow and specific. The backends are where throughput lives, so
benchmarking them first is right. But `ring_poll`'s claim is not a throughput
claim — it is that a *bounded* helper is worth having, that `Budget::once()` is
the safe default, and that the difference between a spin hint and a park matters
per frame. Those are cost claims about the layer above the backend, and the
harness that could price them already exists, already knows how to run a
`Workload` against a `Candidate`, and would need a dependency edge and a
comparison to answer PL33's question about what a batch attempt costs.

Recorded rather than built because adding a benchmark is a change to `ring_bench`,
not to this crate, and because the finding is the family-shaped one: the crate
best placed to measure the tick path is one manifest line away from it and has
been for as long as both have existed.
