# Integration: Seven Dependents and Four That Stay Generic

### Scope

**Purpose:** Record who depends on `ring_slot`, which of them are written
against the `Slot` trait rather than a concrete shape, and where the family's
unified handle forecloses the second shape entirely.

**Responsibility:** The inbound edges to `ring_slot` and what each one actually
uses.

**In Scope:** The seven crates declaring a `ring_slot` dependency; the four
`S : Slot` bounds at `ring_store/src/lib.rs:96`,
`ring_spsc/src/lib.rs:284`, `ring_mpsc/src/lib.rs:355`,
`ring_event/src/lib.rs:231`; `ring_core/src/lib.rs:129-130,
267-268, 336-337, 537-538`.

**Out of Scope:** How far `BytesSlot` reaches specifically, which is
[`integration/002`](002_where_the_second_shape_stops.md). `ring_slot`'s own
single outbound edge — `ring_types`, for one error variant — is covered in
[`workaround/002`](../workaround/002_batchtoolarge_borrowed_for_a_different_shape.md).

---

## The Inbound Edges

```sh
cd "$(git rev-parse --show-toplevel)"
# who declares the dependency — every crate in ring/, not just the ring_* ones
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -l 'ring_slot' */Cargo.toml | sed 's|/Cargo.toml||' \
  | grep -v '^ring_slot$' | sort

# which of the three public items each one touches, comments stripped
printf '%-14s %6s %6s %6s\n' crate Slot Typed Bytes
for c in ring_store ring_spsc ring_tls ring_bench ring_core ring_mpsc ring_event; do
  body=$( grep -vE "^[[:space:]]*//" $c/src/*.rs )
  printf '%-14s %6d %6d %6d\n' "$c" \
    "$( printf '%s' "$body" | grep -cE '\bSlot\b' )" \
    "$( printf '%s' "$body" | grep -c 'TypedSlot' )" \
    "$( printf '%s' "$body" | grep -c 'BytesSlot' )"
done
```

Live output:

```
ring_bench
ring_store
ring_core
ring_event
ring_mpsc
ring_spsc
ring_tls
crate            Slot  Typed  Bytes
ring_store         3      0      0
ring_spsc           2      2      0
ring_tls            0      0      0
ring_bench          0      5      0
ring_core           0     13      0
ring_mpsc           2      2      0
ring_event          2      4      4
```

Seven dependents — more than any other Tier 1 crate in the family. The comment
filter matters here: without it `ring_spsc` and `ring_mpsc` score in the dozens,
almost all of it doctests, which is exactly the distinction this table is drawn
to make.

`ring_tls` scores zero on all three because its dependency is a dev-dependency,
correctly placed:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\[dependencies\]/,/^\[lints\]/p' ring_tls/Cargo.toml
```

Live output:

```
[dependencies]
ring_types = { path = "../ring_types" }
ring_atomic = { path = "../ring_atomic" }
ring_batch = { path = "../ring_batch" }

[dev-dependencies]
ring_store = { path = "../ring_store" }
ring_event = { path = "../ring_event" }
ring_slot = { path = "../ring_slot" }

[lints]
```

---

### SL1 — Four Crates Stay Generic Over Three Layers, and the Fourth Layer Drops It

```sh
cd "$(git rev-parse --show-toplevel)"
# every `S : Slot` bound in the family, doc-comment lines stripped
for f in ring_*/src/*.rs; do
  case "$f" in ring_slot/*) continue;; esac
  grep -E '\bS *: *Slot\b' "$f" | grep -vE '^[0-9]+:[[:space:]]*//' | sed "s|^|${f}:|"
done | sort
```

Live output:

```
ring_store/src/lib.rs:impl< S : Slot + Default > Buffer< S >
ring_event/src/lib.rs:  S : Slot,
ring_mpsc/src/lib.rs:impl< S : Slot + Default > Ring< S >
ring_spsc/src/lib.rs:impl< S : Slot + Default > Ring< S >
```

Four bounds, in the four crates that sit between `ring_slot` and the caller. The
comment filter used to be load-bearing here and no longer is: `ring_spsc:247` and
`ring_mpsc:315` both discussed `S : Slot` in prose, putting the unfiltered count
at six, but both paragraphs explained why the `UnsafeCell` wrapped the whole
`Buffer` — and that arrangement was replaced by `Buffer< UnsafeCell< S > >` when
`Buffer::new`'s bound was narrowed to `Default`. Rewriting the two comments
removed the only two prose mentions in the family, so filtered and unfiltered now
both return four. The filter stays in the command: it is the family's standard
census form, and a claim it makes no difference to is cheaper to keep than to
re-derive the next time a comment mentions a bound.

The trait is used once more outside any bound, as a function value:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'Slot::is_empty' ring_store/src/lib.rs
```

Live output:

```
    self.slots.iter().all( Slot::is_empty )
```

**Finding.** The genericity holds all the way up. `ring_store` is generic, and
so are both ring crates built on it, and so is `ring_event`'s `recycle`. Three
layers of the family preserve the abstraction faithfully; a caller can pick
either shape and everything from storage through drain follows.

That is what makes SL2 a genuine break rather than a missing feature: the family
did the work to stay generic, at four sites, and `ring_core` — the only layer
above them — throws the result away.

The `Default` in `S : Slot + Default` is worth noting alongside: it appears in
three of the four bounds and is not part of the `Slot` trait. Both shapes
implement `Default` by delegating to `empty()`, so the requirement is met, but a
third slot shape would have to supply it separately — the real bound the family
relies on is `Slot + Default`, and only half of it is stated in `ring_slot`.

