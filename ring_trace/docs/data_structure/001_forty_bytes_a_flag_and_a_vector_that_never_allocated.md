# Data Structure: Forty Bytes, a Flag and a Vector That Never Allocated

### Scope

**Purpose:** Record what a `Trace` and a `TraceEntry` actually occupy, establish
that the disabled path costs no heap at all, and price the padding the entry
layout carries at log sizes a diagnostic reaches.

**Responsibility:** The two struct layouts, the field-sum against the measured
size, the disabled trace's allocation behaviour, and what a narrower `count`
would and would not be entitled to assume.

**In Scope:** `ring_trace/src/lib.rs:149-158`, `:202-207`;
`ring_types/src/capacity.rs:23`.

**Out of Scope:** The public fields and what they let a caller build is
[`data_structure/002`](002_three_public_fields_and_the_range_they_imply.md).
The time cost of the disabled path is
[`algorithm/001`](../algorithm/001_the_early_return_that_is_the_whole_feature.md).

---

## The Two Declarations

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the entry --'
command grep -m1 -A9 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_trace/src/lib.rs
echo '  -- the log --'
command grep -m1 -A5 -F '#[ derive( Debug ) ]' ring_trace/src/lib.rs
echo '  -- and the type that bounds a count, in principle --'
command grep 'pub struct Capacity' ring_types/src/capacity.rs
command grep 'pub const fn new\|pub const fn get' ring_types/src/capacity.rs
```

Live output:

```
  -- the entry --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct TraceEntry
{
  /// What happened.
  pub op : TraceOp,
  /// The first sequence involved.
  pub seq : Seq,
  /// How many consecutive sequences the operation covered.
  pub count : usize,
}
  -- the log --
#[ derive( Debug ) ]
pub struct Trace
{
  enabled : bool,
  entries : Mutex< Vec< TraceEntry > >,
}
  -- and the type that bounds a count, in principle --
pub struct Capacity( usize );
  pub const fn new( slots : usize ) -> Result< Self, RingError >
  pub const fn get( self ) -> usize
```

## What They Occupy

*The probe below has no compiled binary left to re-run — the scratch build
under `-tr_probe/` was swept per this project's convention for temporary
files. The layout it measured is unchanged since: `TraceEntry`'s own doc
comment still cites the same 24-byte, 17-byte-of-fields figures this run
produced, and the three fields it sums are the same fields in `src/lib.rs`
today. Read the numbers as the frozen evidence TR9 and TR10 below already
build on, not as something this gate can reconfirm on its own.*

```rust
// -tr_probe/src/bin/entry_layout.rs
/// The same three fields with a `u32` count.
#[ allow( dead_code ) ]
struct NarrowEntry { seq : Seq, count : u32, op : TraceOp }

// A disabled trace, then two enabled ones, filled through the public API.
let off = Trace::disabled();
for i in 0..10_000u64 { off.record( TraceOp::Publish, Seq( i ), 1 ); }
println!( "  disabled after 10,000 records: len {} capacity {}", off.len(), off.entries().capacity() );
```

```
  size_of::<TraceOp>()      1  align 1
  size_of::<Seq>()          8  align 8
  size_of::<usize>()        8  align 8
  size_of::<TraceEntry>()   24  align 8
  sum of the three fields   17  padding 7
  size_of::<NarrowEntry>()  16  with a u32 count
  size_of::<Trace>()        40  align 8
  disabled after 10,000 records: len 0 capacity 0
  enabled, 10000 records: 234 KiB as shipped, 156 KiB with a u32 count
  enabled, 1000000 records: 23437 KiB as shipped, 15625 KiB with a u32 count
