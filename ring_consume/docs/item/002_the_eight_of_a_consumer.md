# Item: The Eight of a Consumer

### Scope

**Purpose:** Take `Consumer`'s eight public items one at a time and record what
each commits to.

**Responsibility:** `Consumer::new`, `cursor`, `barrier`, `position`,
`available`, `available_up_to`, `commit`, `commit_available` — signature, body,
and guarantee.

**In Scope:** `ring_consume/src/lib.rs:216-479`.

**Out of Scope:** `Available`'s six — that is
[`001`](001_the_six_of_a_run.md). The two commits' shared contract, which is
[`api/002`](../api/002_the_two_commits.md).

---

## The Eight

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const )?fn' ring_consume/src/lib.rs | sed -n '7,14p'
```

Live output:

```
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  pub const fn cursor( &self ) -> &'a PaddedCursor
  pub const fn barrier( &self ) -> Barrier< 'a >
  pub fn position( &self ) -> Seq
  pub fn available( &self ) -> Available
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
```

| Item | `const` | `must_use` | Touches an atomic |
|------|:-------:|:----------:|:-----------------:|
| `new( &'a PaddedCursor, Barrier< 'a > ) -> Self` | ✔ | ✔ | ✘ |
| `cursor() -> &'a PaddedCursor` | ✔ | ✔ | ✘ |
| `barrier() -> Barrier< 'a >` | ✔ | ✔ | ✘ |
| `position() -> Seq` | ✘ | ✔ | 1 load |
| `available() -> Available` | ✘ | ✔ | 1 + `n` loads |
| `available_up_to( u64 ) -> Available` | ✘ | ✔ | 1 + `n` loads |
| `commit( Seq ) -> Result< Seq, RingError >` | ✘ | ✘ (`Result`) | 1 + `n` loads, 1 store |
| `commit_available() -> Seq` | ✘ | ✘ | 1 + `n` loads, 1 store |

The `const` boundary and the atomic boundary are the same line, without
exception ([`api/001`](../api/001_sixteen_public_items.md)).

---

### CN28 — `cursor()` Hands Out the Gating Signal, and Says So

```rust
pub const fn cursor( &self ) -> &'a PaddedCursor
{
  self.cursor
}
```

The accessor returns the borrowed cursor with its full `'a` lifetime — not tied
to `&self`, so the reference outlives the `Consumer` that produced it. That is
correct and deliberate: the cursor belongs to the producer's gating set, the
`Consumer` merely borrows it, and handing back a shorter lifetime would be a
lie about ownership.

What a caller can do with it is store it, load it, and — since `PaddedCursor`
exposes `SeqCell` — **store to it.** Nothing in the type prevents a caller from
taking `consumer.cursor()` and writing an arbitrary `Seq`, bypassing both
commits and both halves of the guard
([`invariant/002`](../invariant/002_the_cursor_only_moves_forward.md)).

That is not a defect to fix. The cursor is shared state that the producer reads
directly; a `Consumer` cannot own exclusive write access to it without breaking
the mechanism
([`data_structure/002`](../data_structure/002_two_borrows_and_no_owned_state.md)
CN24). Returning it is necessary for the wiring assertion that
`ring_publish/tests/handshake_test.rs` performs with `ptr::eq`, and
`consume_test.rs`'s `the_consumer_cursor_is_what_the_producer_would_gate_on`
asserts the same identity locally.

The finding is that this is the crate's widest hole and it is documented as an
accessor. `cursor()`'s doc comment describes what it returns. It does not say
that the returned reference permits stores, that a store through it skips every
guarantee `commit` provides, or that the only reason to call it is to assert
wiring identity.

Compare `ring_claim` CL30, where `cursor()`'s documentation actively points
readers at a conflation a sibling crate forbids. Here the documentation is not
wrong — it is silent about the one thing a reader needs to know before using
the return value for anything other than a pointer comparison.

**Cost:** reachable. The accessor must exist and must return what it returns;
what is missing is the sentence saying that writing through it is possible and
always wrong.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '  /// The returned reference is a full [`SeqCell`], so it also permits a' ring_consume/src/lib.rs
```

Live output:

```
  /// The returned reference is a full [`SeqCell`], so it also permits a
  /// direct `store` — which bypasses `commit`'s guard entirely, along with
  /// every guarantee this type provides. The only sound reason to call this
  /// accessor is to assert wiring identity, as the doctest below does with
