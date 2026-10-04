# The registry stays minimal, and each extension waits for a consumer that needs it

Status: Accepted

## Context

`ring_registry::Registry<T>` maps names to `ring_handle::Split<T>` values and owns them. It has one consumer,
`ring_factory::Factory::build_named`, which registers each ring it builds and re-exports `Registry` so its own
callers never name `ring_registry`.

Several extensions each have a real argument for them: rings of different record types in one map, validated names,
a stable order from `names()`, a way to visit every ring, and an immutable `get`. Each also has a cost in failure
modes or in public API that is hard to take back. The registry cannot tell from its own code which of them a caller
will need.

## Decision

The registry stays as it is, and each extension waits for a named consumer that needs it.

- One record type per registry. `Registry<T>` is generic over a single `T`.
- Names are not validated. `register` takes `impl Into<String>`, and the test `unusual_names_are_ordinary_names`
  pins the empty string, spaces and a newline as valid names.
- `Registry::names` yields in `HashMap` order. A caller that needs an order sorts at the call site.
- There is no `values_mut`, `iter_mut`, `drain` or `retain`.
- There is no immutable `get`. `ring_handle::Split` has `new` and `ends(&mut self)` and nothing else, so a
  `&Split<T>` permits no operation. `Registry::contains` is the immutable question that has an answer.

## Alternatives considered

- **Rings of several record types in one map.** This needs `Box<dyn Any>` and a downcast at every retrieval, so
  `get_mut` gains a second way to fail (wrong type, besides absent name). `ring_factory` builds one ring at a time,
  and two registries cost one extra binding.
- **Validated names.** This adds a second failure mode to `register`, a rule to document, and a ruling on names that
  are odd but harmless. `ring_factory` passes names through from its caller and never formats one into output.
- **An ordered map for a stable `names()`.** This question is closed. The consumer that needed a stable listing,
  `ring_factory`'s test, collects `names()` into a `Vec<&str>` and calls `sort_unstable`, which is the cheaper answer.
  The `HashMap` is not the faster choice today. Measured on `get_mut`, a `BTreeMap` is faster below about a dozen
  names, so the `HashMap` buys headroom for large registries, not speed for the small ones that exist.
- **An accessor that visits every ring.** No caller visits rings. The call sites of `names()` read a snapshot of the
  names and never look a ring up from it.
- **An immutable `get`.** It would compile, return `Some`, and allow nothing.

## Consequences

- A program with rings of two record types holds two registries.
- Visiting every ring compiles only by collecting `names()` into owned `String`s and calling `get_mut` on each, since
  `names()` and `get_mut` do not compose under the borrow checker. That costs a `String` and a second lookup per ring,
  measured at about thirty times a `values_mut()` sweep over the same `HashMap`.
- `register` hands the refused `Split` back inside its error, as `Registry::register` documents. Every `Result` it
  returns is 768 bytes wide (448 when this record was written; it follows the cursor padding inside `Split<T>`), on
  the `Ok` path too, and `.expect()` or `.unwrap()` on it needs `T: Debug`, because the
  error contains `Split<T>`. The one consumer, `ring_factory::Factory::build_named`, drops the handed-back ring on
  purpose, so nothing exercises the capability yet.
- `RegistryError` is not `#[non_exhaustive]`. Its doc says an enum "leaves room for that to change without a breaking
  signature", which holds for signatures only. A second variant breaks the exhaustive `match registry.register(..)` in
  `ring_factory::Factory::build_named`. The fix is the attribute plus one `_` arm in `ring_factory`, and it is
  cheapest before a second consumer writes its own `match`.
- Revisit mixed record types when a consumer holds rings of two record types and wants them in one map.
- Revisit name validation when a consumer formats registry names into output where an empty or newline name breaks
  something. `unusual_names_are_ordinary_names` is then the test to change on purpose.
- Revisit the visit accessor when a consumer flushes, drains or reports on every ring.
- Revisit the immutable `get` when `ring_handle::Split` gains a `&self` method.

See [`src/lib.rs`](../../src/lib.rs) and the consumer in [`ring_factory`](../../../ring_factory/readme.md).
