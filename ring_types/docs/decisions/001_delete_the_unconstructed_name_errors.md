# Delete `RingError::NameTaken` and `RingError::NameUnknown`, which nothing constructs

Status: Proposed

## Context

`ring_types::RingError` declares `NameTaken` and `NameUnknown` for a named-ring registry. No code in the workspace
constructs either one. Only `ring_types`' own tests name them, so they appear in classification and `Display` tests
while nothing produces them.

The registry chose its own error. `RingError` is `Copy` and allocation-free because an error on the tick path must not
allocate. `ring_registry::RegistryError::NameTaken { name: String }` carries the colliding name, which `RingError`
cannot, and registration happens once at setup, off the tick path. `ring_registry` does not depend on `ring_types` at
all.

`NameUnknown` has no use under either error type. `ring_registry::Registry::get_mut` and `Registry::remove` return
`Option`, so an absent name is a `None`, not a failure.

`ring_factory::BuildError::NameTaken` is a third spelling of the same failure. `ring_factory::Factory::build_named`
maps `RegistryError::NameTaken` to it and drops the name, because the caller passed the name in and still has it.

A consumer that matches on `RingError::NameTaken` today writes an arm that never fires. `#[non_exhaustive]` forces a
wildcard arm beside it, so nothing warns.

## Decision

Delete both variants, together with their arms in `RingError::is_configuration`, `RingError::is_transient` and
`Display`, and the test entries that name them.

`#[non_exhaustive]` does not make this free. It protects callers against added variants only, and removing a variant
breaks any code that names it. `ring_types` is `publish = true`, so for an outside user who names either variant the
deletion is a breaking release. Inside the workspace the deletion is cheap, because no code outside `ring_types`' own
tests names them.

## Alternatives considered

- **Keep both and document them as reserved.** One clause per variant would stop a reader treating them as live, but
  they would stay in the classifier, the `Display` match and every reader's picture of what a ring can report, on
  behalf of a registry that chose another type.
- **Make them reachable.** The registry would have to return `RingError`, which means dropping the name from the
  message or dropping `Copy` from `RingError`. Dropping `Copy` puts an allocating type on the tick path and breaks
  `ring_bench::RunError`, which wraps `RingError` and derives `Copy`. This is the trade `RegistryError` exists to
  avoid.

## Consequences

- Keeping `RingError` `Copy` and allocation-free costs the family its single error type. `RegistryError` is a second
  public error type, `ring_factory` converts it to `BuildError` by hand, and the name variants in `RingError` were
  never used. Deleting them makes that state plain instead of leaving two variants that suggest otherwise.
- `ring_factory` re-exports `Registry` but not `RegistryError`, so a caller limited to the export contract who calls
  `Registry::register` directly cannot name the error it gets back. Re-exporting `RegistryError` would fix that and
  widen the export contract to two error types. This question is open and independent of the deletion.
- Revisit before acting if code outside `ring_types` starts naming either variant, or if a registry lookup starts
  returning an error instead of `Option`.
