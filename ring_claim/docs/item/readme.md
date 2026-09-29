# item

Fifteen methods across two types, taken one at a time. The census — how many
there are, how many can drop their result silently, what the surface costs — is
[`api/`](../api/readme.md)'s job. This definition is the per-item reading: what
each one costs, what each one guarantees, and where the shape of each was
actually decided.

The two files split by type, and the split is sharper than it sounds, because
the two types agree on almost nothing about how a method should be written.
`Claim`'s eight take `self` by value; `Claimer`'s seven take `&self`. `Claim`'s
are mostly `const`; `Claimer`'s mostly cannot be. `Claim`'s never touch an
atomic; four of `Claimer`'s do. Neither type has a single `&mut self` method,
which is the one thing they do share and the reason the crate works at all.

What the per-item reading turns up, in both files, is that the interesting
decisions were made somewhere else. `Claim`'s `const` boundary was fixed by
`ring_types` choosing derived ordering. `Claimer::cursor()`'s documented purpose
was fixed by an integration that never happened. Reading these fifteen items in
isolation is what makes those visible; reading them as a surface does not.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Eight Readings of a Range](001_the_eight_readings_of_a_range.md) | CL27, CL28 — why exactly seven of eight are `const` and the eighth cannot be, and the guard whose own doctest does not exercise it |
| 002 | [The Seven of the Claimer](002_the_seven_of_the_claimer.md) | CL29, CL30 — the free/reading/writing cost tiers, and the accessor documented for two purposes and used for a third |

### The Two Types, Item for Item

| | `Claim` (8) | `Claimer` (7) |
|--|-------------|---------------|
| Receiver | `self` by value, all 8 | `&self`, all 7 |
| `&mut self` | 0 | **0** |
| `const fn` | 7 | 2 |
| Touch an atomic | **0** | 4 |
| Can fail | 0 | 2 |
| Explicit `#[ must_use ]` | 6 of 8 | 5 of 7 |
| …unannotated because the return type carries one | 2 | 2 |
| External callers | — | 5 of 7 items, 1 each, all `ring_mpsc` |

The `&mut self` row is the load-bearing one. A `Claimer` mutates a cursor
through `&self`, which is what lets several producer threads hold one
simultaneously; the moment any method took `&mut self`, the type would need a
lock above it and `WaitKind::None` would become unimplementable
([`decisions/001`](../decisions/001_compare_exchange_rather_than_fetch_add.md)).
The absence is the design.

### Where Each Item's Shape Was Actually Decided

Both files arrive at the same structural observation from different directions,
and it is the reason this definition earns its place next to `api/`:

| Item | Looks like a choice made here | Actually fixed by |
|------|-------------------------------|-------------------|
| `contains`, `overlaps` not `const` | a constraint from `ring_types` | a choice made here — `ring_batch` compares `.0` and gets both `const` |
| `new`, `sequences` unannotated | inconsistency with the other six | their return types already carrying a **better** `must_use` |
| `claim`, `claim_up_to` unannotated | same | `Result`'s own, plus `Claim`'s messaged one inside it |
| `cursor()`'s documented purpose | this crate's integration plan | an integration `ring_publish` explicitly rejects |
| `headroom()`'s per-consumer cost | a local loop | `GatingSet::slowest`, which reads every registered cursor |

Four of the five point outward. A reader trying to understand why an item looks
the way it does will, more often than not, need to leave the crate.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the fifteen, with their attributes, comment lines stripped
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -E 'must_use|pub (const )?fn'

# receivers: nothing takes &mut self
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -cE 'fn [a-z_]+\( *&mut self'

# what makes Claim's const boundary fall where it does
grep 'derive\|pub const fn' ring_types/src/id.rs | head

