# Lifecycle: A Slot Across One Publish

### Scope

**Purpose:** Record the states a slot passes through in a single publish, that
the family has two incompatible models for doing it, and that only one of them
works for both shapes.

**Responsibility:** The state sequence of one slot from empty through occupied
and back, under each of the family's two drain models.

**In Scope:** `ring_event/src/lib.rs` — `publish_into`, `drain_from` and
`recycle`, and the `Fill`/`Peek` impls for both slot shapes; the `get_mut`+`take`
idiom in `ring_mpsc` and `ring_spsc`. Addressed by name rather than by line,
because SL29's own fix moved every line number this section used to carry.

**Out of Scope:** What survives between publishes — the stale tail across laps —
is [`lifecycle/002`](002_a_slot_across_a_rings_laps.md). That `clear` forgets
rather than erases is
[`pitfall/002`](../pitfall/002_clear_forgets_it_does_not_erase.md).

---

## Two Models, Named

`ring_event` names the operations a ring performs on a slot and provides one
function for each:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E '^pub fn (publish_into|drain_from|recycle)' ring_event/src/lib.rs
```

Live output:

```
pub fn publish_into<S, P>(slot: &mut S, payload: P) -> Result<(), RingError>
pub fn drain_from<S>(slot: &S) -> Option<S::Out<'_>>
pub fn recycle<S>(slot: &mut S)
```

Three operations, three receivers: `&mut`, `&`, `&mut`. `ring_mpsc` and
`ring_spsc` use a different sequence entirely — `push` then
`get_mut( i ).and_then( TypedSlot::take )` — which has two steps, not three.

---

### SL29 — Draining Through `ring_event` Never Empties the Slot; `recycle` Is Mandatory

`drain_from` takes `&S` and delegates to `Peek::peek`, which for both shapes
returns a borrow:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A8 -F 'impl< T > Peek for TypedSlot< T >' ring_event/src/lib.rs
command grep -m1 -A8 -F 'impl< const N : usize > Peek for BytesSlot< N >' ring_event/src/lib.rs
```

Live output:

```
impl< T > Peek for TypedSlot< T >
{
  type Out< 'a > = &'a T where T : 'a;

  fn peek( &self ) -> Option< &T >
  {
    self.get()
  }
}
impl< const N : usize > Peek for BytesSlot< N >
{
  type Out< 'a > = &'a [ u8 ];

  fn peek( &self ) -> Option< &[ u8 ] >
  {
    if self.is_empty() { None } else { Some( self.read() ) }
  }
}
```

`self.get()` and `self.read()` — the two `&self` accessors. Neither mutates, so
after a drain the slot still holds what it held. A test asserts the whole
sequence, for both shapes, in one body:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A24 -F 'fn draining_reads_the_slot_without_emptying_it()' ring_event/tests/event_test.rs \
  | command grep -E 'assert|recycle\( '
```

Live output:

```
  assert_eq!( drain_from( &typed ), Some( &7 ) );
  assert_eq!( drain_from( &bytes ), Some( &b"payload"[ .. ] ) );
  assert!( !typed.is_empty(), "the typed slot is still occupied after being drained" );
  assert!( !bytes.is_empty(), "and so is the byte slot" );
  assert_eq!( drain_from( &typed ), Some( &7 ) );
  assert_eq!( drain_from( &bytes ), Some( &b"payload"[ .. ] ) );
  recycle( &mut typed );
  recycle( &mut bytes );
  assert!( typed.is_empty(), "the third call is the one that empties it" );
  assert!( bytes.is_empty(), "for both shapes, through the same generic body" );
