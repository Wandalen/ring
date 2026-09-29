# Pitfall: A Range That Reads Backwards

### Scope

**Purpose:** Record what this crate's only rendering of a log entry does at the
top of the sequence range, establish that the value it breaks at is a named
public constant the family publishes rather than a hypothetical, and note how far
into degenerate ranges the suite went before stopping.

**Responsibility:** `TraceEntry::end`, `Display for TraceEntry`, what `record`
validates, the family's arithmetic guards, and the two range cases the suite
asserts.

**In Scope:** `ring_trace/src/lib.rs:144-157`, `:256-263`;
`ring_trace/tests/trace_test.rs:139-147`; `ring_mpsc/src/lib.rs:222`.

**Out of Scope:** That the format is undocumented at all is
[`item/001`](../item/001_six_impl_blocks_and_the_three_a_lint_cannot_reach.md).
That `end` open-codes an addition `ring_types` already names is
[`workaround/001`](../workaround/001_an_addition_the_types_crate_already_offers.md).
The upstream wrap itself belongs to `ring_types`
([its `pitfall/001`](../../../ring_types/docs/pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md)).

---

## The Addition, the Rendering, and the Guards

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the range, and the addition behind it --'
command grep -m1 -A20 -F '  /// One past the last sequence this entry covers.' ring_trace/src/lib.rs
echo '  -- what record checks before pushing whatever it was handed --'
command grep -m1 -A7 -F '  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )' ring_trace/src/lib.rs
echo '  -- the guards this crate and the family use on that addition --'
printf '    Panics sections in ring_trace: %s   checked/saturating/wrapping adds in 33 crates: %s\n' \
  "$( command grep -c '# Panics' ring_trace/src/lib.rs || true )" \
  "$( command grep -rc 'checked_add\|saturating_add\|wrapping_add' --include=*.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
for c in ring_*/; do
  k=$( command grep -c 'debug_assert' "$c"src/lib.rs 2>/dev/null || true )
  if [ "$k" != 0 ]; then printf '    debug_assert in %s: %s\n' "$( basename "$c" )" "$k"; fi
done
echo '  -- and how far the suite went into degenerate ranges --'
command grep -m1 -A11 -F '  assert_eq!( entries[ 0 ].end(), Seq( 72 ) );' ring_trace/tests/trace_test.rs | tail -n 9
```

Live output:

```
  -- the range, and the addition behind it --
  /// One past the last sequence this entry covers.
  ///
  /// Saturates rather than wrapping: a bare `+` here would print a range that
  /// reads backwards, or panic under debug assertions, the moment a caller
  /// traces the one `Seq` the family publishes by name —
  /// `ring_mpsc::UNSTAMPED` (`Seq(u64::MAX)`). See `pitfall/001` TR41.
  #[ must_use ]
  pub const fn end( &self ) -> Seq
  {
    Seq( self.seq.0.saturating_add( self.count as u64 ) )
  }
}

impl fmt::Display for TraceEntry
{
  fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result
  {
    write!( f, "{} {}..{}", self.op, self.seq.0, self.end().0 )
  }
}

  -- what record checks before pushing whatever it was handed --
  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )
  {
    if !self.enabled
    {
      return;
    }
    self.entries_guard().push( TraceEntry { op, seq, count } );
  }
  -- the guards this crate and the family use on that addition --
    Panics sections in ring_trace: 0   checked/saturating/wrapping adds in 33 crates: 4
    debug_assert in ring_consume: 1
    debug_assert in ring_core: 1
    debug_assert in ring_flush: 2
    debug_assert in ring_poll: 1
    debug_assert in ring_slot: 1
    debug_assert in ring_tls: 2
  -- and how far the suite went into degenerate ranges --
