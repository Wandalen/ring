# integration

One edge. The initial assignment named three — `ring_handle`, `ring_core`,
`ring_types` — and the manifest keeps one as a real dependency, `ring_core` for
tests only and `ring_types` not at all, with a comment above each explaining the
demotion. That reduction is the crate's cleanest piece of reasoning, and both
instances here are about what it did not follow through on.

The edge that remains carries a type and no behaviour: `Split< T >` appears six
times in compiled library code, all six in type position, and the crate calls
zero methods on one. The edge that was declined is declined on a premise that
does not hold — the family error already has the variant this crate says would
have to be added — and the private error it chose instead carries a payload that
no external consumer of the family can ever observe.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_declared_edge_of_three.md) | One Declared Edge of Three | The demotion, the error type declined, and how far the replacement reaches |
| [002](002_what_ring_handle_requires_and_this_crate_does_not.md) | What `ring_handle` Requires and This Crate Does Not | The type-only edge, and the upstream fact three documents rest on |

## The Variant the Argument Says Would Have to Be Added

`integration/001` declines `ring_types::RingError` because "adding a variant to
the family vocabulary for one consumer's bookkeeping would oblige every other
consumer to match an arm they can never receive." `RingError` declares
`NameTaken` — "A ring is already registered under this name" — with `NameUnknown`
beside it, and the census finds both constructed zero times outside `ring_types`.
The obligation the argument warns about is the status quo; declining the type did
not avoid it.

The real reason is stronger and is written down one crate away: `RingError` is
`Copy` and allocation-free because an error from the tick path must not allocate,
so it cannot carry a name. A registry error that reports the name it refused is
not a variant that could have been added — it is a different type by necessity.

## A Payload That Stops at the Family Boundary

`RegistryError` exists to carry a `String`; that is its whole difference from the
family error. Five crates are nameable from outside `ring_*` and this is
not one of them, nothing in the family re-exports `RegistryError`, and
`ring_factory` — the one exported crate that handles it — converts it to a `Copy`
`BuildError::NameTaken` at `src/lib.rs:203`, discarding both halves with a
comment giving the reason. Every error an external consumer can observe for a
taken name is therefore payload-free, which is the property this crate declined
the family type in order to escape.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the assignment, the demotions, and what survived --'
command grep -n 'ring_handle\|ring_core\|ring_types' ring_registry/Cargo.toml |
  cut -c1-88 | sed 's/^/    /'
echo '  -- what crosses the surviving edge --'
printf '    Split occurrences in compiled library code: %s\n' \
  "$( command grep -v '^ *//[/!]' ring_registry/src/lib.rs | command grep -c 'Split' || true )"
printf '    method calls on a Split value:              %s\n' \
  "$( command grep -v '^ *//[/!]' ring_registry/src/lib.rs | command grep -c 'Split::\|\.ends(' || true )"
echo '  -- the variant the family already carries for this failure --'
command grep -n -B 1 'NameTaken,\|NameUnknown,' ring_types/src/error.rs | sed 's/^/    /'
echo '  -- and how far the replacement error reaches --'
printf '    ring_registry on the declared export surface: %s\n' \
  "$( command grep -c '^ring_registry$' bench_harness/gate/declared/ring/export_surface.txt || true )"
printf '    pub use of RegistryError anywhere in /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/:  %s\n' \
  "$( command grep -rc 'pub use.*RegistryError' --include=*.rs */ | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG17 | `ring_registry` | **wrong doc** | `integration/001` declines `ring_types::RingError` on the grounds that "adding a variant to the family vocabulary for one consumer's bookkeeping would oblige every other consumer to match an arm they can never receive", and the variant is already there: `RingError` declares `NameTaken`, documented "A ring is already registered under this name" — this failure, named for it — with `NameUnknown` beside it, both constructed **zero** times outside `ring_types`, so the obligation the argument warns about is the status quo and declining the type did not avoid it; the real reason is better and `ring_types/docs/integration/002` already writes it from the other side of the seam — `RingError` is `Copy` and allocation-free because a tick-path error must not allocate, so it *cannot* carry the name, making this a forced divergence rather than a judgement about vocabulary hygiene; the same table also names `RegistryError::DuplicateName`, a variant that does not exist and occurs exactly once in the repository |
| RG18 | `ring_registry` | n/a — unadopted | `RegistryError` exists to carry a `String` and that is its whole difference from the family error, and the payload never leaves the family: five crates are nameable from outside `ring_*` and `ring_registry` is not among them, the census finds zero `pub use` of `RegistryError` anywhere, and `ring_factory` — the one exported crate that handles it — converts it at `src/lib.rs:203` to a `Copy` `BuildError::NameTaken`, discarding both halves with the comment that carrying the name "would also put a `String` in a `Copy` error type for no new information", so every error an external consumer can observe for a taken name is payload-free, which is exactly the property this crate declined the family type to escape; the seam is also not clean, since `ring_factory` re-exports `Registry` itself and an outside caller can therefore call `register` and receive a `Result` whose error type they have no way to name — a separate small defect belonging to `ring_factory`; two rows in the error table, who can name the type and what survives the conversion, turn a private-type decision into one whose reach is stated |
| RG19 | `ring_registry` | n/a — doc gap | `Split< T >` appears six times in the crate's compiled library code — the import, the map's value type, `register`'s parameter, `register`'s error payload, `get_mut`'s return, `remove`'s return — and all six are type positions, with **zero** method calls on a `Split` anywhere: no `new`, no `ends`, nothing, so the readme's "never inspects what it stores" understates it, since a crate that merely does not inspect could still construct or convert, and this one does not touch the type at all; the declared dependency is satisfied entirely by naming the type, and a `Registry< V >` over any owned value would compile with the same body and no `ring_handle` in the manifest, so what the concrete type buys is domain meaning rather than capability — the error payload is a *ring*, the drop is *records*, and the crate's whole argument about width and hand-back is about a specific thing worth not destroying; the sentence is also what makes RG12's missing `T : Send` explicable, since a crate that never calls a `Split` method never meets the bound guarding those methods |
| RG20 | `ring_registry` | n/a — unenforced | "There is no immutable `get`" is stated three times — in `get_mut`'s doc, in the module doc's deliberate-omissions list, and as Closed 3 — and every statement reduces to one premise, that `Split`'s inherent impl has exactly `new` (no receiver) and `ends( &mut self )` so a `&Split< T >` permits nothing; the premise is true and is a fact about a different crate, and the decisions file knows it, naming the revisit trigger as `Split` growing a `&self` accessor — but nothing would notice: this crate's library calls no `Split` method so no upstream signature change breaks a compile here, its tests construct a `Split` and call `ends` which would keep working, no test asserts anything about the shape of `Split`'s surface, and `ring_handle` names `ring_registry` zero times so a maintainer adding the accessor has nothing pointing back; the guard belongs in this crate's tests because this is the crate that would be wrong — a bound `&Split< T >` demonstrating it can do nothing, or one comment naming the assumption, converts a thrice-repeated claim into something checkable without opening another crate |
