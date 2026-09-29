# Invariant: Two Ranges That Cannot Be Violated

### Scope

**Purpose:** Record the two numeric ranges every `RingConfig` satisfies, where
each is established, where each is preserved, and what each rests on.

**Responsibility:** `producers >= 1` and `1 <= batch <= capacity` — their
statement, their enforcement, and the cross-crate guarantee the second one
borrows.

**In Scope:** `ring_config/src/lib.rs:74-75`, `:124`, `:142-143`, `:185`,
`:197`; `ring_types/src/capacity.rs:42-45`.

**Out of Scope:** Why the setters may be applied in any order without breaking
either range is
[`invariant/002`](002_the_setters_commute_and_one_absence_is_why.md). Why
correcting was chosen over rejecting is
[`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md).

---

## Both Ranges, End to End

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two ranges, as the getters state them --'
sed -n '/^  \/\/\/ The expected producer count, always at least one\.$/p;/^  \/\/\/ The batch size, always between one and the capacity inclusive\.$/p' ring_config/src/lib.rs
echo '  -- where each range starts: the two literal fields new writes --'
command grep -m1 -A1 -F '        producers : 1,' ring_config/src/lib.rs
echo '  -- and where each is preserved --'
sed -n '/^    self\.producers = if producers == 0 { 1 } else { producers };$/p;/^    let capped = if batch > self\.capacity\.get() { self\.capacity\.get() } else { batch };$/,/^    self\.batch = if capped == 0 { 1 } else { capped };$/p' ring_config/src/lib.rs
echo '  -- the lower bound the batch clamp leans on, enforced in another crate --'
command grep -A 3 'slots == 0' ring_types/src/capacity.rs
echo '  -- the tests that walk both ends of each range --'
command grep 'fn zero_producers_clamps_to_one\|fn batch_clamps_into_one_through_capacity' ring_config/tests/config_test.rs
```

Live output:

```
  -- the two ranges, as the getters state them --
  /// The expected producer count, always at least one.
  /// The batch size, always between one and the capacity inclusive.
  -- where each range starts: the two literal fields new writes --
        producers : 1,
        batch : 1,
  -- and where each is preserved --
    self.producers = if producers == 0 { 1 } else { producers };
    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
    self.batch = if capped == 0 { 1 } else { capped };
  -- the lower bound the batch clamp leans on, enforced in another crate --
    if slots == 0
    {
      return Err( RingError::CapacityZero );
    }
  -- the tests that walk both ends of each range --
fn zero_producers_clamps_to_one()
fn batch_clamps_into_one_through_capacity()
```

---

### RC21 — Both Invariants Are Documented on the Reader, Not on the Thing That Must Preserve Them

Two ranges hold for every `RingConfig` that exists. `producers` is at least one:
`new` writes the literal `1`, and `with_producers` raises a zero back to `1`.
`batch` is between one and the capacity inclusive: `new` writes the literal `1`,
and `with_batch` caps at `self.capacity.get()` and then raises a zero.

Both are stated, and both are stated in the same place — on the getter. `:185`
reads "The expected producer count, always at least one." `:197` reads "The batch
size, always between one and the capacity inclusive." Those are the only two
sentences in the crate that describe the ranges as properties rather than as the
outcome of one particular call.

**Finding.** The statements are accurate and they are attached to the wrong end.
A getter's doc is read by a caller deciding whether to trust a value; an
invariant's statement is needed by whoever is about to add a sixth field or a
fifth setter, and that reader is looking at `:42-49` and `:110-144`, where nothing
says what must survive their change.

The declaration at `:41-49` carries no comment about either range. `with_batch`'s
doc explains why it clamps — "a batch larger than the ring can never be served" —
which is a justification for one call, not a statement that the resulting bound
holds for the type. Nothing anywhere says the two ranges are the crate's contract,
so nothing tells a maintainer that they are what a new setter would have to
preserve.

The gap is cheap to close and currently costs nothing, because the type has four
setters and all four were written together. It is recorded because the ranges are
exactly the kind of property that survives by accident until it does not.

---

### RC22 — The Batch Range Is Enforced in Two Crates and Neither Says So

`with_batch` clamps in two steps, upper bound first:

```rust
let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
self.batch = if capped == 0 { 1 } else { capped };
```

The second step raises a zero to one. It is correct only because `capped` can be
zero solely when the caller passed zero — which requires `self.capacity.get()` to
be at least one. Were a capacity of zero reachable, `with_batch( 5 )` would cap to
`0`, raise to `1`, and store a batch strictly greater than the capacity, breaking
the range `:197` promises in the one place the clamp exists to protect.

That capacity is never zero is enforced in a different crate. `Capacity::new`
returns `RingError::CapacityZero` at `ring_types/src/capacity.rs:44`, and the
tuple field is private, so `Capacity::new` is the only constructor reachable from
here.

**Finding.** So one of this crate's two invariants is not self-contained: its
lower bound is preserved by a clamp whose correctness rests on a guarantee made
and tested in `ring_types`, and neither crate records the connection.

`ring_types` comes closest and stops one beneficiary short. Its type doc says:
"The only constructor rejects zero and non-powers of two, so `mask()` below is
total: every `Capacity` that exists has a usable mask." It names exactly one thing
the non-zero guarantee makes total. `with_batch`'s zero-raise is a second, and it
is in another crate, and it is unnamed.

Nothing is broken. `Capacity` cannot be zero, the clamp is right, and the tests
pass. What is missing is that a change to `Capacity` — admitting zero as a
degenerate ring, say, or adding an unchecked constructor for a hot path — would
break `ring_config`'s documented batch range through a mechanism no comment in
either crate connects, and the two ends are a crate boundary apart.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](002_the_setters_commute_and_one_absence_is_why.md) | Why applying the setters in any order preserves both ranges |
| [`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md) | Why the ranges are enforced by correcting rather than rejecting |
| [`pitfall/001`](../pitfall/001_a_clamp_with_no_way_to_detect_it.md) | What a caller cannot learn when a clamp preserved a range on their behalf |
| [`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md) | That nothing in production reads either of the two bounded fields |

### Sources

| Fact | Where |
|------|-------|
| Both ranges stated, on the getters | `ring_config/src/lib.rs:185`, `:197` |
| Both ranges established | `ring_config/src/lib.rs:74-75` |
| The producer clamp | `ring_config/src/lib.rs:124` |
| The batch clamp, upper bound then lower | `ring_config/src/lib.rs:142-143` |
| The declaration, carrying no statement of either | `ring_config/src/lib.rs:41-49` |
| Zero capacity rejected, in another crate | `ring_types/src/capacity.rs:42-45` |
| `Capacity::new` as the only reachable constructor | `ring_types/src/capacity.rs:23`, `:40` |
| `ring_types` naming `mask()` as the guarantee's one beneficiary | `ring_types/src/capacity.rs:12-14` |

### Tests

| Test | Covers |
|------|--------|
| `zero_producers_clamps_to_one` | The lower bound of the producer range |
| `batch_clamps_into_one_through_capacity` | Both ends of the batch range, including that the cap follows the capacity |
| `defaults_are_the_documented_ones` | That a fresh record already satisfies both |
