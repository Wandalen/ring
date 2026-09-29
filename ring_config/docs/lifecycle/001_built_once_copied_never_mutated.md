# Lifecycle: Built Once, Copied, Never Mutated

### Scope

**Purpose:** Record the record's life from construction to drop — how it is
transformed, how it travels, how long it lives, and where each field stops.

**Responsibility:** The consuming-setter form, the `Copy` hand-off, the single
struct that stores one, and the narrowing at each downstream frame.

**In Scope:** `ring_config/src/lib.rs:41`, `:89`, `:104`, `:122`, `:140`;
`ring_bench/src/lib.rs:208`; `ring_core/src/lib.rs:163`, `:174`,
`:180`; `ring_mpsc/src/lib.rs:408-411`;
`ring_spsc/src/lib.rs:323-326`.

**Out of Scope:** What can be recovered from a ring afterwards is
[`lifecycle/002`](002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md).
What each hand-off costs is
[`data_structure/002`](../data_structure/002_four_times_a_reference_and_nobody_pays_it.md).

---

## From `new` to the Backend

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the record is Copy, so every hand-off is a fresh bitwise copy --'
command grep -m1 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_config/src/lib.rs
echo '  -- every struct field of type RingConfig in thirty-three crates --'
command grep -r '^  [a-z_]* : RingConfig,' --include=*.rs */src
echo '  -- what each frame downstream keeps out of it --'
command grep -m1 -F '    Ok( Self { storage, overflow : config.overflow() } )' ring_core/src/lib.rs
command grep -A 2 'pub fn with_config' ring_mpsc/src/lib.rs ring_spsc/src/lib.rs
echo '  -- and the four setters, each consuming self and returning a new one --'
command grep 'mut self, [a-z_]* : ' ring_config/src/lib.rs
```

Live output:

```
  -- the record is Copy, so every hand-off is a fresh bitwise copy --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
  -- every struct field of type RingConfig in thirty-three crates --
ring_bench/src/lib.rs:  config : RingConfig,
  -- what each frame downstream keeps out of it --
    Ok( Self { storage, overflow : config.overflow() } )
ring_mpsc/src/lib.rs:  pub fn with_config( config : &RingConfig ) -> Self
ring_mpsc/src/lib.rs-  {
ring_mpsc/src/lib.rs-    Self::new( config.capacity() )
--
ring_spsc/src/lib.rs:  pub fn with_config( config : &RingConfig ) -> Self
ring_spsc/src/lib.rs-  {
ring_spsc/src/lib.rs-    Self::new( config.capacity() )
  -- and the four setters, each consuming self and returning a new one --
  pub const fn with_wait( mut self, wait : WaitKind ) -> Self
  pub const fn with_overflow( mut self, overflow : OverflowPolicy ) -> Self
  pub const fn with_producers( mut self, producers : usize ) -> Self
  pub const fn with_batch( mut self, batch : usize ) -> Self
```

---

### RC29 — There Is No Mutation Anywhere, Including in the Setters

All four setters take `mut self` and return `Self`. The `mut` is local: it binds
the moved-in copy so the body can assign into it before handing it back. No
`&mut self` exists on the type, no interior mutability, no `Drop`. The record has
one write per field per constructed value, and after the chain ends nothing can
change it.

It is also `Copy`. So there is no single instance to follow — `let b = a;` leaves
`a` intact and `b` a bitwise duplicate, and every call taking one by value gets
its own. The "lifecycle" of a `RingConfig` is the lifecycle of a value, not of an
object.

**Finding.** That combination makes the type's whole life story two sentences —
built by `new`, optionally replaced four times by setters that consume and
return — and it is the reason every property the corpus records about this crate
is a property of the *type* rather than of any instance. Nothing can be observed
changing, so nothing needs to be synchronised, checked at a boundary, or
invalidated.

It also means there is nothing to look at afterwards. A misconfigured ring cannot
be traced back to the record that configured it, because that record was a value
in a frame that has returned. The one exception is
[`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md)'s
subject: the single struct that keeps one.

---

### RC30 — The Record Narrows at Every Frame, and Three Fields Do Not Survive the First

Exactly one struct in thirty-three crates has a field of type `RingConfig`:
`ring_bench::Workload` at `:208`. Every other consumer receives one, reads what it
wants, and lets it go.

Follow it down. `ring_factory::build` takes it by value and passes it on.
`ring_core::Ring::new` takes `&RingConfig`, branches once on
`config.is_multi_producer()` to pick a backend, and stores exactly one field:
`Ok( Self { storage, overflow : config.overflow() } )`. Then both backends'
`with_config` — `ring_mpsc:410` and `ring_spsc:325`, identical — are
`Self::new( config.capacity() )`, one field of five.

So the funnel is five fields at `new`, two plus a bit at `ring_core`, one at the
backend.

**Finding.** `wait`, `producers` and `batch` do not survive the first frame that
receives them. `producers` gets one bit read out of it — the backend choice — and
is then gone; `wait` and `batch` are not read at all, so their entire life is the
distance from the setter that wrote them to the end of the expression that built
the ring.

This is the same result [`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md)
reaches from the reader census, arrived at from the other direction, and it is
worth having twice because the two views answer different questions. The census
says nobody calls those accessors. The funnel says there is no frame left alive
that could.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md) | What survives the funnel as queryable state |
| [`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md) | The same narrowing seen as a per-field read census |
| [`pattern/001`](../pattern/001_a_consuming_builder_over_a_copy_record.md) | The consuming-setter form the setters use |
| [`data_structure/002`](../data_structure/002_four_times_a_reference_and_nobody_pays_it.md) | What a by-value hand-off costs at each frame |

### Sources

| Fact | Where |
|------|-------|
| `Copy`, no `Drop`, no interior mutability | `ring_config/src/lib.rs:41` |
| All four setters consuming `mut self` | `ring_config/src/lib.rs:89`, `:104`, `:122`, `:140` |
| The one struct field of this type | `ring_bench/src/lib.rs:208` |
| `ring_core` keeping one field and a branch | `ring_core/src/lib.rs:174`, `:180` |
| Both backends keeping only the capacity | `ring_mpsc/src/lib.rs:410`; `ring_spsc/src/lib.rs:325` |

### Tests

| Test | Covers |
|------|--------|
| `the_record_is_copy_and_compares_by_value` | That a hand-off duplicates rather than moves |
| `each_setter_is_independent` | That a setter replaces one field and leaves the rest |
| `every_named_field_is_carried` | The full five the funnel starts with |