---

### SL2 — The Family's Unified Handle Cannot Carry a Bytes Ring

`ring_core` is the crate a caller reaches for when they want a ring without
choosing an implementation. Its storage enum is not generic over `Slot`:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'TypedSlot' ring_core/src/lib.rs
```

Live output:

```
use ring_slot::TypedSlot;
  Spsc( ring_spsc::Ring< TypedSlot< T > > ),
  Mpsc( ring_mpsc::Ring< TypedSlot< T > > ),
  Spsc( &'a mut ring_spsc::Ring< TypedSlot< T > > ),
  Mpsc( ring_mpsc::Ends< 'a, TypedSlot< T > > ),
  Spsc( ring_spsc::Producer< 'a, TypedSlot< T > > ),
  Mpsc( ring_mpsc::Producer< 'a, TypedSlot< T > > ),
  Spsc( ring_spsc::Consumer< 'a, TypedSlot< T > > ),
  Mpsc( ring_mpsc::Consumer< 'a, TypedSlot< T > > ),
        batch.get_mut( 0 ).and_then( TypedSlot::take )
        batch.get_mut( 0 ).and_then( TypedSlot::take )
        out.extend( ( 0..len ).filter_map( | offset | batch.get_mut( offset ).and_then( TypedSlot::take ) ) );
        out.extend( ( 0..len ).filter_map( | offset | batch.get_mut( offset ).and_then( TypedSlot::take ) ) );
```

Thirteen sites, spanning the storage enum, both end types, the producer, the
consumer, and the four drain paths. `TypedSlot< T >` is not a parameter there —
it is written into the type.

`ring_spsc` and `ring_mpsc` are both generic and both carry a `BytesSlot` ring in
their own doctests, so the foreclosure is `ring_core`'s alone:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rn 'Ring< BytesSlot' ring_spsc ring_mpsc --include='*.rs' | sort | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_mpsc/src/lib.rs:/// let ring : Ring< BytesSlot< 16 > > = Ring::new( Capacity::new( 4 ).unwrap() );
ring_mpsc/tests/mpsc_test.rs:    let mut ring : Ring< BytesSlot< 16 > > = Ring::new( capacity( 4 ) );
ring_spsc/src/lib.rs:/// let ring : Ring< BytesSlot< 16 > > = Ring::new( Capacity::new( 4 ).unwrap() );
ring_spsc/src/lib.rs:  /// let mut ring : Ring< BytesSlot< 8 > > = Ring::new( Capacity::new( 2 ).unwrap() );
ring_spsc/src/lib.rs:  /// let mut ring : Ring< BytesSlot< 8 > > = Ring::new( Capacity::new( 2 ).unwrap() );
ring_spsc/tests/spsc_test.rs:    let mut ring : Ring< BytesSlot< 8 > > = Ring::new( cap( CAPACITY ) );
ring_spsc/tests/spsc_test.rs:    let mut ring : Ring< BytesSlot< 4 > > = Ring::new( cap( 2 ) );
```

**Finding.** This crate's own constraint is that "both use the same claim, gating,
and drain — the difference is confined to what a slot contains." That holds
through `ring_store`, `ring_spsc`, and `ring_mpsc`, all of which carry either
shape. It stops at `ring_core`, which hard-wires `TypedSlot< T >` at thirteen
sites, so opaque host traffic — the traffic `BytesSlot` exists for — has no route
through the family's unified handle and must drop to a concrete ring crate to
find one.

Whether that is a defect depends on a ruling nobody has made: either `ring_core`
is deliberately the typed-traffic handle and should say so, or it is the family's
front door and the `TypedSlot` in its signature should be a parameter. Nothing in
`ring_core`'s documentation addresses the question, and its own dependency on
`ring_slot` is what makes the choice invisible — it *looks* like it supports the
crate's whole surface.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_where_the_second_shape_stops.md) | How far `BytesSlot` actually reaches, and the bench this crate's own argument calls for |
| [`pattern/001`](../pattern/001_one_trait_two_shapes.md) | The one-trait-two-shapes pattern, and the single point that enforces it |
| [`api/002`](../api/002_a_trait_with_two_methods.md) | What the trait offers, and why two methods is the whole of it |
| [`type/001`](../type/001_two_shapes_one_trait_no_copy.md) | What each shape commits to, including the `Copy` neither has |

### Sources

| Fact | Where |
|------|-------|
| Seven declared dependents | `*/Cargo.toml` |
| `ring_tls`'s dev-only edge | `ring_tls/Cargo.toml:13-16` |
| The four `S : Slot` bounds | `ring_store:96`, `ring_spsc:284`, `ring_mpsc:355`, `ring_event:231` |
| `Slot::is_empty` as a function value | `ring_store/src/lib.rs:140` |
| `ring_core`'s thirteen `TypedSlot` sites | `ring_core/src/lib.rs` |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_drive_through_the_trait_alone` | That `dyn Slot` drives either shape — the property `ring_store` relies on |
| `the_trait_reports_the_same_cycle_for_both_shapes` | Both shapes indistinguishable through the trait across the full occupancy cycle |
| `ring_store` — `the_same_buffer_type_serves_both_slot_shapes` | The generic instantiated at both shapes, in the crate all rings build on |
| *(to create)* | A compile-fail or doc assertion that `ring_core::Ring` is typed-only, if that is the ruling |
