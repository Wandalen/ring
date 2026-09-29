# API: The Argument Order Split

### Scope

- **Purpose**: Record that `laps_between` takes its two positions in the opposite order from the crate's other three binary functions, and assess what that costs.
- **Responsibility**: Show the split in the signatures, show it in the test suite where both conventions appear three lines apart, and state why the obvious fix is not free.
- **In Scope**: Argument order across `ring_seqno`'s four binary functions.
- **Out of Scope**: Return-type variation across the same four — see [`type/001`](../type/001_what_a_span_is_measured_in.md).

### The Split

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^pub fn ' ring_seqno/src/lib.rs
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

| Function | First parameter | Second parameter | Convention |
|----------|-----------------|------------------|------------|
| `laps_between` | `earlier : Seq` | `later : Seq` | **earlier first** |
| `may_claim` | `producer : Seq` | `consumer : Seq` | later first |
| `free_slots` | `producer : Seq` | `consumer : Seq` | later first |
| `pending` | `producer : Seq` | `consumer : Seq` | later first |

In a ring the producer is always the later position — that is what makes it the
producer. So `laps_between` is alone: three functions take `( later, earlier )`
and one takes `( earlier, later )`.

**Finding SQ8.**

### Both Conventions Appear Three Lines Apart

`tests/seq_test.rs:134-148` is the crate's most important test, and it is where
the split is most visible:

```rust
let consumer = Seq( 8 );
let producer = Seq( 800 );

assert_eq!( producer.0 % 8, consumer.0 % 8 );
assert_eq!( laps_between( consumer, producer, c ), 99 );   // ← consumer first
assert!( !may_claim( producer, consumer, c ) );            // ← producer first
assert_eq!( free_slots( producer, consumer, c ), 0 );      // ← producer first
```

Every line is correct. A reader checking the test has to hold two conventions at
once across three consecutive assertions, and the only thing telling them which
is which is the parameter names in a different file.

This is precisely the hazard M3 in the manual plan is about — except M3 worries
about a caller *swapping* arguments and reads the design as protecting against
it:

> `laps_between( later, earlier )` and `pending( consumer, producer )` are easy to
> call backwards. The design choice is to return zero rather than a huge number.

The protection is real and it works. But M3's own example sentence writes
`laps_between( later, earlier )` as the mistaken form and `pending( consumer,
producer )` as the mistaken form — which are opposite orders, because the two
functions have opposite conventions. The manual plan states the split without
naming it.

### Why the Saturation Masks It

Calling any of the four backwards produces `0` rather than a wrong number,
because `Seq::distance_to` saturates:

| Call | Intent | Result | Visible? |
|------|--------|--------|:--------:|
| `laps_between( later, earlier, c )` | laps apart | `0` | reads as "same lap" |
| `pending( consumer, producer )` | unread count | `0` | reads as "caught up" |
| `may_claim( consumer, producer, c )` | may publish | `true` | reads as "room available" |
| `free_slots( consumer, producer, c )` | free count | `capacity` | reads as "empty ring" |

The first two are safe-ish: a caller who swapped them sees "nothing to do" and
does nothing. **The last two are not.** `may_claim` returning `true` and
`free_slots` returning `capacity` are both permissive answers — a producer acting
on a swapped `may_claim` publishes into a slot that may still be being read.

So the saturation design that M3 credits is protective for the two readings
whose degenerate value is inert, and actively dangerous for the two whose
degenerate value is a green light. That distinction is not drawn anywhere in the
source, and the crate has no test that calls any function backwards on purpose
except `laps_backward_read_zero` (`seq_test.rs:49-53`), which covers one of the
two safe cases.

### What the Fix Would Cost

Changing `laps_between` to `( producer, consumer, capacity )` would make all four
uniform. It is a small change with an outsized blast radius for its size:

