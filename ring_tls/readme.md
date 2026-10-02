# ring_tls

Per-thread, bump-allocated, zero-lock append log.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

Every thread owns a `TlsBuffer<T>` and appends to it with no atomics and no
mutexes. `TlsBuffer::flush_into` then moves all `N` accumulated items into the
ring as one contiguous claim, so they cost one `fetch_add` between them.
`TlsBuffer<T>` is a `Vec<T>` reserved once to its own refusal bound, which is
what makes the accumulation free. The crate does not write to the ring and
does not decide when to flush. A full buffer refuses the push, and the policy
that reacts is `ring_flush`'s.

[`tests/tls_test.rs`](tests/tls_test.rs) asserts the zero and the one through
`ring_atomic::CountingSeq`, and a reader checks the push path by hand against
[`tests/manual/readme.md`](tests/manual/readme.md).

## Decisions

- [A refused `TlsBuffer::push` drops the item, and handing it back waits for the first non-`Copy` payload](docs/decisions/001_a_refused_push_drops_the_item.md)

## Run it

```sh
cargo nextest run -p ring_tls --all-features
cargo test --doc -p ring_tls --all-features
```

| File | Responsibility |
|------|-----------------|
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