# every external call into the Claimer
for m in claim claim_up_to claimed headroom cursor consumers; do
  printf '%-12s %s\n' "$m" "$( grep -rn "claimer\.$m(" */src/*.rs \
    | grep -cv '^ring_claim/' )"
done
```

Live output:

```
#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
  pub const fn new( start : Seq, len : usize ) -> Self
  #[ must_use ]
  pub const fn start( self ) -> Seq
  #[ must_use ]
  pub const fn end( self ) -> Seq
  #[ must_use ]
  pub const fn len( self ) -> usize
  #[ must_use ]
  pub const fn is_empty( self ) -> bool
  #[ must_use ]
  pub const fn contains( self, seq : Seq ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  #[ must_use ]
  pub const fn overlaps( self, other : Self ) -> bool
  #[ must_use ]
  pub fn new( consumers : &'a GatingSet ) -> Self
  #[ must_use ]
  pub const fn cursor( &self ) -> &PaddedCursor
  #[ must_use ]
  pub const fn consumers( &self ) -> &'a GatingSet
  #[ must_use ]
  pub fn claimed( &self ) -> Seq
  #[ must_use ]
  pub fn headroom( &self ) -> usize
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
0
//! publications for the lifetime of a ring, and the slot index derived from it.
/// What *does* wrap is the [`SlotIndex`] derived from it, which is a different
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
  pub const fn next( self ) -> Self
  pub const fn advanced_by( self, n : u64 ) -> Self
  pub const fn distance_to( self, later : Self ) -> u64
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
  pub const fn get( self ) -> usize
claim        1
claim_up_to  0
claimed      1
headroom     1
cursor       1
consumers    0
```

| | Value |
|--|------:|
| Public methods | 15 |
| …on `Claim` | 8 |
| …on `Claimer` | 7 |
| Taking `&mut self` | **0** |
| `const fn` | 9 |
| Explicit `#[ must_use ]` attributes | 12 |
| …messaged | 1 |
| Methods with no explicit attribute | 4 |
| …whose return type carries one anyway | **4** |
| Methods that can return `Err` | 2 |
| Methods that issue an atomic instruction | 4 |
| `Claimer` items with an external caller | **5 of 7** |
| …with more than one | **0** |
| 900-pair divergences if `overlaps` loses its empty guards | **52** |
| …doctest assertions that would still pass | **3 of 3** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL27 | `ring_claim` | **measured cost** | `Claim`'s `const` boundary fell where it did because `contains` and `overlaps` compared `Seq` values through `PartialOrd`, which `const fn` cannot call; `ring_batch::BatchClaim` implements the identical predicates against `.0` and is `const` on both, reaching 7 of 8 where this crate reached 5 of 8 — this crate has since taken the same `.0` escape, so both now reach 7 of 8 |
| CL28 | `ring_claim` | n/a — coverage | `overlaps`'s two `is_empty()` guards are load-bearing, and its doctest picks the one empty case the arithmetic already handles: deleting both guards leaves all three doctest assertions passing while diverging from the first-principles definition on **52 of 900** pairs in the exhaustive test |
| CL29 | `ring_claim` | **misleading doc** | The seven sort into three undocumented cost tiers (three free, two reading, two writing), and `headroom` is the one that misleads: its own doc calls it "a hint only", and a caller who reads it and then claims that many has rebuilt the rejected check-then-act shape outside the crate |
| CL30 | `ring_claim` | n/a — drift | `cursor()` is documented "for `ring_publish` to read and for a gating set … to be built against"; `ring_publish` owns its own `PaddedCursor` and its module doc opens by forbidding that conflation, the `ring_mpsc` dependency runs the other way, and the single real caller uses `addr()` for a cache-line check |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| The trade is real in both directions — reaching through `.0` bypasses the newtype `ring_types` exists to enforce — but neither crate mentions the other or says why it compares the way it does, so a reader of either concludes the shape was forced | [001](001_the_eight_readings_of_a_range.md) |
| Changing one literal in that doctest — `Seq( 0 )` to `Seq( 2 )` on the empty-claim line — converts an assertion that documents the guard into one that also tests it | [001](001_the_eight_readings_of_a_range.md) |
| `headroom`'s cost scales with consumer count — it forwards to `GatingSet::slowest`, which reads every registered cursor — which nothing in its documentation states | [002](002_the_seven_of_the_claimer.md) |
| `claimed()` pins the read ordering to `GATING` (`Acquire`); `cursor()` hands out a cell any caller may `load` with `Relaxed` — which is the better reason to prefer the former, and is not stated | [002](002_the_seven_of_the_claimer.md) |
| `claim_up_to` has zero callers family-wide, and its only appearance outside this crate is a comment in `ring_publish/tests/publish_test.rs:106` describing what it does | [002](002_the_seven_of_the_claimer.md) |

