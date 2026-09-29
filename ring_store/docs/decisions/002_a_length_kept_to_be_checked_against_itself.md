# Decisions: A Length Kept to Be Checked Against Itself

### Scope

**Purpose:** Record that `capacity` is stored as a field although `slots.len()`
always equals it, that `len` exists as a second reading specifically so a test
can compare the two, and that `is_empty` exists only because `len` does and is
documented as always false.

**Responsibility:** The three size-reporting functions and the field behind them
— what each is for, and what keeping all three costs.

**In Scope:** `ring_store/src/lib.rs:56-63, 146-191`; the measured
`size_of`.

**Out of Scope:** That `is_empty` sits three characters from `all_empty` and
means the opposite is [`pitfall/001`](../pitfall/001_the_two_questions_named_is_empty.md).
The struct's layout is [`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md).

---

## Two Fields, One Number

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '#[ derive( Debug ) ]' ring_store/src/lib.rs
```

Live output:

```
#[ derive( Debug ) ]
pub struct Buffer< S >
{
  slots : Box< [ S ] >,
  capacity : Capacity,
}
```

A `Box< [ S ] >` already carries its length. `Capacity` is a validated
power-of-two `usize`, and `new` sizes the slice from exactly that number, so the
two can never disagree.

---

### BF8 — The Redundant Word Is Kept So a Test Can Compare It Against Itself

The crate does not hide the redundancy; it names it as the reason:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  /// Slots allocated — always equal to `capacity().get()`, never more.' ring_store/src/lib.rs | tail -n 5
```

Live output:

```
  ///
  /// Present as a distinct reading from [`Buffer::capacity`] precisely so a test
  /// can assert the two agree; a buffer that over-allocated would still report
  /// the requested capacity.
  ///
```

`capacity()` returns the number that was asked for; `len()` returns the number of
slots that exist. Keeping both means a buffer that allocated the wrong amount is
detectable from outside, which it would not be if `capacity()` were implemented
as `slots.len()` or the field were dropped.

The word is not free:

```
--- (1) what a Buffer is made of ---
  size_of Buffer< TypedSlot< u32 > >  = 24
  size_of Buffer< BytesSlot< 4096 > > = 24
  size_of Box< [ TypedSlot< u32 > ] > = 16
  size_of Capacity                    = 8
  the same three words whatever the slot costs
```

Three words rather than two: pointer, length, capacity. And the two always agree:

```
--- (6) capacity stored, and capacity derivable ---
  capacity().get() = 1024, len() = 1024, equal = true
```

**Finding.** Eight bytes per buffer buys an externally checkable allocation
claim. That is a good trade at this scale — a ring holds one buffer, so the cost
is eight bytes per ring, not per slot, and the property it protects is feature
168's first clause: *allocates exactly `N` slots once*.

What is worth recording is that the redundancy is load-bearing in one direction
only. `len()` can catch an over-allocating `new`, because `capacity()` reports the
request and `len()` reports the result. It cannot catch anything else, because
nothing else writes either value after construction — there is no `resize`, no
`push`, no second constructor
([`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md)
BF20). So the field exists
to guard exactly one line of code, `new`, against exactly one mistake, and the
guard is a doctest and a unit test rather than a type. A `Capacity`-sized slice
constructor would have made the property structural instead of asserted; keeping
the word and the test is the cheaper choice and the crate took it.

**Disposition:** declined — this instance's own text names the crate's actual
choice as the cheaper one and endorses it: eight bytes per ring buys an
externally checkable allocation claim, guards the one line (`new`) it can
break, and a `Capacity`-sized slice constructor trading the field away for a
type-level guarantee is presented as the alternative not taken, not a
recommended fix to `ring_store/src/lib.rs`.

---

### BF9 — `is_empty` Exists to Satisfy a Lint and Says So

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  /// Always false — a `Capacity` cannot be zero, so a buffer always has slots.' ring_store/src/lib.rs
```

Live output:

```
  /// Always false — a `Capacity` cannot be zero, so a buffer always has slots.
  ///
  /// Exists because [`Buffer::len`] does; a `len` without an `is_empty` is a
  /// lint, and a hand-written `is_empty` that could disagree with `len` is
  /// worse than one that provably cannot.
  ///
```

Measured against every capacity the type permits:

```
--- (2) is_empty across every capacity a Capacity permits ---
  capacities 1..=32768 (16 powers of two): is_empty() true for any = false
  the smallest possible buffer: len = 1, is_empty = false
  all_empty() on that same buffer = true
```

Never true, for any legal buffer, because `Capacity::new` rejects zero before a
buffer can exist.

**Finding.** This is a function whose entire return value is knowable at compile
time and whose documentation opens by saying so. The chain is honest at every
link: `len()` is kept for BF8's reason, `clippy::len_without_is_empty` fires on
any `len` without a companion, and the companion is written as `self.slots
.is_empty()` rather than as `false` precisely so it cannot drift if the invariant
behind it ever changes.

Recording it as a decision rather than a defect is the accurate reading, and the
residual cost is not correctness but surface. `Buffer` publishes twelve
functions; one of them is a constant, and it occupies the name a caller would
reach for first when asking whether the ring has anything in it. The function
that answers *that* question is `all_empty`, three characters away, and the
crate's own consumers call neither
([`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md)
BF3). The trap the pairing sets is
[`pitfall/001`](../pitfall/001_the_two_questions_named_is_empty.md).

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](001_panic_rather_than_option.md) | The crate's other argued decision |
| [`pitfall/001`](../pitfall/001_the_two_questions_named_is_empty.md) | What the `is_empty`/`all_empty` pairing does to a reader |
| [`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md) | The three words this decision produces |
| [`invariant/001`](../invariant/001_capacity_equals_length_always.md) | The property `len` exists to make checkable |
| [`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md) | Where the three size functions sit in the surface |

### Sources

| Fact | Where |
|------|-------|
| The two fields | `ring_store/src/lib.rs:58-63` |
| `len`'s stated reason | `ring_store/src/lib.rs:155-157` |
| `is_empty`'s stated reason | `ring_store/src/lib.rs:175-177` |
| `Capacity` rejects zero | `ring_types/src/capacity.rs:40-51` |
| `size_of` and the sixteen-capacity sweep | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `exactly_n_slots_are_allocated_for_capacity_n` | `len()` against `capacity()` across six capacities — the comparison the field exists for |
| `a_buffer_is_never_empty_because_a_capacity_is_never_zero` | `is_empty` returning false, and `len == capacity` |
| `a_buffer_is_exactly_its_slots_and_its_capacity` | That the three words are all there are |
| *(to create)* | The sixteen-capacity sweep, so the "always false" claim is asserted rather than argued |
