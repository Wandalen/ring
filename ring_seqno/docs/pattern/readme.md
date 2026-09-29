# pattern

Reusable shapes this crate instantiates, stated so the next crate can recognise
them.

### Overview Table

| ID | Name | Shape | Instantiated by |
|----|------|-------|-----------------|
| 001 | [The Predicate Beside Its Quantity](001_the_predicate_beside_its_quantity.md) | Ship `may_x()` alongside `x_count()` rather than making callers write `x_count() != 0` | `may_claim` / `free_slots` |
| 002 | [The Shared Fold That Declines an Identity](002_the_shared_fold_that_declines_an_identity.md) | A fold returns `Option` so consumers that disagree about the empty case can each supply their own | `slowest` |

### Both Are About Where a Decision Belongs

001 keeps a decision *in* the crate: the boundary comparison is written once, in
the cheap form, so no caller re-derives it and no caller pays a division for a
question it asked in the obvious way.

002 pushes a decision *out* of the crate: the identity for an empty fold is
deliberately not chosen here, because the two consumers need opposite ones.

The pair is the crate's whole design philosophy in two shapes — centralise what
is objectively determined, refuse to centralise what is not.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ39 | Predicate and quantity | n/a — observation | The predicate/quantity pair recurs at three tiers of the family, and at the third tier one half of it is unused |
| SQ40 | The sweep's blind side | n/a — coverage | The sweep that pins the predicate to its quantity iterates the producer forward only, so the saturating branch both functions depend on is exercised by three point assertions and never by the sweep |
| SQ41 | Opposite resolutions | n/a — observation | `ring_gating` and `ring_barrier` resolve the same `None` to `capacity` and `0` — the empirical proof the fold was right to decline an identity |
| SQ42 | The module doc's citations | n/a — observation | Twenty-three lines of module doc cite four external documents, one of which exists to correct this crate's own earlier description |

### Regenerate

The predicate/quantity pair at each tier that carries it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'pub fn may_claim\|pub fn free_slots\|pub fn admits\|pub fn available' \
  --include=*.rs ring_*/src/ | sed -E 's/:/: /' | LC_ALL=C sort
```

Live output:

```
ring_barrier/src/lib.rs:   pub fn admits( &self, from : Seq, count : u64 ) -> bool
ring_barrier/src/lib.rs:   pub fn available( &self, from : Seq ) -> u64
ring_consume/src/lib.rs:   pub fn available( &self ) -> Available
ring_consume/src/lib.rs:   pub fn available_up_to( &self, max : u64 ) -> Available
ring_cursor/src/lib.rs:   pub fn free_slots( &self ) -> usize
ring_cursor/src/lib.rs:   pub fn may_claim( &self ) -> bool
ring_gating/src/lib.rs:   pub fn admits( &self, producer : Seq, count : usize ) -> bool
ring_mpsc/src/lib.rs:   pub fn available( &self ) -> usize
ring_seqno/src/lib.rs: pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
ring_seqno/src/lib.rs: pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
ring_spsc/src/lib.rs:   pub fn available( &self ) -> usize
```
