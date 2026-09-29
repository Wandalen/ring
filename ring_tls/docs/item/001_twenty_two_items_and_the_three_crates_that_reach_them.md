# Item: Twenty-Two Items, and the Three Crates That Reach Them

### Scope

- **Purpose**: Record the item census and the measured reach of each public name, so claims about this crate's surface rest on a count rather than on the corpus that describes a different crate.
- **Responsibility**: The sixteen public names, the two trait methods a `pub`-keyed count misses, and which crates actually reach them.
- **In Scope**: Every `ring_tls::` reference in the family's non-comment source and every `ring_tls` line in its manifests.
- **Out of Scope**: What the reached names do (→ [`../api/001`](../api/001_writer_append_surface.md), which specifies a surface this crate does not have); the eight specified-but-absent names (→ [`../decisions/001`](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md)).

### Two Structs

| Struct | Owns | Obtained from |
|--------|------|---------------|
| `TlsBuffer< T >` | A `Vec< T >` and a refusal bound | `with_capacity` |
| `Flush< 'a, T >` | Nothing — a `BatchClaim`, a counter, and a borrow | `TlsBuffer::flush_into` |

`TlsBuffer` carries ten of the twelve associated functions. `Flush` carries two
inherent (`claim`, and `next`/`size_hint` through `Iterator`) and is otherwise
an iterator.

### What Reaches Them

```sh
cd "$(git rev-parse --show-toplevel)"
for c in $( ls -d ring_*/ | tr -d / ); do
  [ "$c" = ring_tls ] && continue
  dep=$( grep -vE '^\s*#' $c/Cargo.toml | grep -c '^ring_tls = ' )
  code=$( find $c -name '*.rs' -exec cat {} + 2>/dev/null \
          | grep -vE '^\s*(//|///|//!)' | grep -c 'ring_tls' )
  [ "$dep$code" = 00 ] && continue
  printf '%-14s declares=%-3s uses=%s\n' "$c" "$dep" "$code"
done
```

Live output:

```
ring_bench     declares=1   uses=1
ring_flush     declares=1   uses=3
ring_testkit   declares=1   uses=1
```

**Three crates, all declaring the dependency and all using it.** No prose-only
consumer and no undeclared use — a cleaner reach picture than either composed
core's, and the reason is that this crate's surface is small enough to be used
whole.

The names actually crossed are two: `TlsBuffer` and `with_capacity`, plus
`drain` and `flush_into` inside `ring_flush`. `Flush` is never spelled — it is
obtained from `flush_into` and consumed inline.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | Declares all sixteen public names |
| `../../../ring_flush/src/lib.rs` | Holds a `TlsBuffer< T >` as a struct field and calls `drain` |
| `../../../ring_testkit/src/lib.rs` | Constructs one per staging run |
| `../../../ring_bench/src/lib.rs` | Constructs one per workload |

### TL32 — Two of the Sixteen Public Items Are Filed as Private by Any `pub`-Keyed Count

A trait impl's methods take their visibility from the trait. The awk census
this crate's `item/readme.md` publishes counts them under "priv fn", which is
what the token says and not what a caller sees.

The same awk is used across the family, so the same undercount applies wherever
a crate implements a public trait — this is the first crate in the corpus where
the affected methods are a meaningful share of the surface.

### TL33 — Only Two Public Names Cross a Crate Boundary

`Flush` is obtained from `flush_into` and consumed inline, so its name does
not appear in any consumer — which is why the `std::vec::Drain` in `drain`'s
signature (→ [`../workaround/001`](../workaround/001_the_only_std_type_in_a_public_signature.md))
is the crate's only type-level leak in practice as well as in principle.

Fourteen of the sixteen public items are reached only through the two that are
named.

### TL34 — The Reach Scan Is Manifest-Filtered Because the Code Filter Is Not Enough

A scan that filters only `//`-style comments reports `ring_factory` as a
consumer on the strength of a comment saying the dependency was dropped
(→ TL27). Both filters are needed and they are not the same expression.

This finding records the scan's construction rather than its result, because
the result is only trustworthy given the construction.
