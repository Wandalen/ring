# Invariant: A Read Returns What Was Written

### Scope

**Purpose:** Record the crate's load-bearing property — a `BytesSlot` reads back
exactly the bytes last written and never the tail behind them — what upholds it,
and the one path that escapes it.

**Responsibility:** The relationship between `write` and `read` on
`BytesSlot< N >`, at every length from empty to full.

**In Scope:** `ring_slot/src/lib.rs:328-370`;
`ring_slot/tests/slot_test.rs:207-223, 274-286`.

**Out of Scope:** `Debug` and `PartialEq`'s relationship to the tail — once two
derives that did not honour the property, now hand-written impls that do — is
[`pitfall/001`](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md).
Emptiness specifically is
[`invariant/002`](002_the_two_emptiness_paths_agree.md).

---

## The Property and Its Two Halves

The invariant is one sentence with two halves, and the crate states it as such:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'reads back exactly\|and only those\|never the unused tail' ring_slot/src/lib.rs
```

Live output:

```
//! partially-filled slot reads back exactly what was written and nothing else,
    /// The bytes written, and only those — never the unused tail.
```

Half one — *reads back exactly what was written* — is upheld by `write` copying
the payload to the front of the array. Half two — *and nothing else* — is upheld
by `read` slicing to `self.len` rather than to `N`. Neither half alone is
sufficient, and the two are implemented eleven lines apart:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^    self\.bytes\[ \.\.payload\.len() ]\.copy_from_slice( payload );$/,/^    Ok( () )$/p;/^  pub fn read( &self ) -> &\[ u8 ]$/,/^  }$/p' ring_slot/src/lib.rs
```

Live output:

```
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
    self.len = payload.len();
    Ok( () )
  pub fn read( &self ) -> &[ u8 ]
  {
    &self.bytes[ ..self.len ]
  }
```

---

### SL9 — The Invariant Is Asserted at Every Length, Not at a Sample

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A16 -F '/// A read returns the written bytes and only those — never the unused tail.' ring_slot/tests/slot_test.rs
```

Live output:

```
/// A read returns the written bytes and only those — never the unused tail.
/// Asserted across every length from empty to full, because an off-by-one in
/// the length would leak exactly one stale byte.
#[ test ]
fn a_read_never_returns_the_unused_tail()
{
  const CAP : usize = 8;
  let payload = b"abcdefgh";

  for len in 0..=CAP
  {
    let mut slot = BytesSlot::< CAP >::empty();
    slot.write( &payload[ ..len ] ).unwrap();
    assert_eq!( slot.read(), &payload[ ..len ], "length {len}" );
    assert_eq!( slot.read().len(), len );
  }
}
```

Nine cases, `0..=8` inclusive, each on a fresh slot. The inclusive upper bound
matters: `len == CAP` is the boundary where a `>` and a `>=` in `write` differ,
and it is covered.

**Finding.** The test enumerates the whole domain rather than sampling it, and
its doc comment states why in one line — an off-by-one leaks exactly one stale
byte, which no sampled length would reliably catch. This is the strongest test in
the crate and the reason the invariant can be relied on: for `N == 8` the
property is not argued, it is exhausted.

The exhaustion is over lengths on a *fresh* slot, which leaves the overwrite case
to a separate test — the one that actually exercises a non-zero tail.

---

### SL10 — The Overwrite Case Is Covered Through `read` and Nowhere Else

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A12 -F '/// A shorter write over a longer one truncates the reading — the stale tail must' ring_slot/tests/slot_test.rs
```

Live output:

```
/// A shorter write over a longer one truncates the reading — the stale tail must
/// not reappear, which across ring laps would be a data leak between publishes.
#[test]
fn a_shorter_write_does_not_leak_the_longer_one() {
    let mut slot = BytesSlot::<8>::empty();
    slot.write(b"AAAAAAAA").unwrap();
    assert_eq!(slot.read(), b"AAAAAAAA");

    slot.write(b"bb").unwrap();
    assert_eq!(slot.read(), b"bb", "the previous payload's tail must not reappear");
    assert_eq!(slot.len(), 2);
}
```

This is the only test in the crate that produces a slot whose tail differs from
zero, and every assertion it makes goes through `read()`. Through `read()` the
invariant holds — exactly as claimed.

**Finding.** The invariant is stated as a property of the *slot* ("a
partially-filled slot reads back exactly what was written and nothing else") and
verified as a property of `read()`. For the four accessors that is the same
thing. For two derived traits it once was not: the same slot this test builds
used to be distinguishable from a freshly-written one by `==` and printed its
full tail under `Debug`, until SL41 and SL42's dispositions replaced both with
hand-written impls over `read()`
([`pitfall/001`](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md)).

So the invariant as written is true and the invariant as a reader will understand
it is narrower than it sounds. The precise statement is: *`read` returns what was
written and nothing else.* The crate's own phrasing — "a partially-filled slot
reads back exactly what was written and nothing else" — is what invites the wider
reading, and it is the module comment, the most-read line in the crate.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](002_the_two_emptiness_paths_agree.md) | The second property — that the two emptiness paths can never disagree |
| [`pitfall/001`](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md) | The two traits — now hand-written, once derived — that used to read the tail this invariant is about |
| [`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md) | The three steps of `write`, and where the length is set |
| [`lifecycle/002`](../lifecycle/002_a_slot_across_a_rings_laps.md) | Why the leak framing is about laps rather than about one write |

### Sources

| Fact | Where |
|------|-------|
| The invariant, module-level | `ring_slot/src/lib.rs:22-24` |
| The invariant, on `read` | `ring_slot/src/lib.rs:358` |
| `write`'s copy and length | `ring_slot/src/lib.rs:353-354` |
| `read`'s length bound | `ring_slot/src/lib.rs:367-370` |
| Exhaustive length coverage | `ring_slot/tests/slot_test.rs:207-223` |
| The one non-zero-tail fixture | `ring_slot/tests/slot_test.rs:274-286` |

### Tests

| Test | Covers |
|------|--------|
| `a_read_never_returns_the_unused_tail` | Every length `0..=8` on a fresh slot — the invariant, exhausted |
| `a_shorter_write_does_not_leak_the_longer_one` | The overwrite case, through `read()` |
| `a_write_of_exactly_capacity_is_accepted` | The inclusive upper boundary |
| `a_bytes_slot_round_trips_its_payload` | The simple round trip, and `len` agreeing with what was written |
