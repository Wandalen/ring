# Data Structure: Three Public Fields and the Range They Imply

### Scope

**Purpose:** Record that a `TraceEntry` has one construction site inside the
crate and seven outside it, establish that the two routes are equivalent because
there is nothing to validate, and identify the dimension the three fields do not
carry.

**Responsibility:** Every `TraceEntry` literal in the workspace, what `record`'s
`count` parameter documents, what a zero count is asserted to mean, and whether
anything in the crate records time.

**In Scope:** `ring_trace/src/lib.rs:128-158`, `:256-263`;
`ring_trace/tests/trace_test.rs:139-161`.

**Out of Scope:** What `end()` does at the top of the range is
[`pitfall/001`](../pitfall/001_a_range_that_reads_backwards.md). The layout cost
of those fields is
[`data_structure/001`](001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md).

---

## One Front Door, Seven Side Entrances

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the only place the crate builds an entry itself --'
command grep 'TraceEntry {' ring_trace/src/lib.rs
echo '  -- and every hand-built entry in the crates docs and tests --'
command grep -r 'TraceEntry {' --include=*.rs . | sed 's/^ring\///'
echo '  -- what a zero count is tested to mean --'
command grep -m1 -A1 -F '  let empty = TraceEntry { op : TraceOp::Publish, seq : Seq( 5 ), count : 0 };' ring_trace/tests/trace_test.rs
echo '  -- what record says about its own count argument --'
command grep 'count : usize' ring_trace/src/lib.rs
echo '  -- and whether anything in the crate records time --'
command grep -c 'Instant\|nanos\|Duration\|SystemTime' ring_trace/src/lib.rs || true
```

Live output:

```
  -- the only place the crate builds an entry itself --
/// let entry = TraceEntry { op : TraceOp::Claim, seq : Seq( 8 ), count : 64 };
    self.entries_guard().push( TraceEntry { op, seq, count } );
  -- and every hand-built entry in the crates docs and tests --
ring_trace/tests/trace_test.rs:  let expected : Vec< TraceEntry > = ( 0..50u64 ).map( | i | TraceEntry { op : TraceOp::Publish, seq : Seq( i ), count : 1 } ).collect();
ring_trace/tests/trace_test.rs:  let entry = TraceEntry { op : TraceOp::Publish, seq : Seq( 5 ), count : 3 };
ring_trace/tests/trace_test.rs:  let empty = TraceEntry { op : TraceOp::Publish, seq : Seq( 5 ), count : 0 };
ring_trace/tests/trace_test.rs:  let a = TraceEntry { op : TraceOp::Drop, seq : Seq( 1 ), count : 1 };
ring_trace/tests/trace_test.rs:  let b = TraceEntry { op : TraceOp::Drop, seq : Seq( 1 ), count : 1 };
ring_trace/tests/trace_test.rs:  let c = TraceEntry { op : TraceOp::Claim, seq : Seq( 1 ), count : 1 };
ring_trace/tests/trace_test.rs:  assert_eq!( TraceEntry { op : TraceOp::Claim, seq : Seq( 8 ), count : 64 }.to_string(), "claim 8..72" );
ring_trace/tests/trace_test.rs:  let entry = TraceEntry { op : TraceOp::Publish, seq : Seq( u64::MAX ), count : 1 };
ring_trace/src/lib.rs:/// let entry = TraceEntry { op : TraceOp::Claim, seq : Seq( 8 ), count : 64 };
ring_trace/src/lib.rs:    self.entries_guard().push( TraceEntry { op, seq, count } );
  -- what a zero count is tested to mean --
  let empty = TraceEntry { op : TraceOp::Publish, seq : Seq( 5 ), count : 0 };
  assert_eq!( empty.end(), Seq( 5 ), "a zero-count operation covers nothing" );
  -- what record says about its own count argument --
  pub count : usize,
  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )
  -- and whether anything in the crate records time --
