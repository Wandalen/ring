# Workaround: The Expected Output Names Crates This One Never Sees

### Scope

- **Purpose**: Record the second-order cost W1's compensation actually incurred — the pinned diagnostics reach two crates past this one's dependency list, into private types.
- **Responsibility**: Show what is pinned, how far it reaches, and what that couples together.
- **In Scope**: The contents of `tests/ui/*.stderr`; the crate names and private types they quote; the dependency edges those names do and do not correspond to.
- **Out of Scope**: Why a `trybuild` suite exists at all (→ [`001`](001_asserting_an_absence_needs_a_second_compiler_run.md)); the maintainer-facing trap (→ [`pitfall/002`](../pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md)).

### The Constraint, Restated

W1 says Rust cannot assert an absence, so the crate pins compiler output
instead. **The compiler's output is not a stable, scoped surface** — a
`Sync` error walks the entire type-composition chain and names every private
type along the way, wherever it lives.

That is not a defect in `trybuild` and not a defect in the compiler. It is what
a "required because it appears within the type" note is *for*: telling a reader
exactly where the offending field came from. Pinning it as an expectation
converts a diagnostic aid into a contract.

### What Is Actually Pinned

Seven `.stderr` files. Six quote only `ring_handle::` names. One —
`producer_shared_across_threads.stderr` — quotes a five-link chain:

```
PhantomData<Cell<()>>
  → ring_spsc::Producer<'_, ring_slot::TypedSlot<u32>>
    → ring_core::ProducerInner<'_, u32>
      → ring_core::Producer<'_, u32>
        → ring_handle::Producer<'_, u32>
```

Of those, `ring_core::ProducerInner` is a **private** `enum`, and `ring_spsc`
and `ring_slot` are crates absent from this crate's manifest entirely — they
arrive transitively through `ring_core`.

### The Cost

| Cost | Detail |
|------|--------|
| A pinned private type | `ring_core::ProducerInner` cannot be renamed without failing a test in a different crate |
| Two undeclared coupling edges | `ring_spsc` and `ring_slot` names are load-bearing here and appear in no manifest |
| Coupling to a backend choice | The chain runs through `ring_spsc` because that is which variant `ProducerInner` resolves to for this configuration |
| Blast radius invisible from either end | Nothing in `ring_spsc` says "renaming `Producer` breaks `ring_handle`", and nothing in `ring_handle`'s manifest says it reads `ring_spsc` |

**The fourth row is the workaround's real price.** W1's cost column predicted
brittleness against *toolchain upgrades*. The brittleness that actually
materialized is against **sibling refactors**, which happen far more often and
whose authors have no signal that this file exists.

### Deletion Condition

Same as W1's — a first-class absence assertion retires the whole suite. Short of
that, the reach is reducible but not removable: a narrower probe that produces a
one-link error would pin less, at the cost of testing something further from
what a caller would actually write.

**Not taken.** The test asserts that a `&Producer` cannot cross a thread
boundary, and the realistic way to write that is the way it is written. Trading
a real scenario for a shallower diagnostic would weaken the assertion to buy
tidiness.

### Workarounds

| File | Relationship |
|------|--------------|
| [001_asserting_an_absence_needs_a_second_compiler_run.md](001_asserting_an_absence_needs_a_second_compiler_run.md) | The constraint whose compensation this cost belongs to |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md](../pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md) | The same fact, written for the maintainer who trips on it |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_and_the_backends_beneath.md](../integration/001_one_dependency_and_the_backends_beneath.md) | Why the chain runs through one backend and not another |

### Sources

| File | Relationship |
|------|--------------|
| [`tests/ui/producer_shared_across_threads.stderr`](../../tests/ui/producer_shared_across_threads.stderr) | The pinned chain |
| [`ring_core/Cargo.toml`](../../../ring_core/Cargo.toml) | Where `ring_spsc` and `ring_slot` actually enter |

### Tests

| Test | Relationship |
|------|--------------|
| `both_handles_are_send` | The positive counterpart — asserts the property the pinned file asserts the boundary of |