```

**Disposition:** applied — `cursor()`'s doc comment now says the sentence
this finding names: the returned reference permits a direct store that
bypasses `commit`'s guard and every guarantee the type provides, and the only
sound reason to call the accessor is to assert wiring identity. The crate's
22 tests (1 `allocation_test.rs` + 21 `consume_test.rs`) plus 17 doctests
re-verified passing (`cargo test --all-features`, 2026-09-04). Now prints:
`The returned reference is a full`

---

### CN29 — `barrier()` Returns by Value Because `Barrier` Is `Copy`, and That Is Load-Bearing

```rust
pub const fn barrier( &self ) -> Barrier< 'a >
{
  self.barrier
}
```

Returned by value, not by reference. That works because `Barrier< 'a >` is
`Copy` — it is a slice reference and a length, sixteen bytes
([`data_structure/001`](../data_structure/001_sixteen_and_twenty_four.md)) — and
it matters for a reason the signature does not show.

`ring_barrier`'s module documentation records that the `Barrier` signature was
changed specifically to make the four-operation handshake wireable:

> That is not a hypothetical: it is what made the four-operation handshake in
> `ring_publish/tests/handshake_test.rs` unwireable until this signature changed.

A `Barrier` that borrowed *into* the set rather than being a copyable view over
it would force `Consumer::barrier()` to return `&Barrier`, tied to `&self`, and
a caller could not hold both a `Consumer` and its barrier independently. The
handshake test needs exactly that.

So three signatures agree — `Barrier::over` taking `&'a [ PaddedCursor ]`,
`Consumer`'s field holding a `Barrier< 'a >` by value, and `barrier()` returning
one by value — and the agreement was forced by a test in a fourth crate. That
history is recorded once, in `ring_barrier`, and is invisible from here.

`the_consumer_exposes_the_barrier_it_was_built_over` at `consume_test.rs:321`
asserts the round-trip locally, which is the right test and does not capture why
the shape is what it is.

**Cost:** none. Recorded because the `Copy`-by-value shape looks like a
micro-optimisation and is actually a constraint imposed by a downstream test.

---

## The Other Six

| Item | Notes |
|------|-------|
| `new` | `const`; does **not** reset the cursor — `§ N2` records that this is what makes it safe to build a `Consumer` around a position the producer already gates on |
| `position` | the crate's only `load`; `GATING`, i.e. `Acquire` |
| `available` | three steps, and until `b7e075ca` an allocation — [`algorithm/001`](../algorithm/001_position_frontier_pending.md) |
| `available_up_to` | `available()` then `min`; no cheaper path |
| `commit` | the two-sided guard — [`algorithm/002`](../algorithm/002_the_two_sided_guard.md) |
| `commit_available` | not `commit`'s guard, but an emptiness guard of its own — [`pitfall/001`](../pitfall/001_commit_available_does_not_call_commit.md) |

`new` not resetting the cursor is the subtlest of the six and the only one whose
rationale lives in the manual plan rather than the source. A `Consumer::new`
that zeroed the cursor would be catastrophic — it would tell every producer that
the consumer had read nothing, freeing the whole ring — and the reason it does
not is simply that the constructor stores two borrows and touches nothing.
`§ N2`'s expected output includes "nothing in `available`, `available_up_to`,
`position` or `new`", which is where that guarantee is actually checked.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| item | [001](001_the_six_of_a_run.md) | `Available`'s six |
| api | [001](../api/001_sixteen_public_items.md) | the eight as part of the 16-item surface |
| api | [002](../api/002_the_two_commits.md) | the last two, as a contract |
| data_structure | [002](../data_structure/002_two_borrows_and_no_owned_state.md) | why `cursor()` must return what it returns |
| invariant | [002](../invariant/002_the_cursor_only_moves_forward.md) | the guarantee a store through `cursor()` bypasses |

### Sources

| What | Where |
|------|-------|
| The eight | `ring_consume/src/lib.rs:216-479` |
| `cursor()` | `ring_consume/src/lib.rs:270-273` |
| `barrier()` | `ring_consume/src/lib.rs:288-291` |
| `Barrier`'s signature history | `ring_barrier/src/lib.rs`, module doc |
| `new` not resetting the cursor | `ring_consume/tests/manual/readme.md § N2` |

### Tests

| Claim | Verified by |
|-------|-------------|
| `cursor()` returns the gating cursor | `consume_test.rs`, `the_consumer_cursor_is_what_the_producer_would_gate_on` |
| `barrier()` round-trips | `consume_test.rs:314` |
| `new` touches no atomic | `§ N2`'s grep — no store or load in `new` |
| The `const`/atomic boundary is exact | the table above, read against the eight bodies |
