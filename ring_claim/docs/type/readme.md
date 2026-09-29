# type

Two files for the two halves of `Claimer< 'a >`'s signature: the integers that
move through it, and the borrow that anchors it. `Claim` has no lifetime and
`Claimer` has no arithmetic, so the split falls almost exactly along the two
types — which is itself the crate's shape restated at the type level.

Both files arrive at the same kind of observation, which is what earns this
definition its place next to [`item/`](../item/readme.md) and
[`data_structure/`](../data_structure/readme.md). Those two read the surface and
the memory; this one reads what the *types* commit to, and in both cases the
commitment is narrower than the prose around it. The integer discipline is
followed perfectly and stated nowhere. The lifetime guarantees one thing
precisely and is relied on for three. In neither case is the code wrong; in both
cases a reader who trusts the documentation over the signature is.

The sharpest result is in [002](002_the_lifetime_on_the_claimer.md), and it is
the only place in this crate's corpus where the central invariant can be broken
in safe, single-threaded code: two `Claimer`s over one `GatingSet` compile, and
both grant `Seq( 0 )..Seq( 4 )`. The invariant holds *per claimer*, and nothing
anywhere says so.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Seq, a usize, and Three Casts](001_a_seq_a_usize_and_three_casts.md) | CL47, CL48 — three casts that all widen, the narrowing one two crates away, and the position type this crate never constructs |
| 002 | [The Lifetime on the Claimer](002_the_lifetime_on_the_claimer.md) | CL49, CL50 — one accessor's reference escapes and the other's cannot, and the `Sync` the whole design rests on |

### What Each Type Commits To

| | `Claim` | `Claimer< 'a >` |
|--|---------|-----------------|
| Parameters | none | one lifetime, no type parameter |
| Owns | two integers | a `PaddedCursor` |
| Borrows | nothing | a `GatingSet` for `'a` |
| `Copy` | ✔ | ✘ |
| `Send` / `Sync` | ✔ ✔ — unconditionally | ✔ ✔ — because both its parts are |
| Tied to a ring | **no** | yes, for `'a` |
| Tied to a capacity | **no** | through the gate |
| Can name a slot | **no** — sequences only | no |

The two "no" columns on `Claim` are the crate's boundary written in types, and
they are why the exhaustive `overlaps` test can build 900 claims from literals
with no ring in scope. A range that cannot name a slot cannot be wrong about one.

### Where the Prose Outruns the Signature

Both files find the same failure shape, and it is worth naming once:

| The documentation says | The signature permits | File |
|------------------------|-----------------------|------|
| `cursor()` is "for `ring_publish` to read" | a borrow that dies with the `Claimer` | [002](002_the_lifetime_on_the_claimer.md) |
| `cursor()` is for "a gating set … to be built against" | same — it cannot be stored | [002](002_the_lifetime_on_the_claimer.md) |
| `claim_up_to`'s `Full` means "not even one slot is free" | `Full` at `max = 0` on an empty ring ([`pitfall/002`](../pitfall/002_claiming_zero.md)) | — |
| nothing at all about `Send`/`Sync` | the entire multi-producer design | [002](002_the_lifetime_on_the_claimer.md) |
| nothing at all about `Seq` vs `usize` | a rule followed in all 15 signatures | [001](001_a_seq_a_usize_and_three_casts.md) |

The first two are prose describing a capability the type forbids. The last two
are the reverse — a real, exceptionless discipline that no prose claims. Both
directions cost a reader the same thing: the signature is the only reliable
source, and it is the one a reader consults last.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the integer rule: positions are Seq, counts are usize
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -E 'pub (const )?fn'

# every cast, and its direction
grep -E ' as (u64|usize|u32|i64)' ring_claim/src/lib.rs

# the reciprocal narrowing, two crates away
command grep -m1 -A4 -F 'pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize' ring_seqno/src/lib.rs