### HD47 — One Expected-Output File Names Two Crates With No Edge to This One

The pinned text reaches past the manifest:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- crate names quoted in the pinned expectations --'
command grep -hoE '\bring_[a-z_]+::' ring_handle/tests/ui/*.stderr | sort | uniq -c
echo '  -- crates this one declares --'
command grep -E '^ring_[a-z_]+ =' ring_handle/Cargo.toml | sed 's/ =.*//'
echo '  -- where the two extras actually come from --'
command grep -E '^ring_(spsc|slot) =' ring_core/Cargo.toml
```

Live output:

```
  -- crate names quoted in the pinned expectations --
      2 ring_core::
     13 ring_handle::
      1 ring_slot::
      1 ring_spsc::
  -- crates this one declares --
ring_core
ring_config
ring_types
  -- where the two extras actually come from --
ring_slot = { path = "../ring_slot" }
ring_spsc = { path = "../ring_spsc" }
```

`ring_spsc` and `ring_slot` are quoted once each and declared nowhere here. They
reach this file through `ring_core`, whose private `ProducerInner` enum holds a
`ring_spsc::Producer` variant.

**Six of the seven pinned files stay inside this crate's own namespace and the
seventh reaches four links out.** The difference is not a choice anyone made —
it is what the compiler prints for a `Sync` violation buried in a composed type,
and it is invisible until someone two crates away renames something and a test
here goes red.

### HD48 — A Private Type Is Load-Bearing in Another Crate's Fixtures

`ProducerInner` is not public and is pinned anyway:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- how ProducerInner is declared --'
command grep -E '^(pub )?enum ProducerInner' ring_core/src/lib.rs
echo '  -- where its exact spelling is checked in, outside ring_core --'
# docs/ is excluded: this instance names the type too, and a scan that
# included it would count its own prose as evidence
command grep -r 'ProducerInner' ring_handle/src ring_handle/tests \
  | sed 's|ring_handle/||'
```

Live output:

```
  -- how ProducerInner is declared --
enum ProducerInner< 'a, T >
  -- where its exact spelling is checked in, outside ring_core --
tests/manual/readme.md:`ring_core::ProducerInner` → `ring_handle::Producer`). **A reader who asks "why
tests/ui/producer_shared_across_threads.stderr:note: required because it appears within the type `ring_core::ProducerInner<'_, u32>`
tests/ui/producer_shared_across_threads.stderr:   | enum ProducerInner< 'a, T >
```

`enum ProducerInner` — no `pub`. It is an implementation detail of `ring_core`,
invisible to every consumer, and its exact spelling is checked into
`ring_handle`'s test fixtures and quoted again in the manual test plan.

**Renaming a private type is normally the safest refactor there is.** Here it
fails a test in a crate that cannot name the type, cannot import it, and does
not declare the crate it lives in. The failure will be correct, loud, and
completely mystifying — and `TRYBUILD=overwrite` will make it go away, which is
the outcome to worry about.

**The corpus already reads this same chain as an asset**, and that is the part
worth stating plainly rather than resolving.
[`non_functional_requirement/002`](../non_functional_requirement/002_send_without_sync.md)
argues that pinning `Cell<()>` → `ring_spsc::Producer` →
`ring_core::ProducerInner` → `ring_handle::Producer` is exactly what makes the
`Sync` opt-out legible: "a reader who asks 'why can't I share this' gets the
answer and the owning crate from the test output." That is true. It is also the
mechanism described above.

One line of pinned text is simultaneously the crate's best explanation of an
inherited property and its most fragile coupling. Neither instance is wrong;
the corpus simply had the benefit written down and not the price.

**Disposition:** declined — the finding's own conclusion is that neither side
of the trade-off is wrong; recording the price this file now does is the
remedy, and it already exists as this doc instance. There is no source-level
fix available inside `ring_handle`: the coupling is to a private type in
`ring_core`, and narrowing the pinned chain to a shallower diagnostic is
explicitly rejected two sections above ("Trading a real scenario for a
shallower diagnostic would weaken the assertion to buy tidiness").

