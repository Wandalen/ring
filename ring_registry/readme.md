# ring_registry

Named ring registry.

Depends on [`ring_handle`](../ring_handle/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Originally scoped for three dependency edges; the implementation needs one. The
registry never inspects what it stores — it hashes a name, holds a value, lends
it back, and drops it — so the record type is opaque and `ring_types` never
appears. → [`docs/integration/001`](docs/integration/001_one_declared_edge_of_three.md).

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
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `register` | Takes ownership, or refuses and hands the ring back |
| `get_mut` | Lends the ring; the registry keeps owning it |
| `remove` | Gives ownership to the caller, freeing the name |
| `contains` / `len` / `is_empty` / `names` | Ask without borrowing mutably |

## The mistake it is built to avoid

`HashMap::insert` returns the value it displaced. Ignore that return, and a
second registration under a live name **silently destroys the ring already
there** — along with every record still unread in it.

Every count is right afterwards. Every lookup succeeds. `len()` is 1. Nothing
returns an error. The only two things that tell the implementations apart are a
returned error and a drop counter, which is why this crate's own acceptance
criteria ask for the counter.

→ [`docs/pitfall/001`](docs/pitfall/001_insert_would_have_replaced_silently.md).

## What the implementation settled

| Question | Answer |
|---|---|
| `Entry` or `insert`? | `Entry` — see above |
| Should a refusal return the ring? | Yes; two costs, both found by a failing build rather than foreseen — a `Debug` bound at `.expect()` sites, and a 448-byte `Result` that needs a scoped `allow` |
| Is there an immutable `get`? | No — `&Split< T >` permits no operation at all, so it would be dead API |
| One record type per registry, or many? | One. Heterogeneity needs a downcast at every retrieval and nothing asks for it |

## Layout

| File | Responsibility |
|------|-----------------|
| `docs/` | Invariants, lifecycle, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | `Registry` and `RegistryError` |
| `tests/registry_test.rs` | 14 tests — one per acceptance clause, plus the drop-counter cases |
| `tests/manual/readme.md` | Manual plan and dated run record |