# what the crate imports, against what ring_types exports
grep '^use' ring_claim/src/lib.rs
command grep -rE '^pub (struct|enum)' ring_types/src/*.rs

# the position type this crate never constructs
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -cE 'SlotIndex|%|mask|Capacity'

# the auto-traits nobody mentions
grep -c 'Send\|Sync\|thread' ring_claim/src/lib.rs

# the two accessors, and their differing reach
awk '/^  pub const fn cursor\( &self \) -> &PaddedCursor$/{ print } /^  \/\/\/ assert_eq!\( Claimer::new\( &consumers \)\.consumers\(\)\.len\(\), 2 \);$/{ n2 = NR } n2 && NR == n2 + 3 { print }' ring_claim/src/lib.rs
```

Live output:

```
  pub const fn new( start : Seq, len : usize ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> usize
  pub const fn is_empty( self ) -> bool
  pub const fn contains( self, seq : Seq ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn overlaps( self, other : Self ) -> bool
  pub fn new( consumers : &'a GatingSet ) -> Self
  pub const fn cursor( &self ) -> &PaddedCursor
  pub const fn consumers( &self ) -> &'a GatingSet
  pub fn claimed( &self ) -> Seq
  pub fn headroom( &self ) -> usize
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
    self.start.advanced_by( self.len as u64 )
      let next = current.advanced_by( count as u64 );
      let next = current.advanced_by( granted as u64 );
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
use ring_cursor::{ PaddedCursor, SeqCell, GATING };
use ring_gating::GatingSet;
use ring_types::{ RingError, Seq };
ring_types/src/capacity.rs:pub struct Capacity( usize );
ring_types/src/error.rs:pub enum RingError
ring_types/src/id.rs:pub struct Seq( pub u64 );
ring_types/src/id.rs:pub struct SlotIndex( pub usize );
ring_types/src/policy.rs:pub enum WaitKind
ring_types/src/policy.rs:pub enum OverflowPolicy
0
0
  pub const fn cursor( &self ) -> &PaddedCursor
  pub const fn consumers( &self ) -> &'a GatingSet
```

| | Value |
|--|------:|
| Public signatures following the `Seq`/`usize` rule | **15 of 15** |
| …places the rule is stated | **0** |
| Casts in the crate | **3** |
| …widening `usize` → `u64` | **3** |
| …narrowing | **0** |
| Crates away the reciprocal narrowing lives | 2 (`ring_seqno:98`) |
| …crates its losslessness proof depends on | **1** — local to that line |
| `ring_types` public types | 6 |
| …named by this crate | **2** |
| `SlotIndex`, `%`, `mask`, `Capacity` occurrences in the crate | **0** |
| `ring_types` newtypes with a private field | **1 of 3** — the one with an invariant |
| Accessors returning `&'a` | 1 |
| …returning a borrow that dies with the `Claimer` | 1 |
| …whose docs mention the difference | **0** |
| Occurrences of `Send`, `Sync`, or `thread` in the source | **0** |
| Tests that enforce `Sync` by compiling | **5** |
| …that assert it directly | **0** |
| `Claimer`s constructible over one `GatingSet` | **unbounded** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL47 | family | n/a — observation | All three casts in the crate are `usize` → `u64` widenings serving `Seq::advanced_by`; the reciprocal narrowing at `ring_seqno:98` used to depend on an invariant `ring_claim` maintains and neither crate states, but a refactor moved the narrow to the far side of a `saturating_sub` against a widened `capacity`, so the cast is now lossless by local construction alone |
| CL48 | `ring_claim` | n/a — observation | The crate names 2 of `ring_types`' 6 public types and contains zero occurrences of `SlotIndex`, `%`, `mask`, or `Capacity`: a `Claim` is a range of sequences, never of slots, which is why `overlaps` compares claims from different rings and why the 900-pair test needs no ring at all |
| CL49 | `ring_claim` | n/a — doc gap | `consumers()` returns `&'a GatingSet` and outlives its `Claimer`; `cursor()` returns an elided borrow that cannot; both doc comments are one symmetric sentence and neither mentions it, and the accessor documented for cross-crate use is the one whose lifetime cannot leave the local scope |
| CL50 | `ring_claim` | n/a — unenforced | `Claimer : Sync` is what the entire multi-producer design rests on, holds by auto-derivation, and appears nowhere: zero occurrences of `Send`, `Sync`, or `thread` in a crate whose module doc opens on contention, while four sibling crates write explicit `T : Send` bounds |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| `free_slots`' `saturating_sub` fails toward back-pressure rather than a corrupt grant if that invariant is ever violated — the correct direction, and undocumented | [001](001_a_seq_a_usize_and_three_casts.md) |
| `ring_types` hides the field of the one newtype with an invariant (`Capacity`) and exposes the other two — which is what makes `ring_batch`'s and this crate's `.0` shortcut to two extra `const fn`s each possible, and would make it impossible for `Capacity` | [001](001_a_seq_a_usize_and_three_casts.md) |
| Two `Claimer`s over one `GatingSet` compile and both grant `Seq( 0 )..Seq( 4 )` — the crate's central invariant broken in safe single-threaded code, because the guarantee is per-claimer and the shared `&'a` reference does not say so | [002](002_the_lifetime_on_the_claimer.md) |
| The property is enforced anyway, accidentally, by five `thread::scope` tests that would fail to compile without it — real enforcement whose diagnostic would point at the tests rather than at the field that broke it | [002](002_the_lifetime_on_the_claimer.md) |

