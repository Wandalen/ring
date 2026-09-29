# Integration: One Declared Edge of Three

### Scope

- **Purpose**: Record why two of the three dependency edges the initial design assigned this crate are not needed, and what that says about where the registry sits.
- **Responsibility**: The edges, their justification, and the consequence for `ring_factory`.
- **In Scope**: `ring_handle`, and the absence of `ring_core` and `ring_types` from `[dependencies]`.
- **Out of Scope**: What the registry does with what it stores (→ [`api/001`](../api/001_the_registry_surface.md)).

### System Description

The registry holds rings; it does not build them and does not look inside them.
That single fact decides its entire dependency list, and it decided it
differently from what the initial design expected — three edges were assigned,
one is needed.

The gap is not an error in the assignment. It is the answer to the design
question the crate exists to settle: whether a registry *builds*, *inspects*, or
merely *holds*.

### Integration Points

The initial design assigned this crate three dependency edges —
`ring_handle`, `ring_core`, `ring_types`. The implementation needs one.

| Edge | Assigned | Needed in `src/` | Where it went |
|---|---|---|---|
| `ring_handle` | Yes | **Yes** — `Split< T >` is the stored value | `[dependencies]` |
| `ring_core` | Yes | No — a `Ring< T >` is only ever seen wrapped in a `Split` | `[dev-dependencies]` |
| `ring_types` | Yes | No — `T` is opaque here | Dropped |
| `ring_config` | No | No | `[dev-dependencies]` — building a ring to register is what a test does |

**`T` being opaque is the whole reason.** The registry never looks inside what it
stores: it hashes a name, holds a value, lends it back, and drops it. Nothing in
that requires knowing the value is a ring, let alone what the ring holds — so
`ring_types`' vocabulary (`Capacity`, `Seq`, `OverflowPolicy`) never appears in a
signature or a body.

This was not clear before the implementation. Three edges is what you assign when
you expect the registry to *build* or *inspect* rings; one edge is what you need
when it only *holds* them, and which of those it does was the design question the
crate answers.

#### `Split< T >`, not `Ring< T >`

The stored type is `ring_handle::Split< T >` rather than `ring_core::Ring< T >`,
and this is what removes the `ring_core` edge. `Split` is the type that owns a
ring and hands out `Ends`; `Ring` on its own requires the caller to call `ends()`
and manage the borrow themselves. Storing `Split` means `get_mut` hands back
something immediately usable.

**It also means the registry's public surface names no `ring_core` type at all**,
so a consumer of `ring_registry` needs `ring_handle` in scope and nothing else.
That matters for the export Contract: `ring_handle` is one of the family's five
exported crates and `ring_core` is not.

#### The consequence for `ring_factory`

`ring_factory` depends on both this crate and `ring_core`, and its job is to
build a ring and optionally register it. So the type it must produce to call
`register` is a `Split< T >` — which it constructs from the `Ring< T >` that
`ring_core::Ring::new` returns.

That composition is `ring_factory`'s to document, and is one of the questions its
own decisions register leaves open. Named here only so the edge
`ring_factory → ring_registry` is not read as implying this crate knows anything
about construction. It does not: it receives an already-built ring.

### Error Handling

One error type crosses the seam, and it goes outward only — the registry defines
[`RegistryError`](../type/001_registry_error.md) itself rather than borrowing the
family's:

| Edge | What crosses | Handled how |
|---|---|---|
| `ring_handle` | `Split< T >` values | No failure shape at all — a `Split` is moved in and moved out |
| — (own) | `RegistryError::DuplicateName` | Returned paired with the rejected `Split< T >`, so a collision costs a name and not a ring |

**Why this crate defines its own error rather than reaching for
`ring_types::RingError`.** `RingError` is `Copy` and allocation-free — an
error from the tick path must not allocate — so it cannot carry a name at
all. `NameTaken` carrying the name is the whole point of this crate's error:
a forced divergence from a different constraint, not a vocabulary judgement.

