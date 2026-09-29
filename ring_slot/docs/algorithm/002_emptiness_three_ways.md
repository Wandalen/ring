# Algorithm: Emptiness, Three Ways

### Scope

**Purpose:** Record the three distinct computations the crate uses to answer
"does this slot hold anything?", why they cannot be unified, and the fourth
question — asked one layer up, of a whole buffer — that had to be renamed to
avoid colliding with a fifth.

**Responsibility:** The emptiness computations across both shapes and both access
paths, and the buffer-level fold that composes them.

**In Scope:** `ring_slot/src/lib.rs:178-181, 323-326, 387-390`;
`ring_store/src/lib.rs:138-141, 173-177, 187-191`.

**Out of Scope:** That the two `BytesSlot` paths agree is
[`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md). This
instance is about the computations themselves.

---

## The Three Computations

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- TypedSlot, trait path (the only path) ---'
command grep -m1 -A5 -F 'impl< T > Slot for TypedSlot< T >' ring_slot/src/lib.rs | tail -n 4
echo '--- BytesSlot, inherent path ---'
command grep -m1 -A3 -F '  pub const fn is_empty( &self ) -> bool' ring_slot/src/lib.rs
echo '--- BytesSlot, trait path ---'
command grep -m1 -A5 -F 'impl< const N : usize > Slot for BytesSlot< N >' ring_slot/src/lib.rs | tail -n 4
```

Live output:

```
--- TypedSlot, trait path (the only path) ---
  fn is_empty( &self ) -> bool
  {
    self.0.is_none()
  }
--- BytesSlot, inherent path ---
  pub const fn is_empty( &self ) -> bool
  {
    self.len == 0
  }
--- BytesSlot, trait path ---
  fn is_empty( &self ) -> bool
  {
    Self::is_empty( self )
  }
```

Three function bodies, two computations: an `Option` discriminant test and an
integer comparison. The third is a delegation and computes nothing.

---

### SL15 — The Two Real Computations Read Different Things and Cannot Be Merged

`TypedSlot` reads a discriminant; `BytesSlot` reads a counter. There is no
representation that serves both without cost:

| Shape | Reads | Cost | Distinguishes "published nothing"? |
|-------|-------|------|:----------------------------------:|
| `TypedSlot< T >` | `Option< T >` discriminant | Free — the niche is already there for `Option` | ✔ `Some( () )` ≠ `None` |
| `BytesSlot< N >` | `len == 0` | Free — the length is needed for `read` anyway | ✘ `write( b"" )` is `len == 0` |

Both are free because both read a field the shape carries for another reason. A
unified computation would need a field that neither shape currently has — a
discriminant on `BytesSlot`, or a length on `TypedSlot` — and would cost a byte
per slot on whichever shape gained it.

**Finding.** The two computations are not an inconsistency to be resolved; they
are each the cheapest reading of a field the shape already has. That is why the
`Slot` trait declares `is_empty` as a method rather than deriving it from
something more primitive: there is no more primitive thing both shapes share.

The asymmetry in the last column is the price, and it is the same asymmetry
[`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) SL8 records —
`TypedSlot< () >` can signal "somebody published nothing" and `BytesSlot< N >`
cannot, because the discriminant carries a bit that the length does not.

---

### SL16 — The Fold Belongs to `ring_store`, and `ring_store` Renamed It to Avoid the Collision

The family asks a fourth question that no slot can answer: is the whole *buffer*
empty? `ring_store` answers it — and does not call the answer `is_empty`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A14 -F '  /// Whether every slot is empty.' ring_store/src/lib.rs
```

Live output:

```
  /// Whether every slot is empty.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let buffer : Buffer< TypedSlot< u8 > > = Buffer::new( Capacity::new( 2 ).unwrap() );
  /// assert!( buffer.all_empty() );
  /// ```
  #[ must_use ]
  pub fn all_empty( &self ) -> bool
  {
    self.slots.iter().all( Slot::is_empty )
  }
```

`Slot::is_empty` appears there as a function value — the one place in the family
where the trait method is used without a receiver — folded over every slot with
`all`.

The name matters because `Buffer` *also* has an `is_empty`, twenty lines further
down, answering a different question entirely:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^  \/\/\/ Always false — a `Capacity` cannot be zero, so a buffer always has slots\.$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 4 { print } /^  \/\/\/ assert!\( !buffer\.is_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_store/src/lib.rs
```

Live output:

```
  /// Always false — a `Capacity` cannot be zero, so a buffer always has slots.
  ///
  /// Exists because [`Buffer::len`] does; a `len` without an `is_empty` is a
  /// lint, and a hand-written `is_empty` that could disagree with `len` is
  /// worse than one that provably cannot.
  #[ must_use ]
  pub const fn is_empty( &self ) -> bool
  {
    self.slots.is_empty()
  }
```

The two disagree on every buffer the family ever builds:

Measured, release:

```
a fresh 4-slot buffer holding nothing:
  buffer.all_empty() = true
  buffer.is_empty()  = false
  they disagree      = true
```

**Finding.** The composition is exact — `all` over a per-element predicate,
short-circuiting, so a buffer whose first slot is occupied costs one comparison
rather than `capacity` comparisons. That cost is available only because
`Slot::is_empty` takes `&self` and returns `bool` with no error path: a fallible
or `&mut self` signature would not compose into `all` at all. The two-method
trait was designed narrowly enough to be usable as a function value, and
`ring_store` is the only crate that exploits it.

The naming is the sharper observation. The obvious name for a fold of
`Slot::is_empty` is `is_empty`, and `ring_store` could not use it: clippy's
`len_without_is_empty` had already claimed `is_empty` on `Buffer` for the
container question — *does this collection have zero elements* — which for a
`Buffer` is provably `false`, since `Capacity` cannot be zero. So the crate
carries a permanently-`false` `is_empty` it did not want, and the question anyone
actually asks is spelled `all_empty`.

The trap that leaves is precise: `Buffer::is_empty()` compiles, is `#[ must_use ]`,
reads exactly like the question a caller means, and returns the wrong answer every
time. `ring_store` mitigated it as well as the method can be mitigated — the
first line of the doc is "Always false" — but the mitigation is a doc comment on
a method whose name does the misleading, and doc comments are read after the name
is believed.

**Disposition:** declined — the trap is `ring_store::Buffer::is_empty()`'s own
permanently-false return colliding with its own name; renaming the method or
otherwise promoting the "Always false" warning out of a doc comment a caller
reads after already believing the name is a change to
`ring_store/src/lib.rs`, a different crate with its own corpus, outside
`ring_slot`'s own `src/`, `docs/`, and `Cargo.toml`. `ring_slot` does not depend
on or reference `ring_store` at all, so no file here restates the claim this
finding corrects.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](001_write_is_a_bound_check_and_a_copy.md) | The crate's only branching computation |
| [`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md) | That the two `BytesSlot` paths cannot diverge |
| [`api/002`](../api/002_a_trait_with_two_methods.md) | Why the trait has exactly these two methods and no more |
| [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) | The bit the length does not carry |

### Sources

| Fact | Where |
|------|-------|
| `TypedSlot`'s computation | `ring_slot/src/lib.rs:178-181` |
| `BytesSlot`'s inherent computation | `ring_slot/src/lib.rs:323-326` |
| `BytesSlot`'s delegating trait impl | `ring_slot/src/lib.rs:387-390` |
| The buffer-level fold | `ring_store/src/lib.rs:138-141` |
| The permanently-`false` container `is_empty` | `ring_store/src/lib.rs:173-177, 187-191` |
| The two disagreeing on a fresh buffer | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_slot_holding_a_default_value_is_still_occupied` | That `TypedSlot`'s answer reads the slot, not the payload |
| `the_inherent_and_trait_emptiness_agree` | The delegation, fully qualified on both sides |
| `the_trait_reports_the_same_cycle_for_both_shapes` | Both computations giving the same three-state answer |
| `ring_store` — the `all_empty` doctest | The fold, over a fresh two-slot buffer |
| `ring_store` — the `is_empty` doctest | That the container question is `false` for a one-slot buffer |
