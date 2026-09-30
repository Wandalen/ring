# Item: The Four of a `TypedSlot`

### Scope

**Purpose:** Catalogue `TypedSlot< T >`'s four inherent methods one at a time,
record that two of them are consumed point-free rather than called, and record
that the mutability split between `get` and `take` — which decides how a whole
ring is drained — is explained in two downstream crates and not in this one.

**Responsibility:** `TypedSlot::empty`, `set`, `get`, `take` — what each does,
what each returns, and how each is actually reached.

**In Scope:** `ring_slot/src/lib.rs:95, 120, 133, 154`; the path-form call
sites across `ring_core`, `ring_mpsc`, `ring_spsc`.

**Out of Scope:** `BytesSlot`'s six are
[`item/002`](002_the_six_of_a_bytes_slot.md). The `must_use` attributes on these
signatures are [`api/001`](../api/001_ten_functions_six_const_seven_must_use.md)
SL17 — this instance is about what the methods do, not what guards them.

---

## The Four

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^  \/\/\/ assert!\( TypedSlot::< u8 >::empty\(\)\.get\(\)\.is_none\(\) \);$/{ n1 = NR } n1 && NR == n1 + 3 { print } /^  pub fn set\( &mut self, value : T \) -> Option< T >$/{ print } /^  pub const fn get\( &self \) -> Option< &T >$/{ print } /^  pub fn take\( &mut self \) -> Option< T >$/{ print }' ring_slot/src/lib.rs
```

Live output:

```
  pub const fn empty() -> Self
  pub fn set( &mut self, value : T ) -> Option< T >
  pub const fn get( &self ) -> Option< &T >
  pub fn take( &mut self ) -> Option< T >
```

| Method | Receiver | Returns | Role |
|--------|----------|---------|------|
| `empty` | — | `Self` | Construct. `const`; `Default` delegates here |
| `set` | `&mut self` | `Option< T >` | Produce. Returns what it displaced |
| `get` | `&self` | `Option< &T >` | Read without consuming. `const` |
| `take` | `&mut self` | `Option< T >` | Consume. Leaves the slot empty |

Two construct-or-produce, two read-or-consume; two take `&self`, two take
`&mut self`. Every pairing in that table is load-bearing.

---

### SL25 — `get` and `take` Are Consumed Point-Free, and the Idiom Lives Downstream

Neither is usually called with a receiver. Both are passed as function values:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'TypedSlot::\(get\|take\)' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
  | sed 's|^\([a-z_]*\)/\([a-z]*\)/.*|\1/\2|' | sort | uniq -c
```

Live output:

```
      2 ring_bench/src
      4 ring_core/src
      1 ring_core/tests
      1 ring_event/src
      5 ring_mpsc/src
     17 ring_mpsc/tests
      1 ring_slot/src
      7 ring_spsc/src
     16 ring_spsc/tests
```

Fifty path-form uses across three crates. The shape is always one of two:

```rust
batch.get_mut( offset ).and_then( TypedSlot::take )      // drain, owning
batch.iter().filter_map( TypedSlot::get ).copied()       // drain, borrowing
```

That is the family's entire drain vocabulary, and both halves depend on the
methods being callable without a receiver — which in turn depends on their
signatures being exactly `fn( &T ) -> U` and `fn( &mut T ) -> U` with no extra
argument and no error path. `set` cannot join them: it takes a value, so it has
arity two.

**Correction (2026-09-28):** this section read "Thirty-five path-form uses
across three crates" — `ring_core/src`(4), `ring_mpsc/src`(5),
`ring_mpsc/tests`(13), `ring_spsc/src`(7), `ring_spsc/tests`(7) — until the
census above was re-run. `ring_mpsc`'s and `ring_spsc`'s test suites have both
grown since, and `ring_core` gained a path-form use in its own `tests/` the
original table had no row for at all; the same five categories now sum to
fifty.

**Finding.** `TypedSlot`'s accessors are consumed as function values far more
often than as method calls, and the idiom is documented entirely in the crates
that use it. `ring_slot`'s own doctests call `slot.get()` and `slot.take()` with
a receiver — the form nothing downstream actually uses.