1
```

---

### TR11 — The Front Door Validates Nothing, So There Is No Front Door

`record` is the method the whole crate is organised around, and the entry it
builds at `:262` is `TraceEntry { op, seq, count }` — the three arguments,
unexamined, moved into the struct. Every field is `pub`, so struct-literal
syntax reaches the same place with the same freedom, and that is what the crate
itself does everywhere it is not going through a `Trace`: eight literals in the
test file and one in the type's own doctest.

**Correction (2026-09-28):** this paragraph read "six literals in the test
file and one in the type's own doctest"; the census above now lists eight. Most
recently, a `weak_len_assert_sweep_1633` fix added `let expected : Vec<
TraceEntry > = ( 0..50u64 ).map( ... ).collect()` to strengthen the "one entry
per operation" test — and the count was already one higher than six before that
fix, from a pre-existing `u64::MAX` boundary-case literal the original count
had missed. The doctest count (one) and both findings below are unaffected —
the extra literals are more of the same struct-literal freedom, not a new front
door.

The two routes being equivalent is a consequence of there being nothing to
check. A `count` of zero is legal, and the suite pins what it means — "a
zero-count operation covers nothing" — for the `end()` computation. Whether a
`record( TraceOp::Claim, Seq( 5 ), 0 )` is a meaningful thing to log, and what a
reader should conclude on finding one, is not stated at either the field or the
parameter; the field's doc is one line, "How many consecutive sequences the
operation covered", and `record`'s parameter carries no doc of its own.

**Finding.** This is fine as a design and thin as documentation. The type is a
plain record and should be constructible as one, so making the fields private
would cost the tests their readable literals and buy nothing. What is missing is
the sentence that would make the record self-describing: that `count` is the
number of consecutive sequences the operation covered, that a claim of sixty-four
is one entry rather than sixty-four, and that a count of zero denotes an
operation that covered no sequences — which is exactly the reading the test
asserts and the only place it is written down.

---

### TR12 — A Trace With Order and No Time

The crate's own account of what it is for draws the line at ordering: a counter
answers "how many publishes" and "tells you nothing about which sequences or in
what order"; a trace "answers 'which operations, in what order'". Both halves of
that are true, and the entry carries exactly what they require — an operation
kind, a starting sequence, a span.

There is no timestamp. The census finds zero occurrences of `Instant`, `Duration`,
`SystemTime` or `nanos` in the whole crate, so a recorded log establishes that
publish followed claim and nothing about how long the gap was. The neighbour
covers the other half in aggregate — `ring_stats` carries `wait_nanos` — but that
is a sum across waiters, not a duration attributable to any operation in this
log, so the pair gives counts, totals and order without ever giving the elapsed
time of a single recorded event.

**Finding.** The absence is defensible and probably right: a timestamp would add
eight or sixteen bytes to a twenty-four-byte entry, and reading a clock on the
producers' path is exactly the kind of cost this crate is careful about
elsewhere. What is not right is that the boundary is invisible. The module doc
draws a two-way comparison against `ring_stats` and presents "which, in what
order" as what the trace adds, without saying that the question a reader most
often has next — *how long did that take* — is answerable by neither tool. One
clause in the trace-against-stats section naming duration as outside both would
close it, and would also record the reason, which is that the clock read is not
affordable on this path.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md) | What those three fields occupy |
| [`pitfall/001`](../pitfall/001_a_range_that_reads_backwards.md) | What the two numeric fields can be made to produce |
| [`invariant/002`](../invariant/002_the_order_is_the_thing_a_counter_lacks.md) | The ordering the entry does carry |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | The `ring_stats` boundary this one runs along |

### Sources

| Fact | Where |
|------|-------|
| The single internal construction site | `ring_trace/src/lib.rs:262` |
| Eight hand-built literals plus a doctest | Census above |
| `count`'s one-line field doc | `ring_trace/src/lib.rs:156` |
| The zero-count assertion | `ring_trace/tests/trace_test.rs:145-146` |
| No time-bearing item anywhere in the crate | Census above |
| The trace-against-stats framing | `ring_trace/src/lib.rs:14-21` |

### Tests

| Test | Covers |
|------|--------|
| `an_entry_reports_the_range_it_covers` | Both hand-built forms, including the zero count |
| `entries_compare_by_value_and_print_readably` | Three more literals, and what equality means over them |
| `a_batch_is_one_entry_carrying_its_count_not_n_entries` | The same fields built through `record` instead |
