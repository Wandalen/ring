# workaround

Three constraints this crate absorbs from below, and the one that cost anything
was not the one that looked expensive. `ring_seqno` declining `const` costs
nothing here. `Seq` having no conversion costs one line. A `Vec` built to change
a slice's element type cost a heap allocation on every read the crate performed.

All three were recorded with a deletion condition, because a workaround without
one is just a complaint. One of the three deletion conditions has since been
met, from two crates away and for unrelated reasons: the `Vec` is gone
([002](002_a_vector_to_change_a_slices_type.md) CN53, CN54). The other two are
open.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Five Functions, None `const`](001_five_functions_none_const.md) | CN51, CN52, CN55 — four const-able bodies left non-`const`, `.0` as the only exit from a `Seq`, and the one avoidable escape of the 84, whose recorded remedy its own crate declined |
| 002 | [A Vector That Changed a Slice's Type](002_a_vector_to_change_a_slices_type.md) | CN53, CN54 — an allocation that computed nothing, and which of the three named ways out actually shipped |

### The Three Escapes, Side by Side

| | `ring_seqno` non-`const` | `Seq` has no conversion | the adapting `Vec` |
|--|------------------------|-------------------------|--------------------|
| Owner | `ring_seqno` | `ring_types` | `ring_cursor` |
| This crate's response | call at runtime | `.0`, re-wrapped immediately | paid the allocation |
| Cost here | **none** | **none** | **was 1 alloc per read call, now none** |
| Cost elsewhere | no compile-time capacity arithmetic, family-wide | 84 escapes in 15 files, some gratuitous | was the same allocation on `ring_claim`'s path; one change closed both |
| Deletion condition | add `const` to four unchanged bodies | `impl Step` (nightly) or an `advanced_by` loop | **met** — `slowest` inlined its `.min()` |

### Why Each Is Hard to Notice

**The `const` gap is invisible from above.** `ring_consume`'s own surface is
maximally `const` ([`type/001`](../type/001_availables_const_surface.md) CN47),
so the audit passes here and there is no local symptom. It shows only when the
same audit is run one crate down and four functions turn out to be const-able
with their bodies untouched — proven by compiling them in `const` position.

**The `.0` escape looks the same wherever it appears, and is not the same
thing.** This crate's uses are structurally required: a `Range` needs an
integer, `Seq` is not `Step`, and the value is re-wrapped on the next token —
three escapes, two of them on that one `Range` and the third in the doctest
exercising it. `ring_batch` writes the identical `Range` line, and eight more
across `contains` and `overlaps` for comparisons `Seq`'s derived `Ord` looks
able to handle.

Those eight are not gratuitous, and this document said they were for as long as
it existed. Both methods are `pub const fn`, and a derived `PartialOrd` is not a
`const` impl — `seq >= self.start` inside either one is `error[E0015]: cannot
call non-const operator in constant functions`, compiled and printed by
[001](001_five_functions_none_const.md). The escape is what makes the method
`const` at all. The sibling finding on this page proves four `ring_seqno` bodies
const-able by compiling them; the same two lines pointed one crate sideways
would have refuted this one on the day it was written.

The escape in `ring_batch` that *is* avoidable is `end`'s
`Seq( self.start.0 + self.count as u64 )`, where `Seq::advanced_by` is
`pub const fn` with the body `Self( self.0 + n )`. CN52 never reached it;
`ring_trace`'s corpus did, and recorded it as TR49 and TR50. What this document
adds is what became of that: `ring_trace`'s own `end` declined the substitution
and saturates instead, its source comment says why, and both findings still
prescribe the un-declined form ([001](001_five_functions_none_const.md) CN55).
Both bodies are printed below. Neither change belongs to this crate.

**The allocation hid behind the case worth measuring first.** With an empty
barrier `frontier()` returns `None`, so nothing was allocated. With one
dependency, 1000 calls allocated 1000 times. The cheap check was the one that
reported clean — the same reason it survived in `ring_claim`
([`pitfall/002`](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md)).

### The One That Was Worth Fixing

```rust
// ring_cursor::slowest, as this document found it
let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
ring_seqno::slowest( &positions )

// ring_cursor::slowest, as it stands
cursors.iter().map( | c | c.load( GATING ) ).min()
```

The `Vec` existed so that a `&[ Seq ]` existed to pass. Measured against the
inline equivalent: identical answers including the empty case, one allocation
against zero. Of the three ways out
[002](002_a_vector_to_change_a_slices_type.md) priced, the one that shipped
changed no signature anywhere: `ring_cursor::slowest` folds the loads itself.
The iterator-taking `ring_seqno::slowest` was not built and would no longer pay
for itself, because the allocation it was for is already gone.

What it cost is exactly what CN54 said it bought. `ring_seqno::slowest` still
exists and is still correct, and now has no caller outside its own tests and
doctest — the recipe below prints both bodies, and they are the same fold
written twice. The dependency survives the loss of its reuse: `ring_cursor`
still declares `ring_seqno` and still calls into it from three other methods.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# const-ness one crate down, against the crate below that
command grep -E '^\s*pub (const )?fn' ring_seqno/src/lib.rs
command grep -rE '^\s*pub (const )?fn' ring_types/src/ | sed 's|ring_types/src/||'

# the bodies, to check what would block const. unnumbered deliberately: the
# filter runs first, so a `-n` here would number the *filtered* stream —
# offsets that look like source addresses and point nowhere
command grep -vE "^[[:space:]]*//" ring_seqno/src/lib.rs \
  | command grep -E -A6 'pub fn'

# Seq has no conversion: the only exit is the field
command grep -rE 'impl.*(From|Deref|Into).*Seq' ring_types/src/id.rs || echo "  none"
printf 'escapes across the family: %s in %s file(s)\n' \
  "$( command grep -rho '[a-z_]*\.0' ring_*/src/*.rs | wc -l )" \
  "$( command grep -rl '\.0' ring_*/src/*.rs | wc -l )"

# this crate's escapes, and the gratuitous ones. the per-crate totals count
# occurrences and the listings show lines, so a line carrying four of them
# reads as one line above and as four here
for c in ring_consume ring_batch; do
  printf '%-14s %s escape(s)\n' "$c" \
    "$( command grep -o '[a-z_]*\.0' ring/$c/src/lib.rs | wc -l )"
done
command grep '\.0' ring_consume/src/lib.rs
command grep '\.0' ring_batch/src/lib.rs

# the one escape that is avoidable: `end`'s arithmetic against the const
# constructor that already does exactly it
command grep -A3 'pub const fn end' ring_batch/src/lib.rs
command grep -A3 'pub const fn advanced_by' ring_types/src/id.rs

# the allocation's deletion condition, and the one-line alternative it met.
# unnumbered for the same reason as the bodies above
command grep -vE "^[[:space:]]*//" ring_cursor/src/lib.rs \
  | command grep -A4 'pub fn slowest'
command grep -vE "^[[:space:]]*//" ring_seqno/src/lib.rs \
  | command grep -A3 'pub fn slowest'
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
error.rs:  pub const fn is_configuration( self ) -> bool
error.rs:  pub const fn is_transient( self ) -> bool
capacity.rs:  pub const fn new( slots : usize ) -> Result< Self, RingError >
capacity.rs:  pub const fn get( self ) -> usize
capacity.rs:  pub const fn mask( self ) -> usize
id.rs:  pub const fn next( self ) -> Self
id.rs:  pub const fn advanced_by( self, n : u64 ) -> Self
id.rs:  pub const fn distance_to( self, later : Self ) -> u64
id.rs:  pub const fn get( self ) -> usize
policy.rs:  pub const fn is_non_blocking( self ) -> bool
policy.rs:  pub const fn reports_failure( self ) -> bool
policy.rs:  pub const fn drops_silently( self ) -> bool
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
{
  earlier.distance_to( later ) / capacity.get() as u64
}

#[ must_use ]
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
{
  consumer.distance_to( producer ) < capacity.get() as u64
}

#[ must_use ]
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}