This is the same narrowness [`api/002`](../api/002_a_trait_with_two_methods.md)
SL19 records for `Slot::is_empty` in `ring_store`'s fold, and here it pays off
three crates deep rather than once. The observation worth keeping is that it is
load-bearing and undeclared: adding a parameter to `get` or `take`, or making
either fallible, would compile fine in `ring_slot` and break fifty call
sites elsewhere with no local test to warn of it.

---

### SL26 — The `&self` / `&mut self` Split Is the Whole Drain Contract, and Two Other Crates Explain It

`get` borrows and `take` moves, so a consumer's choice between them is forced
before it reaches a slot at all — by whether it holds `&S` or `&mut S`. The
family says so, twice:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '  /// **This is the only way to move a record out of a batch**, because taking a' ring_spsc/src/lib.rs
echo '---'
command grep -m1 -A2 -F '  /// `get` returns `&S`, and `TypedSlot::take` needs `&mut S`, so before' ring_spsc/tests/spsc_test.rs
```

Live output:

```
    /// **This is the only way to move a record out of a batch**, because taking a
    /// value from a slot needs `&mut` — [`TypedSlot::take`] cannot be reached
    /// through [`get`]. A consumer that reads without consuming uses [`get`]; one
    /// that owns what it drained needs this.
---
    /// `get` returns `&S`, and `TypedSlot::take` needs `&mut S`, so before
    /// `get_mut` existed there was no path from a drained batch to an owned `T` at
    /// all — only to a borrow of one. `ring_mpsc`'s batch had the counterpart from
```

Two statements of the same rule, in `ring_spsc`'s source and `ring_spsc`'s test
suite. The second records history: `Batch::get_mut` was added *because* the
split made owned drains impossible without it, and `ring_mpsc` had the
counterpart first.

**Finding.** The consequence of `get` taking `&self` and `take` taking
`&mut self` is a whole API on another crate — `Batch::get_mut`, which exists for
no other reason — and the explanation for it is written twice in `ring_spsc` and
zero times in `ring_slot`.

The split itself is not in question. `get` must be `&self` or it could not be
`const`, and `take` must be `&mut self` because it mutates. Nothing else is
available. What is missing is the sentence at the definition site saying what
the split costs a consumer: a reader of `TypedSlot`'s four methods sees two ways
to look at a payload and no indication that the choice between them propagates
up through `Batch`, `Consumer`, and `Ring` to the shape of a caller's drain loop.

The narrow fix is a line on `get` pointing at `take` for the owning case, and
the reverse. The wider observation is that this crate's four smallest methods
determine an interface three layers up, and the crate documents them as though
they were local.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_the_six_of_a_bytes_slot.md) | The other shape's six, and what they do not offer |
| [`api/001`](../api/001_ten_functions_six_const_seven_must_use.md) | These four among the ten, and the two missing `must_use` |
| [`api/002`](../api/002_a_trait_with_two_methods.md) | Why none of these four is on the trait |
| [`pattern/002`](../pattern/002_the_displaced_value_returned.md) | `set`'s return, and the shape it borrows from `Option::replace` |
| [`lifecycle/001`](../lifecycle/001_a_slot_across_one_publish.md) | These four in the order a publish uses them |

### Sources

| Fact | Where |
|------|-------|
| The four signatures | `ring_slot/src/lib.rs:95, 120, 133, 154` |
| Fifty path-form uses | `ring_core/src`, `ring_core/tests`, `ring_mpsc/src`, `ring_mpsc/tests`, `ring_spsc/src`, `ring_spsc/tests` |
| The split, stated in source | `ring_spsc/src/lib.rs:1048-1051` |
| The split, stated in tests | `ring_spsc/tests/spsc_test.rs:551-553` |
| The split, unstated here | `ring_slot/src/lib.rs:133, 154` — neither doc mentions the other |

### Tests

| Test | Covers |
|------|--------|
| `a_typed_slot_round_trips_its_value` | All four in sequence, and `take` twice |
| `setting_over_a_value_returns_the_displaced_one` | `set`'s return at the occupied state |
| `a_typed_slot_holds_non_copy_payloads` | `take` moving a `String` out |
| `a_fresh_typed_slot_is_empty` | `empty` and `get` at the initial state |
| `ring_spsc` — `a_record_taken_through_get_mut_leaves_its_slot_empty_across_a_wrap` | The `get_mut` + `take` idiom the split forces |
| `ring_spsc` — `get_and_iter_agree_at_every_offset` | The borrowing half, via `filter_map( TypedSlot::get )` |
