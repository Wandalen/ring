# Decision: The Loom Seam Runs Through a Crate This Manifest Never Names

- **Status**: Open
- **Deciders**: the ring family
- **Turns on**: Whether a build-configuration dependency that is invisible in the manifest should be made visible

### Context

The `exhaustive` test module is compiled only under `RUSTFLAGS="--cfg loom"`.
The manifest explains what makes that work:

> The seam that makes it work is `ring_atomic`'s — it swaps `AtomicSeq` for an
> instrumented one under the same cfg, so this crate's own code is what gets
> explored rather than a re-implementation of it.

That is accurate and `ring_atomic` is not a dependency of this crate.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'path deps:                '; grep -c 'path = "\.\./ring_' Cargo.toml
printf 'named in the manifest:    '; grep -oE 'ring_[a-z]+ = ' Cargo.toml | tr -d ' =' | tr '\n' ' '; echo
printf 'ring_atomic as a dep:     '; grep -cE '^ring_atomic = ' Cargo.toml
printf 'ring_atomic in src:       '; grep -c 'ring_atomic' src/lib.rs
printf 'reaches via ring_cursor:  '; grep -c 'ring_atomic' ../ring_cursor/Cargo.toml
```

Live output:

```
path deps:                5
named in the manifest:    ring_store ring_config ring_cursor ring_slot ring_types 
ring_atomic as a dep:     0
ring_atomic in src:       0
reaches via ring_cursor:  1
```

`ring_atomic` arrives transitively, through `ring_cursor`. This crate imports
`CursorPair`, `SeqCell` and `GATING` from `ring_cursor` and never names
`AtomicSeq` at all.

### Readings

**1 — Correct as written; transitivity is how Cargo works.** Adding
`ring_atomic` to `[dependencies]` would declare a dependency this crate's code
does not use, which is its own defect. The comment describes the mechanism
truthfully and the mechanism happens to be one level down.

**2 — The cfg is the problem, not the dependency.** `--cfg loom` is a flag with
no manifest entry anywhere. It changes the meaning of a type two crates away,
and nothing in this crate's build graph records that. A reader who removes
`ring_cursor`'s `ring_atomic` dependency breaks this crate's loom mode and
nothing fails, because loom mode is not in the default test path
(→ `ring_mpsc`'s `invariant/002`, MP24, for the same shape in the sibling).

**3 — It should be a dev-dependency under the same target.** `[target.'cfg(loom)'.dev-dependencies]`
already exists in this manifest and already names `loom`. Naming `ring_atomic`
there too would document the seam at the site where it applies, without
declaring a dependency the ordinary build carries.

### Decision

Open. Reading 3 is cheap and reversible, and applying it here would *converge*
the two composed cores rather than split them: `ring_mpsc` already names
`ring_atomic` in `[dependencies]` because its stamp protocol uses `AtomicSeq` in
ordinary code (→ SP17). This crate cannot copy that entry — it uses no such type
— so the `cfg(loom)` block is the only place the seam could be declared without
inventing a dependency the ordinary build does not have.

It is filed rather than applied because the wording is shared with the sibling's
manifest, and a justification edited in one of two copies leaves the next reader
two forms and no rule.

### Consequences

The loom mode's correctness depends on a sibling crate's dependency list that
nothing in this crate references or checks.

### Sources

| File | Relationship |
|------|-----------------|
| `Cargo.toml` | Carries the comment and the `cfg(loom)` dev-dependency block |
| `../../../ring_cursor/Cargo.toml` | Where `ring_atomic` actually enters this crate's graph |
| `../../../ring_atomic/src/lib.rs` | The instrumented `AtomicSeq` the comment describes |

### SP16 — The `cfg(loom)` Flag Has No Manifest Entry Anywhere

`[target.'cfg(loom)'.dev-dependencies]` names `loom` itself, but nothing declares
that `ring_atomic` must swap its `AtomicSeq` under the same cfg for this crate's
`exhaustive` module to be meaningful.

**A build mode with no declaration cannot be broken loudly.** Removing
`ring_cursor`'s `ring_atomic` dependency, or changing that crate's cfg gate,
would leave this crate's loom tests compiling and exploring the wrong thing.

### SP17 — The Same Argument Is in the Sibling's Manifest, Where It Is Backed by a Declaration

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_spsc ring_mpsc; do
  printf '%-11s ring_atomic in [dependencies]: %s   the same argument in a comment: %s\n' "$c" \
    "$( grep -cE '^ring_atomic = ' $c/Cargo.toml )" \
    "$( grep -c 'swaps `AtomicSeq`' $c/Cargo.toml )"
done
```

Live output:

```
ring_spsc   ring_atomic in [dependencies]: 0   the same argument in a comment: 1
ring_mpsc   ring_atomic in [dependencies]: 1   the same argument in a comment: 1
```

Both manifests carry the sentence; only one carries the dependency it argues
from. `ring_mpsc` names `ring_atomic` directly because its stamp protocol uses
`AtomicSeq` in ordinary code, so the loom comment there describes a crate the
manifest already declares. Here the identical sentence describes a crate the
manifest never mentions.

**The comment is portable and the declaration behind it is not**, which is the
shape that makes a copied justification quietly weaker in its second home. It
reads as a family convention documented twice; it is one crate documenting a
dependency and another documenting a hope.
