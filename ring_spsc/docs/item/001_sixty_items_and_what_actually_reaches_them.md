# Item: Sixty Items, and What Actually Reaches Them

### Scope

- **Purpose**: Record the item census and the measured reach of each public name, so claims about this crate's blast radius rest on a count rather than on an impression.
- **Responsibility**: The thirty public names, and which of them any other crate actually mentions in code.
- **In Scope**: Every `ring_spsc::` reference in the family's non-comment source.
- **Out of Scope**: What the reached names do (→ [`../api/001`](../api/001_producer_surface.md), [`../api/002`](../api/002_consumer_surface.md)); whether the unreached ones should exist (→ [`../decisions/001`](../decisions/001_the_switching_cost_argument_undercounts_its_own_blast_radius.md)).

### Thirty Public Names

Sixty items, thirty names. The two figures count different things and both are
in [`readme.md`](readme.md)'s table: the census counts every declared item —
sixty, of which seventeen are private and thirteen are `impl` blocks that
declare no name of their own — while the *surface* is what a consumer can spell.
That is five structs, twenty-three associated functions, and two constants. The
structs:

| Struct | Owns | Obtained from |
|--------|------|---------------|
| `Ring< S >` | The slot array and both cursors | `new` / `with_config` |
| `Producer< 'a, S >` | Nothing — a borrow | `Ring::split` |
| `Reservation< 'a, S >` | Nothing — a borrow plus a sequence | `Producer::claim` |
| `Consumer< 'a, S >` | Nothing — a borrow | `Ring::split` |
| `Batch< 'a, S >` | Nothing — a borrow plus a range | `Consumer::drain` |

### What Reaches Them

Asked per name, across every crate in the family rather than a shortlist, and
counting only non-comment lines:

```sh
cd "$(git rev-parse --show-toplevel)"
for n in Ring Producer Consumer Reservation Batch OWN HANDOFF; do
  hits=$( for c in $( ls -d ring_*/ | tr -d / ); do
            [ "$c" = ring_spsc ] && continue
            find $c \( -name '*.rs' -o -name '*.stderr' \) -exec cat {} + 2>/dev/null \
              | grep -vE '^\s*(//|///|//!)' | grep -qE "ring_spsc(::| *::)$n\b" && echo $c
          done | tr '\n' ' ' )
  printf '%-12s %s\n' "$n" "${hits:-none}"
done
```

Live output:

```
Ring         ring_bench ring_core 
Producer     ring_core ring_handle 
Consumer     ring_core 
Reservation  none
Batch        none
OWN          none
HANDOFF      none
```

**Three of the thirty names leave the crate, and they are reached by three
crates rather than the two a manifest scan finds.** `ring_core` and `ring_bench`
depend on this crate and call into it. `ring_handle` does neither — its hit is
inside `tests/ui/producer_shared_across_threads.stderr`, a pinned compiler-error
fixture that names `ring_spsc::Producer<'_, ring_slot::TypedSlot<u32>>` and
copies the declaration line under it
(→ [`../decisions/001`](../decisions/001_the_switching_cost_argument_undercounts_its_own_blast_radius.md)).

The glob above therefore includes `*.stderr` deliberately. A `*.rs`-only scan
reports `Producer` as reached by one crate, which is the answer that makes the
coupling invisible.

| Name | Crossed by | In |
|------|-----------|-----|
| `Ring` | `ring_core`, `ring_bench` | A storage variant, a constructor, a `&mut` borrow, a benchmark harness |
| `Producer` | `ring_core`, `ring_handle` | One enum variant; one pinned diagnostic |
| `Consumer` | `ring_core` | One enum variant |
| `Reservation` | — | — |
| `Batch` | — | — |
| `OWN`, `HANDOFF` | — | — |

Seventeen further crates name `ring_spsc` in prose only.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | Declares all thirty public names |
| `../../../ring_core/src/lib.rs` | Five code references — the whole of this crate's contract in practice |
| `../../../ring_bench/src/lib.rs` | Two code references, `Ring` only |
| `../../../ring_handle/tests/ui/producer_shared_across_threads.stderr` | Pins `Producer`'s declaration without depending on the crate |

### SP26 — Thirty Public Names, Three Crossed

`Reservation` and `Batch` are obtained from a method and used inline — never
spelled in a signature outside this crate — which is the same pattern as
`ring_mpsc`'s `Reserved` and `Batch`.

**The two crates' external vocabularies differ by exactly one name**: `Ends`,
which exists only in the multi-producer crate because its `split` needs an
intermediate. Everything else about the surface a consumer sees is identical, and
that is what makes `ring_core`'s uniform backend possible.

### SP27 — No Enum, No Trait, No Module — in Either Composed Core

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_spsc ring_mpsc; do
  printf '%-11s enums=%s traits=%s\n' "$c" \
    "$( grep -vE '^\s*(//|///|//!)' $c/src/lib.rs | grep -cE '^(pub )?enum ' )" \
    "$( grep -vE '^\s*(//|///|//!)' $c/src/lib.rs | grep -cE '^(pub )?trait ' )"
done
```

Live output:

```
ring_spsc   enums=0 traits=0
ring_mpsc   enums=0 traits=0
```

Zero and zero, twice. Every state either crate has is cursor arithmetic, which is
why `ring_core` must supply its own `Backend` and `Storage` enums to hold the two
of them uniformly rather than inheriting a shared trait.

**The family has no ring trait**, and this census is where that becomes visible
as a fact about the code rather than a design preference.

### SP28 — Twenty-Three Public Functions Across Five Types

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
grep -vE '^\s*(//|///|//!)' src/lib.rs | awk '/^pub struct /{ t=$3; sub(/</,"",t) }
  /^  pub (const )?fn /{ c[t]++ } END{ for ( k in c ) printf "%-14s %d\n", k, c[k] }' | sort
```

Live output:

```
Batch          6
Consumer       5
Producer       6
Reservation    1
Ring           5
```

`Reservation`'s single public function is `sequence()`; everything else about it
is `Deref`/`DerefMut`. That is the narrowest useful surface for a guard — the
caller writes through it and asks it nothing.

`Batch`'s six are the widest, because a read-side handle has to offer both
random access and iteration and both need a mutable variant.
