# integration

This crate's module documentation makes a wiring claim — "the ring's publish and
drain call those" — and the whole of this definition is what happens when that
claim is checked. One manifest in 33 declares an edge to this crate, under
`[dev-dependencies]`. The crate that assembles a ring declares no edge at all and
writes slots with `TypedSlot::set` and reads them with `TypedSlot::take`.

The two instances split the finding by kind. The first is the graph: who declares
what, which crates delegate to this one in prose while unable to call it, and how
the family's other unconsumed crates differ. The second is the mechanism: what
`ring_core` calls instead, why the read half of the one path could not serve it
even with the edge declared, and where the family's genericity over slot shape is
actually lost.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_declarer_and_it_is_a_dev_dependency.md) | One Declarer, and It Is a Dev-Dependency | Every edge in and out, and two prose delegations that cannot be acted on |
| [002](002_the_ring_writes_slots_without_this_crate.md) | The Ring Writes Slots Without This Crate | `ring_core`'s real path, the missing take, and the lost second shape |

## The Graph

Out: `ring_types` and `ring_slot` as dependencies, `ring_store` for tests. In:
`ring_tls/Cargo.toml:15`, a dev-dependency, and nothing else. Three code
references exist outside the crate and exactly one is an import, in
`ring_tls/tests/tls_test.rs`; the other two are module-documentation sentences in
`ring_batch` and `ring_tls` assigning slot contents to this crate's care — from
one crate that declares no edge at all and one that declares a test-only edge.

Eight of the 33 crates have no production consumer, so an unconsumed crate is not
unusual here. What makes this one different is that the others are tools, and
being reached only from tests is what a tool is for.

## The Mechanism

`ring_core` writes with `reserved.set( record )` and reads four times with
`TypedSlot::take`. The write is a wiring difference — `publish_into` would do the
same thing. The read is not: `take` moves the payload out, and every read
operation here borrows. A probe composing `drain_from` with `recycle` to build a
take fails to compile with `E0502`, so adopting the read half is a design change
rather than a manifest line.

And `ring_core` fixes `TypedSlot< T >` eight times while never mentioning
`BytesSlot`, though `ring_spsc::Ring< S >`, `ring_mpsc::Ring< S >` and
`ring_store::Buffer< S >` are all generic in the shape. So this crate's own
"identical path" claim is unrealized twice over, and the two failures are independent.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every edge into this crate, and every code reference outside it --'
command grep -rn 'ring_event' --include=Cargo.toml . | command grep -v '^ring_event/'
command grep -rn 'ring_event' --include=*.rs . | command grep -v '^ring_event/'
echo '  -- what ring_core declares, writes with, and reads with --'
command grep -n 'ring_' ring_core/Cargo.toml | command grep -v '^2:'
command grep -rn '\.set( \|TypedSlot::take' --include=*.rs ring_core/src
echo '  -- the rings genericity, and where it stops --'
command grep -n 'pub struct Ring<' ring_spsc/src/lib.rs ring_mpsc/src/lib.rs
command grep -c 'TypedSlot< T >' ring_core/src/lib.rs || true
command grep -c 'BytesSlot' ring_core/src/lib.rs || true
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV17 | `ring_event` | n/a — unadopted | Exactly one manifest in 33 names this crate — `ring_tls/Cargo.toml:15`, under `[dev-dependencies]` — and exactly one import exists outside it, in `ring_tls`'s test suite, so removing the crate from the workspace breaks nothing but that suite; eight of 33 crates have no production consumer, but the other seven are tools for which test-only reach is the point, while this one's module documentation says it "is what makes the path literally one path" and that "the ring's publish and drain call those", making it a load-bearing component of a design that nothing yet loads, with nothing recording the difference |
| EV18 | `ring_event` | n/a — doc gap | `ring_batch/src/lib.rs:29` and `ring_tls/src/lib.rs:33` both delegate slot contents to this crate in nearly identical words — what goes in them "is `ring_store`'s and `ring_event`'s business" — while `ring_tls` declares the edge only as a dev-dependency and `ring_batch` declares no edge at all, so the sentence names a boundary its own code cannot cross; as architecture both statements are true and worth making, as navigation they send a reader after a call that cannot exist without a manifest change neither crate has made, and naming the delegation as intended rather than current costs four words |
| EV19 | `ring_event` | **wrong doc** | The module doc states as fact that `publish_into` and `drain_from` are what "the ring's publish and drain call", while `ring_core` declares `ring_slot` and not this crate, writes with `reserved.set( record )` at `:369` and reads four times with `TypedSlot::take` — and the read half is not false for want of a manifest line, since `take` moves the payload out and every read operation here borrows (`peek` and `drain_from` both return `Option< …Out< '_ > >`, both impls' `Out` a reference, `recycle` returning nothing), with a probe composing the two into a take failing at `E0502`; adopting the read half is therefore a design change — a fourth function returning an owned value, or a `Peek::Out` that may own, which `:25-32` rules out for the byte shape — and nothing records whether the one path was ever meant to cover reads at all |
| EV20 | `ring_core` | n/a — doc gap | This crate's module documentation states that `TypedSlot<T>` and `BytesSlot` "both round-trip through the **identical** claim/publish/drain path" and the rings can carry either — `ring_spsc::Ring< S >`, `ring_mpsc::Ring< S >` and `ring_store::Buffer< S >` are all generic in the shape — but `ring_core` fixes `TypedSlot< T >` eight times across its ring enum, ends, producer and consumer while mentioning `BytesSlot` zero times, with no parameter, feature or constructor to change it, so every assembled ring is a typed-slot ring and the feature is unrealized twice over independently: even full adoption of `publish_into` and `drain_from` would exercise them at one shape only, and the narrowing is explained nowhere and flagged as temporary nowhere |
