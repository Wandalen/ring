# lifecycle

The record has the shortest life a type can have. It is built by one fallible
constructor, optionally replaced by setters that consume and return, copied
bitwise across every boundary it crosses, read once, and dropped at the end of the
expression that built a ring. There is no `&mut self` anywhere on it, no `Drop`,
no interior mutability, and — because it is `Copy` — no single instance to follow
in the first place.

What makes that worth two instances is where it ends. Five fields go in; two come
out the far side as queryable state, one survives as a coarser tag, and two leave
no trace in any type in the family. A ring cannot be asked what configured it, and
the derived equality that would let a caller check has nothing to compare against.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_built_once_copied_never_mutated.md) | Built Once, Copied, Never Mutated | The consuming setters, the single stored instance, and the narrowing at each frame |
| [002](002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md) | Nothing Can Ask a Built Ring What It Was Configured With | The three surviving readings, the two that vanish, and the unused equality |

## A Value, Not an Object

`RingConfig` derives `Copy` and exposes no `&mut self` method — the count is
zero across the file. The four setters take `mut self` and hand back a new value,
so what looks like mutation is replacement, and `let b = a;` leaves both alive and
independent.

Every property this corpus records about the crate is therefore a property of the
type rather than of an instance. Nothing can be observed changing, so nothing
needs to be invalidated, synchronised, or re-checked at a boundary. The whole
class of lifecycle problems the family's cursor crates spend their documentation
on does not arise here.

## Five Fields In, Two and a Tag Out

Exactly one struct in thirty-three crates keeps a `RingConfig` in a field:
`ring_bench::Workload`. Every other consumer reads and drops. `ring_core::Ring::new`
stores `overflow : config.overflow()` and branches once on the producer count;
both backends' `with_config` are `Self::new( config.capacity() )`, one field of
five.

So a built ring answers three questions — `capacity()`, `overflow()`, `backend()`
— and `backend()` is what `producers` became after being reduced to a bit.
`wait` and `batch` are recoverable from nothing: no type in the family has an
accessor for either except the record itself.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- occurrences of &mut self on the type --'
command grep -c '&mut self' ring_config/src/lib.rs || true
echo '  -- the one struct in the family that stores a RingConfig --'
command grep -rn '^  [a-z_]* : RingConfig,' --include=*.rs */src 
echo '  -- what each downstream frame keeps out of the five --'
command grep -m1 -F '    Ok( Self { storage, overflow : config.overflow() } )' ring_core/src/lib.rs
command grep -m1 -F '    Self::new( config.capacity() )' ring_mpsc/src/lib.rs
echo '  -- and what a built ring can still be asked --'
command grep -n 'pub const fn backend\|pub fn capacity( &self ) -> Capacity\|pub const fn overflow' ring_core/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC29 | `ring_config` | n/a — observation | The type has zero `&mut self` methods, no `Drop` and no interior mutability, and all four setters take `mut self` and return `Self`, so the `mut` binds a moved-in copy rather than exposing mutation — combined with `Copy`, there is no instance to trace at all, which is why every property recorded about this crate is a property of the type and no lifecycle hazard in the family's cursor crates has an analogue here |
| RC30 | `ring_config` | n/a — observation | Exactly one struct in thirty-three crates has a field of type `RingConfig` — `ring_bench::Workload` at `:173` — and every other consumer reads and drops: `ring_core::Ring::new` keeps `overflow` and one branch on the producer count, and both backends' `with_config` are `Self::new( config.capacity() )`, so the funnel runs five fields to two-plus-a-bit to one, and `wait`, `producers` and `batch` do not survive the first frame that receives them |
| RC31 | `ring_config` | n/a — diagnostics | A built ring answers `capacity()`, `overflow()` and `backend()`, where `backend()` is a three-variant tag rather than the configured count — so `Backend::Mpsc` is compatible with a configured `2` and a configured `64` alike — while `wait` and `batch` are recoverable from nothing: a census of every `src/` file in the family finds a `wait( &self )` accessor on `RingConfig` alone, which means a ring spinning where it should have been non-blocking cannot be diagnosed from the running system at all |
| RC32 | `ring_config` | n/a — unadopted | `RingConfig` derives `PartialEq` and `Eq` and no two configurations are ever compared outside this crate, in `src/` or `tests/` — which follows from RC31 rather than standing apart from it, since the only stored configuration in the workspace is the harness's own copy and there is nowhere else a second one could come from, leaving the derive a within-crate facility that `setters_commute` needs to state its property and nothing else can use |