#[ test ]
fn an_entry_reports_the_range_it_covers()
{
  let entry = TraceEntry { op : TraceOp::Publish, seq : Seq( 5 ), count : 3 };
  assert_eq!( entry.end(), Seq( 8 ) );

  let empty = TraceEntry { op : TraceOp::Publish, seq : Seq( 5 ), count : 0 };
  assert_eq!( empty.end(), Seq( 5 ), "a zero-count operation covers nothing" );
}
```

## What the Log Prints for the One `Seq` the Family Publishes

*This probe recorded `end`'s behaviour before TR41's fix below: a bare `+`
that panicked under debug assertions and printed a backwards-reading range in
release when tracing `ring_mpsc::UNSTAMPED`. `end` has since been rewritten to
`saturating_add`, so tracing that same sentinel today prints
`publish 18446744073709551615..18446744073709551615` in both profiles — never
the panic or the backwards range shown here. See TR41's Disposition and the
`end_saturates_instead_of_reading_backwards_at_the_top_of_u64` test it added.*

```rust
// -tr_probe/src/bin/traced_sentinel.rs
// `ring_mpsc::UNSTAMPED` is `Seq( u64::MAX )` — a public constant meaning "this
// slot has never been published". Recording it needs no ring and no 2^64
// publications; it needs one `use`.
let trace = Trace::enabled();
trace.record( TraceOp::Publish, UNSTAMPED, 1 );
let entry = trace.entries()[ 0 ];
match panic::catch_unwind( || entry.to_string() )
{
  Ok( shown ) => println!( "  and printing the log line gives   {shown}" ),
  Err( _ ) => println!( "  and printing the log line           panicked" ),
}
```

```
  debug_assertions: true
  ring_mpsc::UNSTAMPED is Seq(18446744073709551615)
  recording it succeeds; the log holds 1 entry
  and printing the log line           panicked
  an ordinary entry prints as      publish 5..8

  debug_assertions: false
  ring_mpsc::UNSTAMPED is Seq(18446744073709551615)
  recording it succeeds; the log holds 1 entry
  and printing the log line gives   publish 18446744073709551615..0
  an ordinary entry prints as      publish 5..8
```

---

### TR41 — The Failure Lands in the Diagnostic's Own Output Line

Three crates in the family already record what a bare `+` on a `Seq` does at the
top of the range: it panics under debug assertions and wraps to zero without
them. What is specific here is not the arithmetic — it is where the result comes
out.

Everywhere else the overflowed value stays inside a computation and corrupts a
number. In this crate it goes through `Display`, which is the only rendering the
log has, and produces `publish 18446744073709551615..0` — a range whose upper
bound is below its lower bound, printed as a line of a trace, read by someone who
is already investigating a failure. Under debug assertions the same line panics
instead, in the middle of formatting, inside a tool that exists to observe a run
that is going wrong.

The value is not hypothetical. `ring_mpsc` publishes `UNSTAMPED : Seq = Seq( u64::MAX )`
as its "never published" sentinel, and the probe shows what a trace of that slot
prints: one `use`, one `record`, one `to_string`. The trace accepts it without
complaint — `record` checks `self.enabled` and nothing else, then pushes whatever
three values it was handed — so the entry is stored and looks ordinary until the
moment someone reads it.

**Finding.** Recorded as a latent hazard because the failure mode is the one the
crate is meant to protect against. A prior design record for the poisoning bug
states the standard exactly — a
trace must not "kill the producer thread it was added to observe while telling
its reader nothing had happened" — and a `Display` that panics mid-format does
the first while a range that reads backwards does the second. The remedy is
small and local: `end` returning `Seq( self.seq.0.saturating_add( self.count as u64 ) )`,
or `Display` printing the count rather than a computed bound when the addition
would carry. Either keeps the log printable at the one value the family hands out
by name.

```sh
cd "$(git rev-parse --show-toplevel)"
CARGO_TARGET_DIR=/tmp/ring_trace_verify cargo test -p ring_trace --all-features end_saturates_instead_of_reading_backwards_at_the_top_of_u64 -- --nocapture 2>&1 | command grep -E 'end_saturates|test result'
```

Live output:

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test end_saturates_instead_of_reading_backwards_at_the_top_of_u64 ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s
```