| Cost | Detail |
|------|--------|
| Silent at the call site | Both parameters are `Seq`. Swapping them compiles, and the wrong answer is `0` — the same value a correct call often returns |
| Four assertions to invert | `seq_test.rs:29-33, 42-43, 52, 145` |
| One doctest to rewrite | `src/lib.rs:44-47` |
| Nothing in the family breaks | `laps_between` has zero callers outside this crate — see [`workaround/002`](../workaround/002_laps_between_has_no_caller.md) |

The last row is the interesting one: the change is *free* in the family, because
nothing calls the function. That is an argument for doing it, and equally an
argument that it does not matter. It is not done here — a signature change is a
change with its own verification run, and this crate's whole documentation set
records rather than repairs.

**A cheaper mitigation exists and is also not taken:** the type system can carry
what the parameter names currently carry alone. Newtypes `Producer( Seq )` and
`Consumer( Seq )` would make every swap a compile error across all four
functions, at the cost of a wrapper at every call site. That is a family-wide
design question, not this crate's to settle — recorded in
[`decisions/002`](../decisions/002_saturating_rather_than_signed.md) § What a
Signed Distance Would Have Changed.

### Why the Order Is What It Is

`laps_between`'s order is not arbitrary — it reads as English. "Laps between
*earlier* and *later*" puts the earlier one first, the way `a..b` does, and the
`distance_to` method it calls has the same shape (`earlier.distance_to( later )`).

The other three read as questions about a producer: "may this producer claim",
"how many slots free for this producer", "how much is pending for this producer".
The subject comes first, and the subject is the producer.

Both conventions are locally defensible. That is exactly why the split survived
review — each function, read alone, has its arguments in the obvious order.

### SQ8 — Three Adjacent Lines, Two Conventions

The test file itself demonstrates the split, without remarking on it:

```
seq_test.rs:145   assert_eq!( laps_between( consumer, producer, c ), 99 );
seq_test.rs:146   assert!( !may_claim( producer, consumer, c ) );
seq_test.rs:147   assert_eq!( free_slots( producer, consumer, c ), 0 );
```

**Finding.** `laps_between( earlier, later, … )` reverses the order of `may_claim`, `free_slots` and `pending`, all of which take the later position first, and `seq_test.rs:145-147` uses both conventions on three adjacent lines.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A4 -F '/// Takes `( earlier, later, .. )`' ring_seqno/src/lib.rs
```

Live output:

```
/// Takes `( earlier, later, .. )` — the opposite order from [`may_claim`],
/// [`free_slots`] and [`pending`], which all take the later (producer)
/// position first. A swapped call still compiles and still returns a
/// plausible-looking `0` rather than an error (see `api/002` SQ8).
///
```

**Disposition:** applied — added a doc-comment note to `laps_between` naming the reversed order relative to `may_claim`, `free_slots` and `pending`, and pointing at this finding, so the split is visible at the one place a caller outside this crate would actually see it: the signature's own doc. Verified via `cargo test -p ring_seqno --all-features`, 2026-09-04 — ring_seqno's 11 unit tests plus 5 doctests all pass, including `laps_between`'s own doctest. Now prints:
`position first. A swapped call still compiles and still returns a`

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | The four functions, and the test where both conventions meet |

### APIs

| File | Relationship |
|------|--------------|
| [001_five_functions_and_no_types.md](001_five_functions_and_no_types.md) | The surface these four sit in |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_saturating_rather_than_signed.md](../decisions/002_saturating_rather_than_signed.md) | Why a swap yields `0` rather than an error |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_reading_free_slots_on_a_narrow_target.md](../pitfall/002_reading_free_slots_on_a_narrow_target.md) | The other way these four can disagree |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_what_a_span_is_measured_in.md](../type/001_what_a_span_is_measured_in.md) | The return-type variation across the same four |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:50, 73, 95, 112` | The four signatures |
| `ring_types/src/id.rs:82` | `distance_to`, whose shape `laps_between` follows |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:145-147` | Both conventions, three adjacent lines |
| `tests/seq_test.rs:49-53` | The one deliberate backwards call — a safe case |
| `tests/manual/readme.md` M3 | States the hazard without naming the split |
