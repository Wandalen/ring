# Pattern: One Trait, Two Shapes

### Scope

**Purpose:** Record the shape of the abstraction — a two-method trait, two
implementors, and static dispatch everywhere — and record that the trait is
object-safe, that nothing uses it as an object, and that the payload half of the
abstraction lives in a different crate.

**Responsibility:** The abstraction pattern `ring_slot` establishes and the
family consumes: what the trait carries, how it is dispatched, and where the rest
of it is.

**In Scope:** `ring_slot/src/lib.rs:38-45`; the three generic containers;
`ring_event`'s `Fill` and `Peek`.

**Out of Scope:** The trait's *method choice* — why `is_empty` and `clear` and
nothing else — is [`api/002`](../api/002_a_trait_with_two_methods.md). The
`set`-returns-displaced convention is
[`pattern/002`](002_the_displaced_value_returned.md).

---

## The Whole Abstraction

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F 'pub trait Slot' ring_slot/src/lib.rs
```

Live output:

```
pub trait Slot
{
  /// Whether this slot currently holds nothing.
  fn is_empty( &self ) -> bool;

  /// Return the slot to its empty state.
  ///
  /// **Not a promise to overwrite.** For a shape that owns what it stores
```

No supertraits, no associated types, no generic methods, no default bodies. Two
required methods, one immutable and one mutable, both taking a receiver and
neither taking or returning a payload.

---

### SL37 — The Trait Is Object-Safe and the Family Dispatches Statically Everywhere

Both methods take a plain receiver and neither mentions `Self` in a position that
would forbid it, so `Slot` is usable as a trait object. Measured, release, with
both shapes in one slice:

```
--- Slot as a trait object ---
  &dyn Slot compiles; two shapes in one slice, 2 empty
  a &dyn Slot is 16 bytes; a &BytesSlot< 8 > is 8
```

It compiles, and it costs a doubled pointer plus an indirect call per method.
Nothing in the family pays that:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'dyn Slot\|Box< dyn\|&dyn' ring_*/src/*.rs 2>/dev/null | grep -vE ':[[:space:]]*//' \
  || echo '  none — the trait is never used as an object'
```

Live output:

```
  none — the trait is never used as an object
```

Every consumer is generic over one type parameter instead:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -E '^pub struct (Buffer|Ring)<' ring_store/src/lib.rs ring_mpsc/src/lib.rs ring_spsc/src/lib.rs | sed 's|^ring/||' | sort
```

Live output:

```
ring_store/src/lib.rs:pub struct Buffer< S >
ring_mpsc/src/lib.rs:pub struct Ring< S >
ring_spsc/src/lib.rs:pub struct Ring< S >
```

**Finding.** The pattern is monomorphisation, not polymorphism: a ring is
`Ring< TypedSlot< Record > >` or `Ring< BytesSlot< 16 > >`, chosen at the type
level, with `is_empty` and `clear` inlined to a field comparison and a store
([`algorithm/002`](../algorithm/002_emptiness_three_ways.md) prices both). That
is the right choice for a hot-path ring and it is the reason a slot can be two
bytes of work per operation instead of a virtual call.

What is worth recording is that the object-safe form remains available and
unclaimed. A ring holding *mixed* shapes — some typed slots, some byte slots — is
expressible today with no change to this crate, at a cost the probe measures, and
nothing in the family says whether that is a supported use or an accident of the
trait's shape. The two-method minimalism that makes `Slot` cheap to implement
also makes it object-safe by construction, so the question never had to be
decided and never was.

---

### SL38 — The Trait Carries Lifecycle, Not Payload, So the Abstraction Spans Two Crates

`Slot` can ask whether a slot is empty and can empty it. It cannot put anything
in or take anything out — `set`, `get`, `take`, `write`, and `read` are all
inherent, none declared on the trait
([`api/001`](../api/001_ten_functions_six_const_seven_must_use.md)). Generic code
holding an `S : Slot` can therefore manage a slot's whole lifecycle and never
touch its contents.

The missing half exists, one crate away:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn '^pub trait' ring_slot/src/lib.rs ring_event/src/lib.rs \
  | sed 's|^ring/||' | sort | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_event/src/lib.rs:pub trait Peek
ring_event/src/lib.rs:pub trait Fill< S >
ring_slot/src/lib.rs:pub trait Slot
```

Three traits describing one slot, split across two crates: `Slot` for lifecycle
in the crate that owns the types, `Fill` for the write direction and `Peek` for
the read direction in a crate three tiers up.

**Finding.** The split is defensible and undocumented. `Fill< S >` is
parameterised by the *slot* rather than the payload, so it is the payload types
that implement it — `&[ u8 ]` fills a `BytesSlot`, a record fills a
`TypedSlot`. That impl cannot live in `ring_slot` without `ring_slot` knowing
every payload type in the family, which is exactly the coupling the generic
design exists to avoid. So the direction traits belong upstream of the shapes,
and `ring_event` is where they went.

The cost is discoverability. A reader arriving at `ring_slot` to learn how a slot
is used generically finds a trait that cannot move data, no mention of `Fill` or
`Peek`, and no pointer to the crate that has them — the module comment names the
two shapes and the cost asymmetry between them and stops there. The one sentence
that would fix it ("payload access is generic through `ring_event::Fill` and
`ring_event::Peek`; this trait carries only lifecycle") is absent, and the
family's own layering makes it awkward to add: `ring_slot` is Tier 1 and
`ring_event` is Tier 2, so the reference can only be prose, never a link the
compiler checks.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/002`](002_the_displaced_value_returned.md) | The other convention this crate establishes, and how four crates read it |
| [`api/002`](../api/002_a_trait_with_two_methods.md) | Why these two methods and not more |
| [`api/001`](../api/001_ten_functions_six_const_seven_must_use.md) | The inherent surface the trait deliberately omits |
| [`integration/001`](../integration/001_seven_dependents_and_four_that_stay_generic.md) | The three consumers that stay generic, and the one layer that does not |
| [`lifecycle/001`](../lifecycle/001_a_slot_across_one_publish.md) | `Fill` and `Peek` in use, and the third call they require |

### Sources

| Fact | Where |
|------|-------|
| The trait, entire | `ring_slot/src/lib.rs:38-45` |
| No `dyn` dispatch in the family | `ring_*/src/*.rs` — no occurrence |
| The three generic containers | `ring_store/src/lib.rs:59`, `ring_mpsc/src/lib.rs:308`, `ring_spsc/src/lib.rs:239` |
| The payload traits | `ring_event/src/lib.rs:46, 95` |
| Object safety and the fat-pointer cost | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_drive_through_the_trait_alone` | One generic function over both implementors |
| `the_trait_reports_the_same_cycle_for_both_shapes` | The lifecycle the trait can express |
| `the_inherent_and_trait_emptiness_agree` | The trait method against its inherent twin |
| `ring_store` — `the_same_buffer_type_serves_both_slot_shapes` | Monomorphisation over two shapes at the container level |
| *(to create)* | A `&dyn Slot` holding both shapes, pinning object safety as intentional |