The trade is that a caller composing a registry with the rest of the family
handles two error types. That is the honest count — there genuinely are two
unrelated failure domains — and the boundary is stated here rather than blurred
by a conversion impl.

### Compatibility Requirements

| # | Obligation | On whom |
|---|---|---|
| G1 | Build the ring before registering it | The caller — this crate constructs nothing |
| G2 | Wrap it in a `Split< T >` | The caller — `ring_handle::Split::new` |
| G3 | Choose `T`; one per registry | The caller (→ [`decisions/readme.md`](../decisions/readme.md) Pending 1) |

---

## The Error That Was Declined, and Where It Can Go

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order. The pattern for this
# file is anchored: the findings below are appended to it and name the same
# variant, so an unanchored search would match its own commentary.
i=ring_registry/docs/integration/001_one_declared_edge_of_three.md
echo '  -- the error row above, and the variant it names --'
command grep -h '^| — (own) |' "$i" | cut -c1-92 | sed 's/^/    /'
echo '  -- the variant the crate actually declares --'
command grep -n 'NameTaken' ring_registry/src/lib.rs | cut -c1-72 | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the family variant this file argues would have to be added --'
command grep -m1 -A4 -F '  /// A ring is already registered under this name.' ring_types/src/error.rs | sed 's/^/    /'
printf '    RingError::NameTaken constructed anywhere outside ring_types: %s\n' \
  "$( command grep -rn 'RingError::NameTaken' --include=*.rs */ | command grep -v 'ring_types/' | wc -l )"
echo '  -- the five crates a consumer outside the family may name --'
tail -5 bench_harness/gate/declared/ring/export_surface.txt | sed 's/^/    /'
echo '  -- and what the one exported consumer does with this error --'
command grep -m1 -A3 -F '      // The name is discarded rather than carried into `BuildError`: the caller' ring_factory/src/lib.rs | sed 's/^/    /'
echo '  -- the reason this file gives for declining RingError --'
command grep -m1 '^all\. .NameTaken. carrying the name' "$i" | cut -c1-92 | sed 's/^/    /'
```

Live output:

```
  -- the error row above, and the variant it names --
    | — (own) | `RegistryError::DuplicateName` | Returned paired with the rejected `Split< T >
  -- the variant the crate actually declares --
      NameTaken
          Self::NameTaken { name } => write!( f, "a ring is already regis
      /// [`RegistryError::NameTaken`] if the name is already live, pair
            Err( ( RegistryError::NameTaken { name }, ring ) )
  -- the family variant this file argues would have to be added --
      /// A ring is already registered under this name.
      NameTaken,
      /// No ring is registered under this name.
      NameUnknown,
      /// A batch of the requested length cannot be served — the ring's whole
    RingError::NameTaken constructed anywhere outside ring_types: 0
  -- the five crates a consumer outside the family may name --
    ring_factory
    ring_handle
    ring_tls
    ring_flush
    ring_types
  -- and what the one exported consumer does with this error --
          // The name is discarded rather than carried into `BuildError`: the caller
          // passed it in and still has it. Carrying it would also put a `String` in
          // a `Copy` error type for no new information.
          Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),
  -- the reason this file gives for declining RingError --
    all. `NameTaken` carrying the name is the whole point of this crate's error:
```

---

### RG17 — The Reason Given for Declining `RingError` Describes an Addition the Family Has Already Made

The argument above is that a shared error is the wrong home for this failure:
"adding a variant to the family vocabulary for one consumer's bookkeeping would
oblige every other consumer to match an arm they can never receive."

The variant is already there. `ring_types::RingError` declares `NameTaken`,
documented "A ring is already registered under this name" — this failure,
named for it — and `NameUnknown` beside it. Whatever cost the sentence is
weighing was paid before this crate was written, and every consumer that matches
`RingError` exhaustively already carries the arm. The census finds the variant
constructed **zero** times outside `ring_types` itself, so the obligation the
argument warns about is the status quo, and declining the type did not avoid it.