```

---

### TR9 — The Disabled Trace Never Allocates, and Nothing Says So

Ten thousand `record` calls against a disabled trace leave the vector at length
zero **and capacity zero**. `Vec::new` does not allocate, `record` returns before
touching it, and so a trace that is switched off costs forty bytes of stack or
struct-embedding and not one byte of heap for the whole life of the program.

That is a stronger property than the criterion asks for and stronger than the
crate claims. The criterion says zero entries. The test file's header says a
trace recording when off "would put a lock and an allocation on the path being
measured", which states the risk avoided rather than the guarantee achieved. No
line in the crate says the guarantee: that a disabled `Trace` performs no
allocation, ever, regardless of how many operations pass through it.

**Finding.** This is the crate's best property and its least documented one. It
is also the one a future change could lose silently — pre-sizing the vector in
`enabled()` for performance would be a natural optimisation and would have to be
written so that `disabled()` does not do the same, with nothing today to say why.
One sentence on `Trace` or on `disabled()`, stating that a disabled trace never
allocates and that this is deliberate, converts a measured accident into a
contract. It is also directly assertable: `assert_eq!( trace.entries().capacity(),
0 )` after a loop of records, which no test does.

---

### TR10 — Seven Bytes in Twenty-Four Are Padding

`TraceEntry`'s three fields sum to seventeen bytes and the struct is
twenty-four. Rust orders the fields itself and puts the eight-aligned `Seq` and
`usize` first, leaving the one-byte discriminant with seven bytes of tail
padding — twenty-nine per cent of every entry. At a million recorded operations,
a size a trace reaches in well under a second of the traffic these rings are
built for, that is 23,437 KiB where the same information needs 15,625.

The saving is available because `count` does not need sixty-four bits. It is
"how many consecutive sequences the operation covered", which a ring bounds by
its capacity. A `u32` count gives a sixteen-byte entry, one third smaller, with
the discriminant fitting in the same word as the count and no padding left over.

**Finding.** The honest framing is that this is a range decision and not a free
win. `ring_types::Capacity` is a newtype over `usize` with power-of-two
validation and no declared ceiling, so nothing in the type system forbids a
capacity above four billion — a ring that would need at least thirty-two
gibibytes of slots, but representable. Narrowing `count` to `u32` therefore means
asserting a bound the family currently leaves open, which is a reasonable thing
to do and a different thing from an oversight. Either the entry should carry the
narrower field with a comment naming the bound it assumes, or `TraceEntry`'s doc
should note that a third of the log is padding and that the crate accepted it to
avoid pinning a capacity ceiling.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A2 -F 'not a narrower' ring_trace/src/lib.rs
```

Live output:

```
/// `count` is a `usize`, not a narrower `u32`, though every count is bounded by
/// a ring's [`Capacity`](ring_types::Capacity) — a `usize` newtype with no
/// declared ceiling. Narrowing here would mean asserting a bound the type it is
```

**Disposition:** applied — the doc-only repair: `TraceEntry`'s doc now states
the padding is a deliberate consequence of not asserting a capacity bound
`ring_types::Capacity` itself does not assert, with the measured 24-vs-17 byte
figures inline. Narrowing `count` to `u32` is a breaking public-field change
gated on committing to that bound family-wide — left to a feature task, not this
disposition. `cargo doc -p ring_trace` with `RUSTDOCFLAGS="-D warnings"` confirms
the new `ring_types::Capacity` intra-doc link resolves cleanly. Now prints: `not a narrower`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_three_public_fields_and_the_range_they_imply.md) | What those fields let a caller construct |
| [`algorithm/001`](../algorithm/001_the_early_return_that_is_the_whole_feature.md) | The other half of what "disabled" costs |
| [`non_functional_requirement/001`](../non_functional_requirement/001_zero_when_not_is_a_count_not_a_cost.md) | The criterion these numbers are measured against |
| [`type/001`](../type/001_five_discriminants_and_the_array_beside_them.md) | The one-byte discriminant that pays the padding |

### Sources

| Fact | Where |
|------|-------|
| `TraceEntry`'s three fields | `ring_trace/src/lib.rs:149-158` |
| `Trace`'s flag and vector | `ring_trace/src/lib.rs:202-207` |
| `Capacity` as an unbounded `usize` newtype | `ring_types/src/capacity.rs:23` |
| 24 bytes with 7 of padding; 16 with a `u32` | Probe above |
| Capacity zero after 10,000 disabled records | Probe above |
| 23,437 KiB against 15,625 KiB at a million | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `a_disabled_trace_records_zero` | The length half of the disabled guarantee |
| `a_batch_is_one_entry_carrying_its_count_not_n_entries` | Why `count` exists at all |
| `entries_compare_by_value_and_print_readably` | The derives the layout carries |
