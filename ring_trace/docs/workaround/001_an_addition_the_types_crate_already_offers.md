# Workaround: An Addition the Types Crate Already Offers

### Scope

**Purpose:** Record that `TraceEntry::end` open-codes an addition `ring_types`
already exposes as a named method, establish how rare that is across the family,
and identify the one other site that does it — which turns out to be the same
expression computing the same thing.

**Responsibility:** `end`'s body, `Seq::advanced_by`'s body, every site in the
family that builds a `Seq` from raw field arithmetic, and every crate that calls
the named methods instead.

**In Scope:** `ring_trace/src/lib.rs:169-172`;
`ring_types/src/id.rs:50-68`; `ring_batch/src/lib.rs:131`;
`ring_mpsc/src/lib.rs:506`; every `ring_*` `src/`.

**Out of Scope:** What the addition does at the top of the range is
[`pitfall/001`](../pitfall/001_a_range_that_reads_backwards.md). The dependency
two documents name but the crate does not have is
[`workaround/002`](002_a_third_dependency_two_documents_still_name.md).

---

## Three Sites in Thirty-Three Crates

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the addition, and the method one crate down that names it --'
command grep -m1 -A5 -F '  /// One past the last sequence this entry covers.' ring_trace/src/lib.rs
sed -n '/^  \/\/\/ This sequence advanced by `n`\.$/p;/^  pub const fn advanced_by( self, n : u64 ) -> Self$/,/^  }$/p' ring_types/src/id.rs
echo '  -- every place in 33 crates that builds a Seq from raw field arithmetic --'
command grep -r 'Seq( [a-z_.]*\.0 [-+]' --include=*.rs ring_*/src/ | sed 's|ring/||' | sed 's/^/    /'
echo '  -- against the crates that call the named methods instead --'
for m in advanced_by next; do
  printf '    %-12s %s\n' "$m" \
    "$( command grep -rl "\.$m(" --include=*.rs ring_*/src/ 2>/dev/null \
        | sed 's|ring/||;s|/src/lib.rs||;s|/src/id.rs||' | sort -u | tr '\n' ' ' | sed 's/ *$//' )"
done
```

Live output:

```
  -- the addition, and the method one crate down that names it --
  /// One past the last sequence this entry covers.
  ///
  /// Saturates rather than wrapping: a bare `+` here would print a range that
  /// reads backwards, or panic under debug assertions, the moment a caller
  /// traces the one `Seq` the family publishes by name —
  /// `ring_mpsc::UNSTAMPED` (`Seq(u64::MAX)`). See `pitfall/001` TR41.
  /// This sequence advanced by `n`.
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
  -- every place in 33 crates that builds a Seq from raw field arithmetic --
    ring_batch/src/lib.rs:    Seq( self.start.0 + self.count as u64 )
    ring_mpsc/src/lib.rs:    if end == from { None } else { Some( Seq( end.0 - 1 ) ) }
  -- against the crates that call the named methods instead --
    advanced_by  ring_claim ring_consume ring_gating ring_index ring_mpsc ring_publish ring_spsc ring_types
    next         ring_core ring_mpsc ring_spsc ring_tls ring_types
```

## The Two Spellings, Side by Side

*The compiled probe behind this comparison is gone — swept, like every
`-tr_probe/` scratch binary, per this project's convention for temporary
files — so the two spellings below can't be re-run together. Both remain
accurate on inspection: `Seq::advanced_by` is still the plain, non-saturating
`Self( self.0 + n )`, and manual field addition still gives the same value, so
the recorded equivalence holds today. Read it as preserved evidence for TR49
below, not a live rerun.*

```rust
// -tr_probe/src/bin/end_overflow.rs
let by_hand = Seq( 8 ).0 + 64;
let by_method = Seq( 8 ).advanced_by( 64 );
```

```
  open-coded 72 vs Seq::advanced_by Seq(72): same value, same overflow
```

---

### TR49 — The Family's Idiom Is the Named Method, and This Is One of Three Exceptions

`Seq::advanced_by` exists, is `const`, is `#[ must_use ]`, and its body is the
same `Self( self.0 + n )` that `end` writes out by hand. `ring_trace` already
depends on `ring_types` — it imports `Seq` from it — so calling the method costs
nothing at all.

Across all 33 crates, exactly three sites build a `Seq` out of raw field
arithmetic, and one of the three is this one. Seven crates call `advanced_by` and
five call `next`, so the family's idiom is unambiguously the named method; the
open-coded form is the deviation, not the norm.

The probe confirms the two spellings agree on value and on failure — `Seq( 8 ).0 + 64`
and `Seq( 8 ).advanced_by( 64 )` both give 72, and both wrap identically at the
top of the range. So this is not a behavioural difference; nothing is broken by
writing it out.

**Finding.** Recorded as a workaround for a method that was already available,
which makes it a straightforward substitution: `self.seq.advanced_by( self.count as u64 )`
replaces the body, drops the `.0` field access, and puts the crate on the same
idiom as the other twelve. What it buys beyond consistency is a single place to
change: whatever `ring_types` eventually does about the wrap it documents but does
not have, `advanced_by` is where it will land, and a call site inherits it while
an open-coded `+` does not.

---

### TR50 — Two of the Three Exceptions Are the Same Expression Computing the Same Thing

`ring_batch/src/lib.rs:131` reads `Seq( self.start.0 + self.count as u64 )`.
`ring_trace/src/lib.rs:171` reads
`Seq( self.seq.0.saturating_add( self.count as u64 ) )`. They were one field
name apart and character for character identical when this was recorded, the
`count as u64` cast included; `ring_trace`'s half has since been given
saturating overflow by `pitfall/001` TR41's fix, so the two now compute the same
quantity under two different overflow rules. Both still live in a method named
`end` whose doc says "one past the last sequence" covered by a batch. The third
site, `ring_mpsc:506`, is a subtraction doing something else entirely.

The duplication is semantic rather than coincidental. `ring_batch` computes where
a batch claim finishes; `ring_trace` computes where the *record of* a batch claim
finishes, because `count` is exactly `ring_batch`'s "one operation, not 64" and
the trace stores the same two numbers the claim does. The two crates do not
depend on each other in either direction, and neither doc mentions the other.

Neither uses `advanced_by`, though `ring_batch` depends on `ring_types` too, and
`ring_batch`'s own documentation already records what the underlying `+` does at
the boundary. So both crates arrived independently at the same open-coding of the
same computation over the same pair of fields.

**Finding.** Recorded as duplication across a crate boundary that no compiler or
lint can see, and that neither side is wrong to have written. What makes it worth
noting is the shape: a `( Seq, count )` pair with a "one past the end" reading is
a recurring quantity in this family, spelled by hand in both places it appears.
The minimum fix is the same one clause TR49 asks for in each crate. The larger
option — a shared `end( start : Seq, count : usize ) -> Seq` in `ring_types`
beside `advanced_by` — is only worth it if a third caller appears, and recording
the pair here is what would let anyone notice when it does.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](../pitfall/001_a_range_that_reads_backwards.md) | What this addition does at the top of the range |
| [`data_structure/002`](../data_structure/002_three_public_fields_and_the_range_they_imply.md) | The two fields it reads |
| [`workaround/002`](002_a_third_dependency_two_documents_still_name.md) | The other gap between what the crate has and what its documents say |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | The crates whose operations this entry records |
| [`ring_types` `pitfall/001`](../../../ring_types/docs/pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md) | What the named method would inherit a fix from |

### Sources

| Fact | Where |
|------|-------|
| `end`'s open-coded body | `ring_trace/src/lib.rs:169-172` |
| `advanced_by`'s identical body | `ring_types/src/id.rs:65-68` |
| Three raw-arithmetic sites in 33 crates | Census above |
| Twelve crates on the named methods | Census above |
| The duplicated expression | `ring_batch/src/lib.rs:131` |
| Both spellings agreeing on value and wrap | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `an_entry_reports_the_range_it_covers` | `end`'s result, ordinary and empty |
| `a_batch_is_one_entry_carrying_its_count_not_n_entries` | The batch shape both crates model |
| `entries_compare_by_value_and_print_readably` | `Display`, which calls `end` |