#[ must_use ]
pub fn pending( producer : Seq, consumer : Seq ) -> u64
{
  consumer.distance_to( producer )
}

#[ must_use ]
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
  none
escapes across the family: 84 in 15 file(s)
ring_consume   3 escape(s)
ring_batch     12 escape(s)
  /// let seen : Vec< u64 > = Available::new( Seq( 5 ), 2 ).sequences().map( | s | s.0 ).collect();
    ( self.start.0..self.end().0 ).map( Seq )
  /// In a debug build, if `start.0 + count` overflows `u64` — unreachable via
    Seq( self.start.0 + self.count as u64 )
    seq.0 >= self.start.0 && seq.0 < self.end().0
    ( self.start.0..self.end().0 ).map( Seq )
      && self.start.0 < other.end().0 && other.start.0 < self.end().0
  pub const fn end( &self ) -> Seq
  {
    Seq( self.start.0 + self.count as u64 )
  }
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}

pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
```

The four `const` evaluations, the two compile errors, and the 1-vs-0 allocation
comparison are all quoted at the point they are used in the two instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN51 | `ring_seqno` | n/a — observation | Four of five bodies are const-able unchanged, proven by compile-time evaluation; the fifth is blocked by `slice::iter`, and the crate below is 12/12 `const` |
| CN52 | `ring_types` | n/a — observation | `.0` is the only exit from a `Seq` — 84 escapes in 15 files; this crate's are `Range`-forced, and `ring_batch`'s eight comparison escapes were recorded here as gratuitous and are not — `contains` and `overlaps` are `pub const fn`, where a derived `PartialOrd` is `error[E0015]` |
| CN53 | `ring_cursor` | **measured cost** | The allocation changes a slice's element type and computes nothing; the inline equivalent returns identical answers with zero allocations |
| CN54 | `ring_cursor` | **measured cost** | What it buys is one dependency edge; a `slowest` taking an iterator removes it from four methods here and from `ring_claim`'s path in the same change |
| CN55 | `ring_trace` | **wrong doc** | The one avoidable escape was already TR49/TR50, which say `TraceEntry::end` writes `Self( self.0 + n )` by hand and prescribe `advanced_by`; `end` reads `saturating_add` and its source comment says why, TR49's own quoted output prints that comment, and no Disposition records the divergence |
