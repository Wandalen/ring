# Algorithm: One Mask, No Modulo

### Scope

**Purpose:** Record that the whole addressing algorithm is a delegated bitmask,
that this crate refuses to reimplement it and says why, that the two `at`
functions differ in body for a reason the borrow checker imposes, and that the
crate delegating to `ring_index` uses one of the three functions `ring_index`
publishes.

**Responsibility:** How a sequence becomes a slot — the arithmetic, where it
lives, and who calls it.

**In Scope:** `ring_store/src/lib.rs:245-257`;
`ring_index/src/lib.rs:1-17, 48-52`; the family-wide `ring_index` census.

**Out of Scope:** Construction is
[`algorithm/002`](002_one_allocation_n_defaults.md). Whether an index that did
*not* come from the fold is in range is
[`decisions/001`](../decisions/001_panic_rather_than_option.md).

---

## The Whole Fold

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -A3 -F 'pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex' ring_index/src/lib.rs
```

Live output:

```
#[ must_use ]
pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
{
  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
}
```

One `as`, one `&`. No branch, no division, no error path — and the reason it can
be total is stated one crate away:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A8 -F '//! the [`ring_types::SlotIndex`] it addresses. The feature'"'"'s whole reason for' ring_index/src/lib.rs
```

Live output:

```
//! the [`ring_types::SlotIndex`] it addresses. The feature's whole reason for
//! constraining capacity to a power of two is that this fold is then a bitmask
//! rather than a division — an integer `%` costs on the order of 20–40 cycles on
//! current x86, and it sits on every operation that touches a slot. The claim
//! path is not one of them: `ring_claim`, `ring_publish`, `ring_consume`, and
//! `ring_cursor` work in sequence space end to end and never fold.
//!
//! The "20–40 cycles" figure above is asserted for x86, not measured in this
//! repository: on this crate's own build host the fold costs roughly 0.9
```

---

### BF10 — The Crate Refuses to Own the Arithmetic, and the Two Bodies Still Differ

`at` is the fold composed with `get`, and the doc explains the composition as a
deliberate non-duplication:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^  \/\/\/ Borrow the slot a sequence addresses, folding through `ring_index`\.$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 4 { print } /^  \/\/\/ assert_eq!\( buffer\.get\( SlotIndex\( 2 \) \)\.get\(\), Some\( &1 \) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 13 { print }' ring_store/src/lib.rs
```

Live output:

```
  /// Borrow the slot a sequence addresses, folding through `ring_index`.
  ///
  /// The convenience that keeps the fold in one place: a caller that wrote its
  /// own `seq % capacity` here would be the second implementation of the thing
  /// `ring_index` exists to be the only one of.
  #[ must_use ]
  pub fn at( &self, seq : Seq ) -> &S
  {
    self.get( of( seq, self.capacity ) )
  }

  /// Mutably borrow the slot a sequence addresses.
  #[ must_use ]
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
  {
    let index = of( seq, self.capacity );
    self.get_mut( index )
```

Two bodies for one composition. The obvious reading of the two-line body is that the borrow checker forced it —
`self` is mutably borrowed by `get_mut` while `self.capacity` still needs
reading. Compiled, it is not so: two-phase borrows accept the one-line form.

```
  the one-line form compiles: yes
```

**Finding.** The pair is one algorithm written twice, and the second writing is
two lines instead of one for no reason the compiler requires and none the source
states. `at` composes; `at_mut` binds a temporary and then composes. Both spell
the same thing.

The consequence is small and worth naming because the *explanation* is the trap
rather than the code: a reader who assumes the split is borrow-checker-imposed
concludes the API has a constraint it does not have, and a maintainer who tidies
`at_mut` into one line expects a fight and gets none. Nothing breaks either way.
Making the two bodies match would remove the question; leaving them and saying
why would too.

The delegation itself is exactly right and unusually well argued. `at` exists so
that `seq % capacity` is never written twice in the family, and the doc names
that as the point rather than leaving it to be inferred.

---

### BF11 — Two Crates Use `ring_index` and Both Import Only `of`

`ring_index` publishes three functions:

```sh
cd "$(git rev-parse --show-toplevel)"
grep '^pub fn ' ring_index/src/lib.rs
```

Live output:

```
pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
pub fn aliases( a : Seq, b : Seq, capacity : Capacity ) -> bool
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
```

The family's executable code reaches for it exactly twice:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r 'ring_index' ring_*/src/*.rs | grep -v '^ring_index/' \
  | grep -vE ':[[:space:]]*(///|//!|//)' | sed 's|^ring/||' | sort
```

Live output:

```
ring_batch/src/lib.rs:use ring_index::of;
ring_store/src/lib.rs:use ring_index::of;
```

Two crates, one imported name each, the same name both times. `aliases` and `run`
have no caller anywhere in the family outside `ring_index`'s own tests and
doctests.

**Finding.** A Tier 1 crate ships three functions and the family consumes one.
That is a finding about `ring_index` rather than about this one, and it is
recorded from here because this is the crate that would use the other two if
anything did: `aliases( a, b, capacity )` answers exactly the question
`invariant/002`'s non-aliasing property is about, and `run( start, count,
capacity )` produces exactly the index sequence a batch drain over a `Buffer`
would walk.

Neither is called, and the reason is visible in the shapes. `aliases` is
`of( a ) == of( b )`, which a caller holding both sequences can write inline and
which the buffer never needs — it addresses one slot at a time. `run` returns a
`Vec< SlotIndex >`, so a batch consumer that used it would allocate once per
batch on a path whose entire premise is that allocation happened at construction
([`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md)).
The buffer offers no batch accessor to pair it with, so the two crates that walk
a range of sequences fold each one individually instead.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_one_allocation_n_defaults.md) | The other algorithm in the crate — construction |
| [`decisions/001`](../decisions/001_panic_rather_than_option.md) | What happens to an index the fold did not produce |
| [`invariant/002`](../invariant/002_two_distinct_indices_never_alias.md) | The property the fold guarantees, and `aliases` would have answered |
| [`api/002`](../api/002_six_ways_to_reach_a_slot.md) | Where `at`/`at_mut` sit among the six accessors |
| [`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md) | Why a `Vec`-returning helper has no place on this path |

### Sources

| Fact | Where |
|------|-------|
| The fold | `ring_index/src/lib.rs:48-52` |
| Why it is total, and what a `%` would cost | `ring_index/src/lib.rs:10-17` |
| `at` and `at_mut` | `ring_store/src/lib.rs:245-257` |
| The one-line form compiling | Release probe over a struct mimicking `Buffer`'s fields |
| The three published functions | `ring_index/src/lib.rs:49, 74, 119` |
| Two importers, one imported name | `ring_batch/src/lib.rs:36`, `ring_store/src/lib.rs:26` |

### Tests

| Test | Covers |
|------|--------|
| `a_sequence_addresses_the_slot_ring_index_says_it_does` | `at_mut` against `of` directly, forty sequences over capacity 8 |
| `a_full_lap_overwrites_and_a_partial_one_does_not` | The lap boundary the mask produces |
| `storage_survives_being_addressed_out_of_order` | Sixteen scrambled sequences, each landing where the fold says |
| `ring_index` — `index_test.rs` | The fold itself, including `aliases` and `run`, which only it exercises |
| *(to create)* | A doctest showing `at` and `at_mut` agree, so the two bodies stay one algorithm |
