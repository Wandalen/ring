# invariant

Properties that must hold for every state a `GatingSet` can be in.

### Overview Table

| ID | Name | Claim |
|----|------|-------|
| 001 | [The Bound Is the Minimum and Only the Minimum](001_the_bound_is_the_minimum_and_only_the_minimum.md) | No consumer's progress but the slowest one's ever raises the headroom |
| 002 | [This Crate Names No Ordering](002_this_crate_names_no_ordering.md) | Zero atomic operations, zero `Ordering::`, zero `unsafe` — every concurrency decision is inherited |

### Both Are Absence Claims

Neither invariant says the crate does something. 001 says the bound is *not*
influenced by the average, the median, or the asking consumer. 002 says the crate
does *not* read an atomic, name an ordering, or write `unsafe`.

Absence claims are the ones that decay silently — nothing fails when a second
`.iter()` or a stray `Ordering::Relaxed` appears. Both are therefore checked by
the manual plan with greps whose expected output is *no output at all*.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT23 | The minimum-across-a-set property | n/a — coverage | It has three tests, and only one of them varies the slow consumer's index — so only `the_slowest_consumer_sets_the_bound_regardless_of_position_in_the_set` would catch a fold that returned the first or last element rather than the least |
| GT24 | Where the invariant is enforced | n/a — unenforced | In `ring_cursor::slowest`, in another crate. This crate's contribution is passing the whole slice; nothing here asserts the fold is a minimum, so a change to it reaches this crate as a behavioural test failure rather than a compile error |
| GT25 | The doctests | n/a — diagnostics | The crate names no ordering in any line the compiler builds into the library — measured zero — and names `Ordering::Release` five times inside doc examples. Those compile and run under `cargo test --doc`, so the invariant as stated holds for the library and not for everything this file causes to execute |
| GT26 | How the invariant is kept | **latent hazard** | By delegation. The ordering lives in `ring_cursor::GATING`, which this crate does not import, and arrives inside `ring_cursor::slowest`. A future direct cursor read here would have to name an ordering to compile, and nothing would flag it — the invariant is a property of the current call graph, not a constraint on the source |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- non-doc lines naming an ordering --'
command grep -cE '^[^/]*Ordering' ring_gating/src/lib.rs || true
echo '  -- doc lines naming one --'
command grep -c 'Ordering::' ring_gating/src/lib.rs || true
echo '  -- control: the crate that does name it, once, for the family --'
command grep '^pub const GATING' ring_cursor/src/lib.rs
echo '  -- and the tests that carry the minimum rule --'
command grep -E 'fn (a_stalled_consumer_stops|the_slowest_consumer_sets|one_stalled_consumer_stops)' ring_gating/tests/gating_test.rs
```

Live output:

```
  -- non-doc lines naming an ordering --
0
  -- doc lines naming one --
5
  -- control: the crate that does name it, once, for the family --
pub const GATING : Ordering = Ordering::Acquire;
  -- and the tests that carry the minimum rule --
fn a_stalled_consumer_stops_the_producer_at_exactly_one_lap()
fn the_slowest_consumer_sets_the_bound_regardless_of_position_in_the_set()
fn one_stalled_consumer_stops_the_producer_for_everyone()
```

**Zero in the code, several in the examples, one in the crate next door, and
three tests for the rule itself.** The invariant holds for everything the
compiler builds into this library; the doctests, which also execute, are the
exception it does not cover — [`002`](002_this_crate_names_no_ordering.md).