**Disposition:** applied — `end` now returns exactly the remedy this finding
names: `Seq( self.seq.0.saturating_add( self.count as u64 ) )`. Tracing
`ring_mpsc::UNSTAMPED` no longer panics under debug assertions or wraps to a
backwards-reading range in release; `end()` saturates at `Seq( u64::MAX )` and
`Display` prints `publish 18446744073709551615..18446744073709551615` either
way. Added `end_saturates_instead_of_reading_backwards_at_the_top_of_u64` to
`tests/trace_test.rs`, pinning both the saturated `end()` value and the
rendered string at the exact sentinel the probe above used. Verified via
`cargo test -p ring_trace --all-features`, 2026-09-04 — ring_trace's 20 unit
tests plus 9 doctests all pass, including the new test. Now prints:
`test end_saturates_instead_of_reading_backwards_at_the_top_of_u64 ... ok`

---

### TR42 — The Suite Reasoned About the Empty Range and Stopped One Line Short

`an_entry_reports_the_range_it_covers` tests two cases. The first is ordinary —
`count : 3` from `Seq( 5 )` ends at `Seq( 8 )`. The second is deliberate: a
zero-count entry, asserted to end where it starts, with the reason written into
the assertion message, "a zero-count operation covers nothing". Somebody thought
about what a degenerate range means and wrote the conclusion down.

The other degenerate range is one line away and absent. `entries_compare_by_value_and_print_readably`
asserts two `to_string()` values, `"drop 1..2"` and `"claim 8..72"`, both
ordinary. Nothing in the crate tests a `seq` or a `count` near the top of `u64`,
and nothing warns about one: no `# Panics` section anywhere in the file, no
`debug_assert`, and across all 33 crates zero calls to `checked_add`,
`saturating_add` or `wrapping_add` — the family's arithmetic posture is a bare
`+` everywhere, with two crates carrying a `debug_assert` about something else.

**Finding.** Recorded as a coverage gap with an unusually specific shape. This is
not a case of nobody having considered boundary behaviour; the author considered
it, picked the low end, asserted it, and explained it. The high end takes the
same three lines and the same `TraceEntry` literal the test already builds, and
it is the end that changes behaviour by build profile. One assertion pinning what
`Display` does at `Seq( u64::MAX )` would state the crate's actual contract there
— whichever contract TR41's remedy settles on — instead of leaving it as
whatever `u64` addition happens to do under the profile in use.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](../item/001_six_impl_blocks_and_the_three_a_lint_cannot_reach.md) | Why the format is undocumented in the first place |
| [`data_structure/002`](../data_structure/002_three_public_fields_and_the_range_they_imply.md) | The two public fields this reads from |
| [`workaround/001`](../workaround/001_an_addition_the_types_crate_already_offers.md) | The named method `end` open-codes instead |
| [`pitfall/002`](002_a_recovery_no_caller_can_reach.md) | The crate's other failure path, in the opposite direction |
| [`ring_types` `pitfall/001`](../../../ring_types/docs/pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md) | The upstream wrap and the doc that denies it |
| [`ring_index` `pitfall/001`](../../../ring_index/docs/pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md) | The same profile split, and the sentinel that reaches it |

### Sources

| Fact | Where |
|------|-------|
| `end` and the `Display` it feeds | `ring_trace/src/lib.rs:169-181` |
| `record` validating only the flag | `ring_trace/src/lib.rs:256-263` |
| What a traced `UNSTAMPED` prints | Probe above |
| The sentinel's declaration | `ring_mpsc/src/lib.rs:222` |
| No `# Panics`, no checked arithmetic in 33 crates | Census above |
| The two range cases the suite asserts | `ring_trace/tests/trace_test.rs:139-147` |

### Tests

| Test | Covers |
|------|--------|
| `an_entry_reports_the_range_it_covers` | The ordinary range and the empty one |
| `entries_compare_by_value_and_print_readably` | Two ordinary `Display` outputs |
| `a_batch_is_one_entry_carrying_its_count_not_n_entries` | The batch case `end` exists for |
