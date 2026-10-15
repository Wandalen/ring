# A refused `TlsBuffer::push` drops the item, and handing it back waits for the first non-`Copy` payload

Status: Deferred

## Context

`ring_tls::TlsBuffer::push` takes `item: T` by value and returns `Result<(), RingError>`. When the buffer is full it
returns `RingError::Full`, and the item, already moved into the call, is dropped when the call returns.
`ring_types::RingError::Full` is a unit variant of a `Copy` enum the whole family shares, so the error has nowhere to
put the item.

Refusal is an expected path. `TlsBuffer::is_full` exists so a caller can flush and retry, and a retry needs the item.
The family already has an idiom for handing a refused value back, in `ring_registry::Registry::register`, which
returns `Result<(), (RegistryError, Split<T>)>`.

An earlier `push` doc comment claimed a refused push did not destroy the item. The signature cannot keep that
promise, so the doc was corrected and the signature left alone.

## Decision

Keep the signature. `push` drops a refused item, its `# Errors` section says so, and it tells callers holding a `T`
that owns a resource to check `TlsBuffer::is_full` before calling.

Changing the signature to `Result<(), (RingError, T)>` is deferred. It is a breaking change to a published crate on
the family's export contract, and today it would buy nothing, because every caller stages a `Copy` type.
`ring_testkit::Script::run` stages `u32`, and `ring_bench` stages its `Record` (a `u64`) through
`ring_flush::Flusher::append`.

## Alternatives considered

- **Return the item as `Result<(), (RingError, T)>`.** It matches `Registry::register` and costs one
  `.map_err(|(e, _)| e)` at each current call site. Deferred rather than rejected, for the reasons above.
- **Carry the item inside `RingError::Full`.** `RingError` is `Copy`, allocation-free and not generic, so it cannot
  hold a `T`.

## Consequences

- A `T` that owns a resource, such as a heap allocation, a file or a join handle, loses one per refused push. The
  obvious retry, `if buffer.push(x).is_err() { flush(); buffer.push(x) }`, compiles only for `Copy` types. For any
  other `T` the compiler rejects the use after move, and the caller has no way to get the item back.
- `ring_flush::Flusher<T>` is generic and inherits the drop through `Flusher::append`, whose doc says so.
- Revisit when the first non-`Copy` payload is staged. `TlsBuffer::push` and `Flusher::append` then widen together to
  hand the refused item back.