```

Both shapes behave identically, which is the point — and both need the third
call. Draining twice returns the same payload twice, which is the observable
form of the same fact: nothing was consumed.

```text
cargo nextest run -p ring_event draining_reads_the_slot_without_emptying_it
```

**Finding.** In `ring_event`'s model a slot's cycle is `empty → occupied →
occupied-and-read → empty`, four states with three transitions, and the last
transition is a separate call a caller must remember to make. Forgetting it is
not a compile error and not a runtime error: the ring keeps working, the slot
keeps reporting non-empty, and any consumer asking `all_empty()` gets `false`
for a fully-drained ring.

`ring_event`'s own doc gives the reason the third operation exists — "a
shape-specific reset would be a fourth path the identical-path claim does not
cover" — and that reason is about uniformity across shapes, not about safety. It
justifies why `recycle` is generic; nothing justifies why it is *separate*, and
separate is what makes it forgettable.

The cost is one line per drain and the failure mode is silent. A `drain_and_recycle`
composing the two would remove it, at the price of forbidding the read-without-
consuming case that `Peek`'s borrow return exists to allow — so the split is
defensible. It is the absence of a warning that is not.

**Disposition:** applied — `drain_from`'s own documentation now opens by denying
its name — "**This does not empty the slot, and the name is the trap.**" — states
that `recycle` is a mandatory third call, and names the failure precisely: "a
drained ring every slot of which reads occupied". Its doctest asserts
`!slot.is_empty()` after the read and `slot.is_empty()` only after `recycle`, so
the warning is executable rather than advisory, and
`draining_reads_the_slot_without_emptying_it` in `ring_event/tests/event_test.rs`
asserts the same sequence for both shapes with the second drain returning the
same payload again. The finding's own preferred remedy — a composed
`drain_and_recycle` — is declined for the reason the finding itself gives:
`Peek`'s borrow return exists to allow reading without consuming, and a composed
call would forbid it. The warning was the gap; the split was not. Falsified by
replacing `recycle`'s body with `let _ = slot;` and re-running the test, which
panicked at `the third call is the one that empties it`.
Now prints: `assert!( typed.is_empty(), "the third call is the one that empties it" );`

---

### SL30 — The Other Model Empties as a Side Effect, and Only `TypedSlot` Can Use It

`ring_mpsc` and `ring_spsc` drain with `TypedSlot::take`, which mutates:

```
--- the ring_mpsc model: take empties as a side effect ---
typed: took Some(7), slot empty without any recycle: true
```

Two steps, no third call, no way to forget it — the slot is emptied by the same
operation that reads it. That is the safer shape, and `BytesSlot` cannot have it:

```
--- and why BytesSlot cannot use that model ---
bytes: read "payload", slot empty: false
bytes: `read` borrows; there is no `take` on this shape
```

There is no `BytesSlot::take`, and there could not usefully be one: taking a
`[ u8; N ]` by value would copy `N` bytes out of a slot the ring owns and will
overwrite anyway, which is exactly the cost the shape exists to avoid
([`decisions/001`](../decisions/001_two_shapes_rather_than_one.md)).

| Model | Steps | Empties on drain? | Works for | Failure if a step is missed |
|-------|:-----:|:-----------------:|-----------|-----------------------------|
| `ring_event` — publish/drain/recycle | 3 | ✘ | Both shapes | Slot reads occupied forever; silent |
| `ring_mpsc`/`ring_spsc` — push/take | 2 | ✔ | `TypedSlot` only | Not possible |

**Finding.** The family runs two lifecycle models and the safer one is the
narrower one. `TypedSlot` gets a self-clearing drain because `Option::take`
gives it one for free; `BytesSlot` gets a borrow-and-recycle drain because a
moving read would defeat its purpose. Neither crate is wrong and the two are not
reconcilable — the difference is forced by what the payload *is*.

What is missing is any statement of it. `ring_slot` documents `take` and `read`
as accessors without noting that one leaves the slot empty and the other does
not, and a reader moving from a `TypedSlot` ring to a `BytesSlot` one carries
over a two-step habit that silently becomes wrong. The place for that sentence
is `read`'s doc — "unlike [`TypedSlot::take`], this borrows and leaves the slot
occupied; use [`Slot::clear`] to release it" — and it is not there.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](002_a_slot_across_a_rings_laps.md) | What the next lap finds, and what `clear` did not remove |
| [`item/001`](../item/001_the_four_of_a_typed_slot.md) | `take` and `get`, and the mutability split behind these two models |
| [`item/002`](../item/002_the_six_of_a_bytes_slot.md) | `read`, and why there is no `take` beside it |
| [`api/002`](../api/002_a_trait_with_two_methods.md) | Why `clear` is on the trait and the accessors are not |
| [`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md) | The publish transition, in three steps |

### Sources

| Fact | Where |
|------|-------|
| The three named operations | `ring_event/src/lib.rs:173, 207, 229` |
| `Peek` for both shapes borrows | `ring_event/src/lib.rs:127-135, 137-145` |
| `recycle`'s stated reason | `ring_event/src/lib.rs:216-218` |
| The two-step idiom | `ring_mpsc/src/lib.rs:1233-1235`, `ring_spsc/src/lib.rs:1090` |
| Every state observation | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_typed_slot_round_trips_its_value` | The two-step model — `set`, `take`, empty |
| `clearing_a_bytes_slot_empties_the_reading` | The third step, on the shape that needs it |
| `clearing_a_typed_slot_is_idempotent` | `recycle` after a `take` — harmless, not an error |
| `ring_event` — the `recycle` doctest | The three-step model end to end |
| *(to create)* | A `BytesSlot` drained without `recycle`, asserting it still reads occupied |
