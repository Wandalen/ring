# ring_registry

Named ring registry.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

The registry never inspects what it stores. It hashes a name, holds a value,
lends it back, and drops it, so the record type is opaque.

## What it does

A name-to-ring map that **owns** the rings in it. One name, one ring, and a name
that is taken cannot be taken again until it is given up.

```rust
let mut registry = Registry::new();
registry.register( "events", Split::new( ring ) ).unwrap();

assert!( registry.get_mut( "events" ).is_some() );
assert!( registry.get_mut( "telemetry" ).is_none() );
```

| Operation | Effect |
|---|---|
| `register` | Takes ownership, or refuses and hands the ring back |
| `get_mut` | Lends the ring; the registry keeps owning it |
| `remove` | Gives ownership to the caller, freeing the name |
| `contains` / `len` / `is_empty` / `names` | Ask without borrowing mutably |

## The mistake it is built to avoid

`HashMap::insert` returns the value it displaced. Ignore that return, and a
second registration under a live name **silently destroys the ring already
there**, along with every record still unread in it.

Every count is right afterwards. Every lookup succeeds. `len()` is 1. Nothing
returns an error. The only two things that tell the implementations apart are a
returned error and a drop counter, which is why the tests count drops.

## Decisions

- [The registry stays minimal, and each extension waits for a consumer that needs it](docs/decisions/001_the_registry_stays_minimal_until_a_consumer_asks.md)

## Known limitations

- `Registry::remove` has no `#[must_use]`, so `registry.remove("events");`
  drops the ring and every unread record in it without a warning.

## Run it

```sh
cargo nextest run -p ring_registry --all-features
cargo test --doc -p ring_registry --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | `Registry` and `RegistryError` |
| `tests/registry_test.rs` | Tests across the public API, including the drop-counter cases |
| `tests/manual/readme.md` | Manual plan and dated run record |
