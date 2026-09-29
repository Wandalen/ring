# API: Sixteen Public Items

### Scope

**Purpose:** Take the crate's public surface as a contract and establish what it
promises, what it withholds, and whether its annotations are complete.

**Responsibility:** All 16 public items — their `const`-ness, their `must_use`
annotations, and the three omissions.

**In Scope:** `ring_consume/src/lib.rs` public declarations; the
`#[ must_use ]` attributes and the types that carry their own; the `const fn`
subset.

**Out of Scope:** The two commits' shared contract — that is
[`002`](002_the_two_commits.md). What the types commit to structurally, which is
[`type/001`](../type/001_availables_const_surface.md).

---

## The Surface

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*(pub |#\[ must_use)' ring_consume/src/lib.rs | grep -v '///'
grep -c 'must_use' ring_consume/src/lib.rs
grep -c 'must_use = ' ring_consume/src/lib.rs
grep -c 'pub const fn' ring_consume/src/lib.rs
```

Live output:

```
pub struct Available
  #[ must_use ]
  pub const fn new( start : Seq, len : u64 ) -> Self
  #[ must_use ]
  pub const fn start( self ) -> Seq
  #[ must_use ]
  pub const fn end( self ) -> Seq
  #[ must_use ]
  pub const fn len( self ) -> u64
  #[ must_use ]
  pub const fn is_empty( self ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
pub struct Consumer< 'a >
  #[ must_use ]
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  #[ must_use ]
  pub const fn cursor( &self ) -> &'a PaddedCursor
  #[ must_use ]
  pub const fn barrier( &self ) -> Barrier< 'a >
  #[ must_use ]
  pub fn position( &self ) -> Seq
  #[ must_use ]
  pub fn available( &self ) -> Available
  #[ must_use ]
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
11
0
8
```

Live output for the three counts: `11`, `0`, `8`.

| | `Available` | `Consumer< 'a >` | Total |
|--|:-----------:|:----------------:|:-----:|
| Struct | 1 | 1 | 2 |
| Methods | 6 | 8 | 14 |
| …`const fn` | 5 | 3 | 8 |
| …`#[ must_use ]` | 5 | 6 | 11 |
| …messaged `must_use` | 0 | 0 | **0** |

Sixteen public items, eight of them `const`, eleven annotated `must_use`, none
of those annotations carrying a message.

---

### CN18 — The Three Unannotated Methods Are Each Correctly Unannotated

Eleven of fourteen methods carry `#[ must_use ]`. The three that do not:

| Method | Returns | Why the omission is right |
|--------|---------|---------------------------|
| `Available::sequences` | `impl Iterator< Item = Seq >` | `Iterator` is `#[ must_use ]` at the trait level — std already warns |
| `Consumer::commit` | `Result< Seq, RingError >` | `Result` is `#[ must_use ]` at the type level — std already warns |
| `Consumer::commit_available` | `Seq` | discarding it is legitimate; the commit happened, the return is informational |

So the surface's `must_use` coverage is complete: every method whose result must
not be dropped either carries the attribute or returns a type that carries it,
and the one method whose result is genuinely optional carries neither.

That is worth stating explicitly because a `grep -c 'must_use'` returning 11 out
of 14 looks like three omissions, and two of the three are covered by
annotations that live in `core` rather than in this file. A reviewer counting
attributes would flag them; a reviewer reading types would not.

The third — `commit_available` returning a bare `Seq` — is the one real
decision, and it is right. A caller that commits everything and does not care
how far that reached has done nothing wrong. Contrast `commit`, where the
`Result` must be inspected because a refusal means the cursor did not move and
the caller's model of the ring is now wrong.

**Cost:** none. Recorded because the count invites a false finding.

---

### CN19 — Every `must_use` in the Crate Is Unmessaged, and Here That Is Correct

Zero of the eleven carry a message. In `ring_claim` the equivalent figure was
the subject of a finding, because `Claim`'s `must_use` carries the most severe
message in the family — dropping a `Claim` strands a slot permanently — and that
severity is communicated by a string with no runtime mechanism behind it.

Here there is nothing to say. The strongest consequence of ignoring any of these
eleven returns is that the caller did some arithmetic for nothing:

| Ignored return | Consequence |
|----------------|-------------|
| `Available::start` / `end` / `len` / `is_empty` | none — pure accessors on a `Copy` value |
| `Available::new` | none — constructs a value and drops it |
| `Consumer::new` / `cursor` / `barrier` | none — borrows, no state |
| `Consumer::position` | none — one wasted atomic load |
| `Consumer::available` / `available_up_to` | one wasted load per dependency; one wasted allocation too, until `b7e075ca` |

The last row is the only one with a real cost, and it is a performance cost, not
a correctness one — which is precisely the case a `must_use` message is not for.
It is also a smaller cost than when this was written: what made it worth naming
was the allocation, and the loads are what remain.

So the asymmetry between the two halves of the handshake extends to their
annotations, and it is the *right* asymmetry: `ring_claim` needs a message
because dropping a `Claim` breaks the ring, and `ring_consume` needs none
because dropping an `Available` breaks nothing. The unmessaged eleven here are
not a weaker version of `Claim`'s treatment; they are the correct treatment for
values that carry no obligation.

What neither crate records is that the distinction was drawn deliberately. A
reader comparing the two files sees eleven bare `#[ must_use ]` in one and a
severe message in the other, with nothing indicating whether that reflects a
judgement about obligation or simply two authors' habits.

**Cost:** none. Recorded so the contrast with `ring_claim` reads as a decision
rather than an inconsistency.

---

## Eight `const fn`, and Why the Other Six Are Not

| `const` | Not `const` | Why not |
|---------|-------------|---------|
| `Available::new`, `start`, `end`, `len`, `is_empty` | `Available::sequences` | returns `impl Iterator`; the `map` closure is not `const` |
| `Consumer::new`, `cursor`, `barrier` | `Consumer::position`, `available`, `available_up_to`, `commit`, `commit_available` | all perform an atomic load or store |

The split is exactly the line between values and atomics: everything that
touches a `PaddedCursor` is non-`const`, everything that does not is `const`.
That is the strongest possible reading of the annotation and it is followed
without exception.

`Consumer::new` being `const` is the notable one — a `Consumer` can be built in
a `const` context because it is two borrows and no initialisation
([`data_structure/002`](../data_structure/002_two_borrows_and_no_owned_state.md)).
`ring_claim`'s `Claimer::new` is not `const`, because it owns a `PaddedCursor` it
must initialise.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| api | [002](002_the_two_commits.md) | the contract the two commit forms share |
| type | [001](../type/001_availables_const_surface.md) | what the `const` surface commits to |
| item | [001](../item/001_the_six_of_a_run.md) | `Available`'s six, one at a time |
| item | [002](../item/002_the_eight_of_a_consumer.md) | `Consumer`'s eight, one at a time |
| data_structure | [002](../data_structure/002_two_borrows_and_no_owned_state.md) | why `Consumer::new` can be `const` |

### Sources

| What | Where |
|------|-------|
| `Available`'s six methods | `ring_consume/src/lib.rs:98-185` |
| `Consumer`'s eight methods | `ring_consume/src/lib.rs:216-479` |
| `Iterator`'s trait-level `must_use` | `core::iter::Iterator` |
| `Result`'s type-level `must_use` | `core::result::Result` |

### Tests

| Claim | Verified by |
|-------|-------------|
| 16 public items | the `grep -nE '^\s*pub '` listing above |
| 11 `must_use`, 0 messaged | `grep -c 'must_use'` → 11; `grep -c 'must_use = '` → 0 |
| 8 `pub const fn` | `grep -c 'pub const fn'` → 8 |
| The three omissions are covered or correct | reading each return type against std's own attributes |
