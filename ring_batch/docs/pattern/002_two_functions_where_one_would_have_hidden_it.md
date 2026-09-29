# Pattern: Two Functions Where One Would Have Hidden It

### Scope

**Purpose:** Record the crate's second structural decision — gated and ungated
claiming as two separate public functions rather than one with an optional
barrier — the reason it gives, and the shape the same decision takes one tier up.

**Responsibility:** `claim` against `claim_gated` as an API split;
`ring_batch::claim_gated` as a free function against `ring_claim::Claimer` as a
struct.

**In Scope:** `ring_batch/src/lib.rs:198-202`, `:221-224`, `:306-314`;
`ring_claim/src/lib.rs:257-261`, `:421`.

**Out of Scope:** That the gate does not close the race is
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md).
The two independent type parameters are
[`type/001`](../type/001_the_ring_that_can_gate_against_itself.md). The
two-error split inside the gated form is
[`decisions/002`](../decisions/002_two_errors_not_one.md).

---

## The Split, and the Reason Given for It

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the reason, on the cheap half --'
command grep -m1 -A4 -F '/// Performs **no gating**. A claim taken without consulting a consumer barrier' ring_batch/src/lib.rs
echo '  -- the cheap half --'
command grep -m1 -A3 -F 'pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim' ring_batch/src/lib.rs
echo '  -- and the expensive one --'
command grep -m1 -A8 -F 'pub fn claim_gated< P : SeqCell, C : SeqCell >' ring_batch/src/lib.rs
```

Live output:

```
  -- the reason, on the cheap half --
/// Performs **no gating**. A claim taken without consulting a consumer barrier
/// can outrun the ring; [`claim_gated`] checks first — exactly for a single
/// producer, advisory under several (see its own `# One producer only`
/// section below). Both exist because the SPSC path knows its own consumer
/// and the MPSC path does not, and forcing the cheap case through the gated
  -- the cheap half --
pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
{
  BatchClaim::new( cursor.fetch_add( count as u64, order ), count )
}
  -- and the expensive one --
pub fn claim_gated< P : SeqCell, C : SeqCell >
(
  producer : &P,
  consumer : &C,
  count : usize,
  capacity : Capacity,
  order : Ordering,
)
-> Result< BatchClaim, RingError >
```

The split itself is the right call. One function with an `Option< &C >` barrier
would put a branch on the hot path, give the ungated case a `Result` it can never
return `Err` from, and hide in a parameter the single most important thing a
reader needs to know about which of the two they are calling.

---

### BA40 — The Split Was Made for Two Callers and Got One

The reason names two consumers by name. Neither exists:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two paths the reason names --'
for c in ring_spsc ring_mpsc
do
  printf '    %-10s depends on ring_batch : %s\n' "$c" "$( command grep -c 'ring_batch' ring/$c/Cargo.toml || true )"
done
echo '  -- who actually depends on it --'
command grep -rl 'ring_batch' --include=Cargo.toml . | command grep -v '^ring/Cargo.toml$' | sed 's|ring/||;s|/Cargo.toml||' | sort
echo '  -- and what that one imports --'
command grep 'use ring_batch' ring_tls/src/lib.rs
echo '  -- call sites in this crate own tests --'
printf '    claim(       : %s\n' "$( command grep -c 'claim(' ring_batch/tests/batch_test.rs || true )"
printf '    claim_gated( : %s\n' "$( command grep -c 'claim_gated(' ring_batch/tests/batch_test.rs || true )"
```

Live output:

```
  -- the two paths the reason names --
    ring_spsc  depends on ring_batch : 0
    ring_mpsc  depends on ring_batch : 0
  -- who actually depends on it --
ring_batch
ring_tls
  -- and what that one imports --
use ring_batch::{ claim, BatchClaim };
  -- call sites in this crate own tests --
    claim(       : 5
    claim_gated( : 11
```

**Finding.** `ring_spsc` and `ring_mpsc` are the two paths the doc gives as the
reason for the split, and neither has `ring_batch` in its manifest — both wrote
their own claiming instead
([`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md) BA31). The
single crate that does depend on `ring_batch` is `ring_tls`, and its import line
is `use ring_batch::{ claim, BatchClaim }`: the cheap half and the type, not the
gated half.

So half the prediction landed. The cheap case is real, has a user, and that user
would indeed have paid for a barrier read it does not need. The gated case has
eleven call sites, all of them inside this crate's own test file, and no caller
anywhere in the family — which is why the race
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md)
measures in it has never bitten anyone.

That is not an argument against having written it. It is the reason the doc's
justification should name a mechanism rather than two crates: the crates moved,
the mechanism did not.

---

### BA41 — The Same Gate, One Tier Up, Is a Struct

`ring_claim` builds the identical gate and does not spell it as a function:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the gate as a struct, three tiers up --'
command grep -m1 -A5 -F 'pub struct Claimer< '"'"'a >' ring_claim/src/lib.rs
echo '  -- and its claim --'
command grep -m1 -F '  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >' ring_claim/src/lib.rs
```

Live output:

```
  -- the gate as a struct, three tiers up --
pub struct Claimer< 'a >
{
  cursor : PaddedCursor,
  consumers : &'a GatingSet,
}

  -- and its claim --
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
```

**Finding.** `Claimer` owns its producer cursor by value and borrows the
consumers, so `Claimer::claim` takes one argument where `claim_gated` takes five
— capacity comes from the gating set and the ordering is fixed inside. The two
functions do the same work and disagree about where the work's context lives.

The difference is not only ergonomic. `claim_gated`'s producer and consumer
arrive as two independent references with two independent type parameters, which
is what makes `claim_gated( &cell, &cell, count, capacity, order )` type-check
and grant every request — `free_slots( at, at, capacity )` is always the full
capacity, so a ring gated against itself is never full
([`type/001`](../type/001_the_ring_that_can_gate_against_itself.md)). `Claimer`
cannot be misused that way: owning one side and borrowing the other makes the
two-cells-are-one-cell mistake unrepresentable.

Both spellings are defensible at their tiers — a Tier 2 crate with no storage
dependency has nothing to own, and a free function over `SeqCell` is exactly what
that constraint produces. What is missing is the sentence saying so, in either
crate. `ring_claim` does not record that it upgraded the free function to a
struct, or why; `ring_batch` does not record that its form trades a safety
property for the freedom to work without owning anything.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_the_range_object.md) | The other shape these two crates share |
| [`type/001`](../type/001_the_ring_that_can_gate_against_itself.md) | The signature this split makes possible |
| [`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md) | What the gated half does not actually prevent |
| [`decisions/001`](../decisions/001_ordering_is_the_callers_except_where_it_is_not.md) | The other decision documented in these two doc comments |

### Sources

| Fact | Where |
|------|-------|
| The reason for the split | `ring_batch/src/lib.rs:198-202` |
| The two signatures | `ring_batch/src/lib.rs:221-224`, `:306-314` |
| Who depends on the crate, and what they import | Census above; `ring_tls/src/lib.rs:44` |
| The struct form | `ring_claim/src/lib.rs:257-261`, `:421` |

### Tests

| Test | Covers |
|------|--------|
| `a_full_ring_refuses_with_full_and_advances_nothing` | The gated half's back-pressure path |
| `an_oversized_request_is_refused_before_the_ring_is_even_consulted` | Its configuration path |
| `a_claim_of_sixty_four_issues_one_operation_not_sixty_four` | The ungated half the one real caller uses |
| *(to create)* | Nothing asserts the two halves agree when the ring has room |
