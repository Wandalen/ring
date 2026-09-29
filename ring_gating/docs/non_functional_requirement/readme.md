# non_functional_requirement

Two requirements the crate's own feature entry makes explicit — one about cost,
one about safety. Neither is established by anything that runs in this crate; the
cost one is now pinned a crate over, by the test that was written when the
allocation it was about turned out to have been deleted.

### Overview Table

| ID | Name | Requires |
|----|------|----------|
| 001 | [Every Gating Read Allocates Nothing](001_every_gating_read_allocates_nothing.md) | The producer hot path paid a heap allocation and still pays four un-inlinable cross-crate calls per gate read |
| 002 | [The Gate Must Never Over-Report](002_the_gate_must_never_over_report.md) | A stale reading may under-report room and must never over-report it |

### Where They Come From

The cost requirement is that consulting the gating set — on the producer's hot
path — pays its cost in every measured claim; no number is attached to it up
front.

The safety requirement is stated by the crate's concurrent test, in its comment
rather than in its assertion — which is the subject of [002](002_the_gate_must_never_over_report.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT38 | The hot-path cost | n/a — coverage | The documented hot-path cost is still unmeasured: `ring_bench` is over twelve hundred lines and mentions the gate zero times. Its allocation half was answered from the other direction — the allocation was removed, and `ring_cursor/tests/allocation_test.rs` pins the zero — but a removed cost is not a measured one |
| GT39 | Inlining | **measured cost** | With no `#[ inline ]` anywhere in the 33 crates and no LTO configured, three of the chain's five remaining steps cannot be inlined across their crate boundaries in a default release build |
| GT40 | `a_gate_read_concurrently_with_a_consumer_never_over_reports_room` | n/a — coverage | It asserts `headroom <= CAPACITY`, which is true by construction for every possible input; the test cannot fail |
| GT41 | What over-reporting would require | n/a — coverage | The real hazard is a stale cursor read — reporting room a consumer has not released — which is an ordering question answered in `ring_cursor` by `GATING`. Nothing here tests it and nothing here could: the ordering is not named in this crate, so a test of it would be a test of the delegate |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every gating read goes through this one call --'
command grep 'ring_cursor::slowest' ring_gating/src/lib.rs | command grep -v '///' | sed 's/^ *//'
printf '  -- which allocated, one crate over, until b7e075ca; Vec< Seq > left there now: %s --\n' \
  "$( command grep -c 'Vec< Seq >' ring_cursor/src/lib.rs )"
echo '  -- and the test one crate over that now pins the zero --'
command grep 'fn the_gating_fold_allocates_nothing_at_every_arity' ring_cursor/tests/allocation_test.rs
echo '  -- files carrying an inline attribute anywhere in the family --'
command grep -rl '#\[ inline' --include=*.rs ring_*/src/ 2>/dev/null | wc -l
echo '  -- and the benchmark crate, which never mentions the gate --'
command grep -ci 'gating' ring_bench/src/lib.rs || true
```

Live output:

```
  -- every gating read goes through this one call --
//! `ring_cursor::slowest` returns `None` for an empty set rather than `Seq::ZERO`,
ring_cursor::slowest( &self.cursors )
  -- which allocated, one crate over, until b7e075ca; Vec< Seq > left there now: 0 --
  -- and the test one crate over that now pins the zero --
fn the_gating_fold_allocates_nothing_at_every_arity()
  -- files carrying an inline attribute anywhere in the family --
1
  -- and the benchmark crate, which never mentions the gate --
1
```

**One call, no allocation, no inlining anywhere, and no benchmark.** The five arms
are the whole cost argument as it now stands: the read is still delegated, the
delegate no longer allocates and there is finally a test saying so, nothing in the
family asks the compiler to collapse the chain, and the crate whose job is to
measure the write path now cites the gate once — a comment pointing at a design
doc, not a benchmark of it. One of the four costs this definition was opened about
was removed by a crate two tiers down without anything here noticing; the other
three are exactly where they were.