The real reason is better than the one given, and `ring_types` writes it down:
`RingError` is `Copy` and allocation-free because an error from the tick path
must not allocate, so it *cannot* carry the name. A registry error that names the
name it refused is therefore not a variant that could have been added — it is a
different type by necessity. That is a forced divergence, not a judgement call
about vocabulary hygiene, and it is a stronger defence of the decision than the
one this file makes.

**Finding.** Recorded as a correct decision defended on a premise that does not
hold. The repair is to swap the reason: `RingError` is `Copy`, `NameTaken`
carries no payload and never could, and the name is the whole point of this
crate's error — three sentences that are all checkable, replacing one about a
hypothetical addition that the family made and left unconstructed. The
counterpart account in
[`ring_types/docs/integration/002`](../../../ring_types/docs/integration/002_the_registry_that_declined_the_shared_error.md)
already reasons this way from the other side of the seam; the two documents
describe one decision and only one of them gives its actual cause.

**Disposition:** applied — `:81-85` now gives the checkable reason: `RingError`
is `Copy` and allocation-free so it cannot carry a name, and `NameTaken`
carrying the name is the whole point of this crate's error — replacing the
"one consumer's bookkeeping" framing the census contradicts. Now prints: `carrying the name is the whole point`

---

### RG18 — The Payload That Justifies the Private Error Cannot Cross the Export Seam

`RegistryError` exists to carry a `String`. That is its whole difference from
the family error, and this file's error table is where the difference is argued.

The export surface says who may see it. Five crates are nameable from outside
`ring_*` — `ring_factory`, `ring_handle`, `ring_tls`, `ring_flush`,
`ring_types` — and `ring_registry` is not among them; this crate's own
contribution is reached *through* `ring_factory`, by design. No crate re-exports `RegistryError`: the
census finds zero `pub use` of it anywhere in the family. And `ring_factory`,
the one exported crate that handles it, converts it away at `src/lib.rs:203`,
discarding both halves of the payload with a comment giving the reason —
"Carrying it would also put a `String` in a `Copy` error type for no new
information." `BuildError` is `Copy`, exactly as `RingError` is.

So the name never leaves. Every error an external consumer of this family can
observe for a taken name is payload-free, which is the property this crate
declined the family type in order to escape. Worse, the seam is not clean:
`ring_factory` re-exports `Registry` itself, so an outside caller can hold one
and call `register` — and gets back a `Result` whose error type they have no way
to name, because the crate that declares it is off the surface.

**Finding.** Recorded as a capability confined to the family that declared it —
the same shape as
[RG14](../decisions/002_three_pending_questions_and_the_one_consumer.md)'s
hand-back contract, discarded by its only consumer, one level further out. This
file is the right place to say it, because it is the file that owns the export
Contract argument and already draws the right conclusion for `ring_handle`. Two
rows in the error table — who can name `RegistryError` (nothing outside the
family) and what survives the conversion (`BuildError::NameTaken`, payload-free)
— turn a private-type decision into one whose reach is stated. The nameability
gap on the re-exported `Registry::register` is a separate, small defect and
belongs to `ring_factory`.

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_registry_surface.md](../api/001_the_registry_surface.md) | The surface these edges support |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_name_one_ring.md](../invariant/001_one_name_one_ring.md) | E3 — `Split< T >` not being `Clone` is what enforces R2 |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The one dependency edge, and the two moved to dev |
| [`ring_handle/src/lib.rs`](../../../ring_handle/src/lib.rs) | `Split< T >` — the stored type |

### Tests

| File | Relationship |
|------|--------------|
| `tests/registry_test.rs` | Its `ring` helper is G1 and G2 performed by a caller — the two steps this crate does not do |
