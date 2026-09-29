# Pattern: One Owner for One Arithmetic Fact

### Scope

**Purpose:** State the pattern this crate exists to instantiate — one crate owns
one arithmetic fact, and everyone else reaches it — record how the pattern is
enforced, and record that the rule the code actually follows is narrower and
sharper than the one the prose states.

**Responsibility:** The pattern as a rule about where a fold may live, tested
against the four crates that hold ring-shaped storage.

**In Scope:** `ring_index/src/lib.rs:48-52`;
`ring_store/src/lib.rs:59-63, 203-207, 220`;
`ring_mpsc/src/lib.rs:21-25, 299-309, 540-547`;
`ring_spsc/src/lib.rs:239-262`.

**Out of Scope:** the violation as a defect, with its own failure mode, is
[`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md). The consumer
census is
[`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md).

---

## The Pattern as Stated

One arithmetic fact — *which slot does this sequence address* — lives in exactly
one function, in exactly one crate, and every consumer reaches it rather than
re-deriving it. `ring_store`'s own comment states the rule from the consumer
side:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^  \/\/\/ Borrow the slot a sequence addresses, folding through `ring_index`\.$/,/^  \/\/\/ `ring_index` exists to be the only one of\.$/p;/^  pub fn at( &self, seq : Seq ) -> &S$/p' ring_store/src/lib.rs
```

Live output:

```
  /// Borrow the slot a sequence addresses, folding through `ring_index`.
  ///
  /// The convenience that keeps the fold in one place: a caller that wrote its
  /// own `seq % capacity` here would be the second implementation of the thing
  /// `ring_index` exists to be the only one of.
  pub fn at( &self, seq : Seq ) -> &S
```

---

### IX41 — The Pattern Is Enforced by a `use` Line and Audited by Nobody

**Finding.** Nothing in the toolchain expresses this pattern. There is no trait a
consumer must implement, no lint, no visibility restriction — `Capacity::mask` is
`pub const fn`, so writing `seq.0 as usize & capacity.mask()` in any crate that
depends on `ring_types` compiles cleanly and produces the identical answer.

What holds the pattern up is that a violation is *visible* in a way the reviewer
can enumerate: a crate that folds must either name `ring_index` in its manifest
or write the mask itself, and both are one grep away. The census that found this
crate's single violation is four lines of shell.

The finding is that those four lines are not run by anything. They are not a
test, not a CI step, not a `#[ deny ]`. The pattern's entire enforcement is that
somebody eventually greps, and the interval between the violation landing and
somebody grepping is unbounded — in this case long enough that the violating
crate's module comment now documents the violation as the design (IX42).

The asymmetry worth naming: `Capacity`'s power-of-two invariant is enforced by
privacy and is therefore a compile error to break, while the one-owner rule
sitting directly on top of it is enforced by prose and is therefore a code review
someone did not do. Both are called invariants in this corpus. Only one is one.

---

### IX42 — The Rule the Code Follows Is "The Fold Travels With the Container"

The stated pattern — one owner, everyone reaches it — does not predict where the
violation happened. A narrower rule does: the fold lives with the storage it
addresses, so a struct that holds ring-shaped storage either wraps it in
`Buffer` and inherits the fold, or holds it bare and writes one.

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- ring_spsc, one storage --'
command grep -m1 -F '  slots : Buffer< UnsafeCell< S > >,' ring_spsc/src/lib.rs
echo '  -- ring_mpsc, two storages, seven lines apart --'
sed -n '/^  slots : Buffer< UnsafeCell< S > >,$/p;/^  stamps : Box< \[ AtomicSeq ] >,$/p' ring_mpsc/src/lib.rs
echo '  -- and how the second one is addressed --'
command grep -m1 -A7 -F '  /// The stamp cell addressing `seq`.' ring_mpsc/src/lib.rs
```

Live output:

```
  -- ring_spsc, one storage --
  slots : Buffer< UnsafeCell< S > >,
  -- ring_mpsc, two storages, seven lines apart --
  slots : Buffer< UnsafeCell< S > >,
  stamps : Box< [ AtomicSeq ] >,
  -- and how the second one is addressed --
  /// The stamp cell addressing `seq`.
  fn stamp( &self, seq : Seq ) -> &AtomicSeq
  {
    let index = ( seq.0 as usize ) & self.capacity().mask();
    // The mask is `capacity - 1` for a power-of-two capacity, which `Capacity`
    // enforces at construction, so the index is always in range.
    &self.stamps[ index ]
  }
```

**Finding.** `ring_mpsc` obeys the pattern and breaks it in the same struct. Its
`slots` field is a `Buffer`, so every access folds through `Buffer::at` and
therefore through `ring_index`. Its `stamps` field is a bare `Box< [ AtomicSeq ] >`
declared seven lines below, and it needed a fold that nothing supplied, so it
got a hand-written one.

`ring_spsc` holds one storage, wrapped, and has no hand-written fold anywhere.
`ring_store` *is* the wrapper. `ring_batch` holds no storage at all and imports
`of` directly. Four crates, and the rule "does this struct hold ring-shaped bytes
outside a `Buffer`?" predicts the one violation exactly.

That reframes what the fix would be. Adding a `use ring_index::of;` to
`ring_mpsc` would restore the letter of the stated pattern and change nothing
about the actual hazard, which is that a mask derived from `consumers`' capacity
indexes an array sized from a constructor argument stored nowhere. The rule that
would have prevented it is the narrower one: put the stamps in a container that
owns its own capacity, and the fold arrives with it.

The module comment makes this a design statement rather than an oversight:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A1 -F '//! So publication is a `Release` store into `stamps[ seq & mask ]`, and a slot' ring_mpsc/src/lib.rs
```

Live output:

```
//! So publication is a `Release` store into `stamps[ seq & mask ]`, and a slot
//! is published exactly when its stamp equals the sequence addressing it. No
```

`stamps[ seq & mask ]` is written into the crate's own summary of itself, at line
23, above every field it describes. Whatever else it is, it is not accidental,
and a reviewer looking for a second fold would have found it in the first screen
of the file.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/002`](002_stateless_arithmetic_over_borrowed_types.md) | The shape the pattern takes in the two crates that own no type |
| [`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md) | The violation as a defect, and the three-copy capacity hazard behind it |
| [`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md) | The consumer census this pattern is audited by |
| [`type/002`](../type/002_the_sentence_the_public_field_contradicts.md) | The same convention-versus-compiler split, one level down at `SlotIndex` |

### Sources

| Fact | Where |
|------|-------|
| The rule stated from the consumer side | `ring_store/src/lib.rs:203-207, 220` |
| Two storages in one struct | `ring_mpsc/src/lib.rs:329, 336` |
| The hand-written fold | `ring_mpsc/src/lib.rs:540-547` |
| The fold as a documented design | `ring_mpsc/src/lib.rs:23` |
| One storage, cell-per-slot | `ring_spsc/src/lib.rs:260` |

### Tests

| Test | Covers |
|------|--------|
| `ring_batch::drain_order_folds_through_ring_index_and_not_a_second_implementation` | The pattern asserted, in `ring_batch`, as a test — declared there, not here |
| *(to create)* | The same assertion for `ring_mpsc::stamp`, which would fail today |
| *(to create)* | A census test: no crate outside `ring_index` writes `& capacity.mask()` |
