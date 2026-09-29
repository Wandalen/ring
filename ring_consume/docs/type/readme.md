# type

What the two types commit to, at the type level rather than the behavioural one.
`Available` commits to being cheap and total — five of six items `const`, no
lifetime, `Copy`. `Consumer` commits to being a *view*: one lifetime, no owned
state, no `Copy`.

The crate's `const` surface is maximal — every item the language permits to be
`const` is, and the one exception is provable. That audit is worth running
because the identical audit fails one crate down, in `ring_seqno`
([`workaround/001`](../workaround/001_five_functions_none_const.md)).

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [`Available`'s Const Surface](001_availables_const_surface.md) | CN47, CN48 — a maximal `const` surface with a provable exception, and the niche `Seq( pub u64 )` forecloses |
| 002 | [The Lifetime on `Consumer`](002_the_lifetime_on_consumer.md) | CN49, CN50 — one lifetime unifying two independent borrows for free, and a withheld `Copy` that its own accessors bypass |

### What Each Type Commits To

| | `Available` | `Consumer< 'a >` |
|--|-------------|------------------|
| Lifetime | none | one, covering both fields |
| Derives | `Debug, Clone, Copy, PartialEq, Eq` | `Debug` |
| `const` items | 5 of 6 | 3 of 8 |
| Owned state | two integers | none |
| Constructible by a caller | ✔ — `new` is public | ✔ — `new` is public |
| Duplicable | ✔ — `Copy` | ✔ — via its own accessors |

`Available` deliberately does not derive `Ord` (two runs are equal or they are
not; there is no ordering on ranges anyone wants by default) and `Consumer`
deliberately does not derive `Copy`. The first is unambiguously right. The second
is right and much narrower than it looks.

### The Ownership Asymmetry

```rust
pub struct Claimer< 'a >          pub struct Consumer< 'a >
{                                 {
  cursor : PaddedCursor,            cursor : &'a PaddedCursor,
  consumers : &'a GatingSet,        barrier : Barrier< 'a >,
}                                 }
```

Same role, opposite ownership — and it follows from the wiring rather than from a
principle. A producer is the sole writer of its own cursor and can own it; a
consumer's cursor must be reachable by the producer's gating set, so it has to be
lent.

The consequence is that `Claimer` is structurally single-instance and `Consumer`
is not. Three `Consumer`s over one cursor compile and run, and the third can be
built entirely from the first's own accessors:

```rust
let c = Consumer::new( a.cursor(), a.barrier() );   // reconstructs a non-Clone type
```

### Where `Seq`'s Shape Reaches This Crate

`Seq( pub u64 )` — public field, no `From`, no `Deref`, no `Into` — produces two
opposite consequences here, and the corpus records both because they trade
against each other:

| Consequence | Where |
|-------------|-------|
| `Option< Seq >` is 16 bytes where a niche would give 8 | `frontier()`'s return, on the hot path — [001](001_availables_const_surface.md) CN48 |
| `.0` is available as an escape, which `sequences()` requires | [`workaround/001`](../workaround/001_five_functions_none_const.md) CN52 |

Closing the first requires a private field, which removes the second. Neither
direction is documented anywhere.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the const boundary, and every item on each side of it
grep -E '^\s*pub (const )?fn' ring_consume/src/lib.rs

# the derives that were taken and the ones that were not
grep -B1 'pub struct Available\|pub struct Consumer' ring_consume/src/lib.rs
grep -B1 'pub struct Barrier'  ring_barrier/src/lib.rs
grep -B1 'pub struct Claimer'  ring_claim/src/lib.rs

# Seq's declaration: public field, and what it forecloses
grep -B1 'pub struct Seq' ring_types/src/id.rs
grep -rE 'impl.*(From|Deref|Into).*Seq' ring_types/src/id.rs || echo "  no conversion"

# every public type in the family carrying a lifetime, and the one with two
command grep -rE '^pub struct [A-Za-z]+< ' ring_*/src/*.rs | sed 's|ring/||'

# const density across the family, for context
for c in ring_*/src/lib.rs; do
  printf '%-28s %2d const / %2d total\n' "$( echo "$c" | sed 's|ring/||;s|/src/lib.rs||' )" \
    "$( grep -cE '^\s*pub const fn' "$c" )" "$( grep -cE '^\s*pub (const )?fn' "$c" )"
done
```

Live output:

```
  pub const fn new( start : Seq, len : u64 ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> u64
  pub const fn is_empty( self ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  pub const fn cursor( &self ) -> &'a PaddedCursor
  pub const fn barrier( &self ) -> Barrier< 'a >
  pub fn position( &self ) -> Seq
  pub fn available( &self ) -> Available
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct Available
--
#[ derive( Debug ) ]
pub struct Consumer< 'a >
#[ derive( Debug, Clone, Copy ) ]
pub struct Barrier< 'a >
#[ derive( Debug ) ]
pub struct Claimer< 'a >
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct Seq( pub u64 );
  no conversion
ring_align/src/lib.rs:pub struct CacheAligned< T >( T );
ring_barrier/src/lib.rs:pub struct Barrier< 'a >
ring_store/src/lib.rs:pub struct Buffer< S >
ring_claim/src/lib.rs:pub struct Claimer< 'a >
ring_consume/src/lib.rs:pub struct Consumer< 'a >
ring_core/src/lib.rs:pub struct Ring< T >
ring_core/src/lib.rs:pub struct Ends< 'a, T >
ring_core/src/lib.rs:pub struct Producer< 'a, T >
ring_core/src/lib.rs:pub struct Consumer< 'a, T >
ring_flush/src/lib.rs:pub struct Flusher< 'a, T >
ring_handle/src/lib.rs:pub struct Split< T >
ring_handle/src/lib.rs:pub struct Ends< 'a, T >
ring_handle/src/lib.rs:pub struct Producer< 'a, T >
ring_handle/src/lib.rs:pub struct Consumer< 'a, T >
ring_handle/src/lib.rs:pub struct Drain< 'c, 'a, T >
ring_mpsc/src/lib.rs:pub struct Ring< S >
ring_mpsc/src/lib.rs:pub struct Ends< 'a, S >
ring_mpsc/src/lib.rs:pub struct Producer< 'a, S >
ring_mpsc/src/lib.rs:pub struct Reserved< 'a, S >
ring_mpsc/src/lib.rs:pub struct Consumer< 'a, S >
ring_mpsc/src/lib.rs:pub struct Batch< 'a, S >
ring_registry/src/lib.rs:pub struct Registry< T >
ring_shutdown/src/lib.rs:pub struct Stopped< 'a >
ring_shutdown/src/lib.rs:pub struct Guarded< 'a, T >
ring_slot/src/lib.rs:pub struct TypedSlot< T >( Option< T > );
ring_slot/src/lib.rs:pub struct BytesSlot< const N : usize >
ring_spsc/src/lib.rs:pub struct Ring< S >
ring_spsc/src/lib.rs:pub struct Producer< 'a, S >
ring_spsc/src/lib.rs:pub struct Reservation< 'a, S >
ring_spsc/src/lib.rs:pub struct Consumer< 'a, S >
ring_spsc/src/lib.rs:pub struct Batch< 'a, S >
ring_tls/src/lib.rs:pub struct TlsBuffer< T >
ring_tls/src/lib.rs:pub struct Flush< 'a, T >
ring_align                    4 const /  5 total
ring_atomic                   2 const /  6 total
ring_barrier                  4 const /  9 total
ring_batch                    7 const / 11 total
ring_bench                   25 const / 40 total
ring_store                   3 const / 12 total
ring_claim                    9 const / 15 total
ring_config                  11 const / 12 total
ring_consume                  8 const / 14 total
ring_core                     2 const / 16 total
ring_cursor                   5 const / 13 total
ring_debug                    0 const /  5 total
ring_event                    0 const /  3 total
ring_factory                  0 const /  3 total
ring_flush                    3 const / 17 total
ring_gating                   1 const / 11 total
ring_handle                   1 const / 12 total
ring_index                    0 const /  3 total
ring_mpsc                     7 const / 29 total
ring_overflow                 3 const /  4 total
ring_poll                    12 const / 20 total
ring_publish                  1 const /  6 total
ring_registry                 0 const /  8 total
ring_seqno                      0 const /  5 total
ring_shutdown                 7 const / 23 total
ring_slot                     6 const / 10 total
ring_spsc                     5 const / 23 total
ring_stats                    2 const / 16 total
ring_testkit                  0 const / 11 total
ring_tls                      2 const / 10 total
ring_trace                    5 const / 11 total
ring_types                    0 const /  0 total
ring_wait                     1 const /  6 total
```

Sizes (`Seq` 8, `Option< Seq >` 16, `Option< NonZeroU64 >` 8) come from a
`size_of` probe; the constness negatives come from compiling the two bodies with
`const` added. Both outputs are quoted in [001](001_availables_const_surface.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN47 | `ring_consume` | n/a — observation | Every item the language permits to be `const` is `const`; the one exception is blocked by `Iterator::map` and the compile error proves it |
| CN48 | `ring_types` | n/a — observation | `Seq( pub u64 )` forecloses the niche, doubling `Option< Seq >` on the hot path, and the same public field is the escape hatch `sequences()` needs |
| CN49 | `ring_consume` | n/a — observation | The single `'a` unifies two independent borrows by variance and costs a caller nothing; the signature reads as a constraint and is not one |
| CN50 | `ring_consume` | n/a — doc gap | Withholding `Copy` blocks only the accidental duplicate; `cursor()` and `barrier()` reconstruct the type in one line, and the derive reads as protection of the single-consumer invariant |
