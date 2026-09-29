# Workaround: `BatchTooLarge` Borrowed for a Different Shape

### Scope

**Purpose:** Record that `write`'s only error variant belongs to a different
concept — a batch against a ring — that its field documentation and rendered
message both describe that other concept, and that the borrow is nonetheless the
right call given what `RingError` offers.

**Responsibility:** `RingError::BatchTooLarge` as `ring_slot` uses it: the units
it carries, the message it produces, and the classification it inherits.

**In Scope:** `ring_slot/src/lib.rs:351`;
`ring_types/src/error.rs:63-71, 115-118, 175-178`; the four crates that
construct the variant.

**Out of Scope:** `write`'s step sequence and the ordering that makes a refusal
total is [`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md).
The `const` question is
[`workaround/001`](001_four_functions_that_could_be_const.md).

---

## The Variant, as `ring_types` Declares It

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A8 -F '  /// A batch of the requested length cannot be served — the ring'"'"'s whole' ring_types/src/error.rs
```

Live output:

```
  /// A batch of the requested length cannot be served — the ring's whole
  /// capacity is smaller than the request, so no amount of draining helps.
  BatchTooLarge
  {
    /// Slots asked for.
    requested : usize,
    /// Slots the ring has in total.
    capacity : usize,
  },
```

A batch. A ring. Slots asked for, slots the ring has. Every noun in the
declaration belongs to the multi-slot reservation path.

---

### SL51 — `ring_slot` Fills Slot-Denominated Fields with Bytes, and the Message Says So

Four crates construct the variant, and one of them means something different by
it:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r 'RingError::BatchTooLarge' ring_*/src/*.rs | grep -vE ':[[:space:]]*//' \
  | grep -v '^ring_types/' | sort
```

Live output:

```
ring_batch/src/lib.rs:    return Err( RingError::BatchTooLarge { requested : count, capacity : capacity.get() } );
ring_claim/src/lib.rs:      return Err( RingError::BatchTooLarge
ring_gating/src/lib.rs:      return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
ring_slot/src/lib.rs:      return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
```

`ring_batch`, `ring_claim`, and `ring_gating` pass a `count` of slots against a
ring's capacity — exactly what the fields document. `ring_slot` passes
`payload.len()` against `N`: a count of **bytes** against a **single slot's**
width. Same variant, same field names, different unit and different scope.

The `Display` arm cannot tell the difference, because nothing in the value says
which crate built it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '      Self::BatchTooLarge { requested, capacity } =>' ring_types/src/error.rs
```

Live output:

```
      Self::BatchTooLarge { requested, capacity } =>
      {
        write!( f, "batch of {requested} exceeds ring capacity {capacity}" )
      }
```

So a twenty-byte payload refused by a `BytesSlot< 16 >` reports, verbatim:

```
  payload      : 20 bytes
  slot capacity: 16 bytes
  Debug        : BatchTooLarge { requested: 20, capacity: 16 }
  Display      : batch of 20 exceeds ring capacity 16
```

**Finding.** The message names a batch that does not exist and a ring capacity
that is not involved — the ring in question might hold a thousand slots, and its
capacity has nothing to do with this refusal. A caller logging the error learns
three wrong things: that a batch was attempted, that the ring is the constraint,
and that the numbers are slot counts.

Nothing catches it, because the crate's own test asserts the structure and never
the rendering:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'Display\|to_string()\|format!' ring_slot/tests/slot_test.rs \
  || echo '  the suite never renders an error'
```

Live output:

```
  assert_eq!( format!( "{cleared:?}" ), "BytesSlot { payload: [] }" );
  assert!( !format!( "{cleared:?}" ).contains( "115" ), "no byte of `secret` survives into the printed form" );
```

`an_oversized_write_is_refused_with_both_numbers` compares against
`Err( RingError::BatchTooLarge { requested : 5, capacity : 4 } )` — the two
numbers, which are right, and never the sentence built from them, which is not.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_slot
command grep -F 'names a multi-slot reservation this call never makes' src/lib.rs
```

Live output:

```
  /// wording that names a multi-slot reservation this call never makes —
```

**Disposition:** applied — `write`'s own `# Errors` doc comment no longer lets
a reader assume the borrowed variant renders faithfully; it now says the
message still reads "batch" and "ring capacity" even though this call never
reserves a batch or touches the ring's capacity, and that `RingError` has no
byte-denominated variant to borrow instead. The `ring_types` field-doc side of
this finding is a different crate's file and is out of this crate's own
`src/`, `docs/`, `Cargo.toml`, so it is not touched here.
Now prints: `names a multi-slot reservation this call never makes`

---

### SL52 — The Borrow Is Still the Correct Call, and That Is Why It Needs Recording

`ring_slot` sits at Tier 1 and depends on `ring_types` alone. Its options for an
oversized write were: borrow an existing variant, add one to `ring_types`, or
define a crate-local error type. The third fragments the family's single error
type for one condition. The second widens a Tier 0 crate's public enum — a
breaking change to every consumer's exhaustive `match` — for a crate that had not
shipped a caller yet.

Borrowing also inherits a classification, and the inherited one happens to be
right:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -A2 -F '      | Self::CapacityNotPowerOfTwo( _ )' ring_types/src/error.rs
```

Live output:

```
      Self::CapacityZero
      | Self::CapacityNotPowerOfTwo( _ )
      | Self::BatchTooLarge { .. }
      | Self::PolicyUnsupported => true,
```

`is_configuration()` returns `true`, `is_transient()` returns `false` — measured
on the refusal above. For `ring_batch` that reading holds because the ring's
capacity is fixed at construction; for `ring_slot` it holds because `N` is fixed
at *compile time*, which is stronger. Retrying cannot help either caller, and the
one classification serves both.

**Finding.** This is a workaround that is currently better than the alternative,
which is precisely the kind that goes unexamined until it breaks. The borrow
costs one wrong sentence in a log; the fix costs a new variant in a Tier 0 crate
and a `match` arm in every consumer of `RingError`.

Two things would make it safe to leave. A one-line note at the construction site
saying the variant is borrowed and the message will name a batch — so the next
reader does not spend time looking for the batch. And a note on the variant's own
declaration in `ring_types` saying its fields are slots *or* bytes depending on
the constructor — so the next person tightening `RingError` does not narrow the
field docs to slots and silently make `ring_slot`'s use a lie in the type system
as well as in the prose.

Neither note exists. What exists instead is a variant whose documentation
describes three of its four construction sites, and a fourth site that matches
none of the words.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_four_functions_that_could_be_const.md) | The crate's other absorbed constraint, in the function signatures |
| [`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md) | The bound check that produces this error, and what it leaves untouched |
| [`item/002`](../item/002_the_six_of_a_bytes_slot.md) | `write` among the six, and the `Result` it alone returns |
| [`integration/001`](../integration/001_seven_dependents_and_four_that_stay_generic.md) | The `ring_types` edge that made borrowing the cheap option |
| [`invariant/001`](../invariant/001_a_read_returns_what_was_written.md) | The property preserved when the write is refused |

### Sources

| Fact | Where |
|------|-------|
| The variant and its field docs | `ring_types/src/error.rs:63-71` |
| The `Display` arm | `ring_types/src/error.rs:175-178` |
| The `is_configuration` membership | `ring_types/src/error.rs:115-118` |
| The four construction sites | `ring_batch/src/lib.rs:318`, `ring_claim/src/lib.rs:425`, `ring_gating/src/lib.rs:287`, `ring_slot/src/lib.rs:351` |
| No rendered-error assertion in the suite | `ring_slot/tests/slot_test.rs` — no occurrence |
| The rendered message and classification | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `an_oversized_write_is_refused_with_both_numbers` | The variant and both fields, structurally |
| `a_failed_write_leaves_the_previous_contents_intact` | That the refusal is total, which the error implies |
| `a_failed_write_onto_an_empty_slot_leaves_it_empty` | The same, from the empty side |
| `a_write_of_exactly_capacity_is_accepted` | The boundary the error does *not* fire at |
| *(to create)* | The rendered `Display` string for a slot-sized refusal, pinning the borrowed wording |
