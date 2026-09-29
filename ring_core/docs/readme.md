# docs

Design documentation for `ring_core`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Dispatch to a backend on publish and on drain — the only two procedures this crate owns |
| `api/` | The publish and drain contracts, marked per operation as uniform or per-backend |
| `data_structure/` | Four parallel three-way enums whose arity is a compile-time property |
| `decisions/` | Architecture Decision Records for open trade-offs this crate could not close unilaterally |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Six in-house edges, one external edge, and the unreconciled surface above |
| `invariant/` | What a composition layer must cost nothing of, and what swapping the thing beneath it may not change |
| `item/` | Thirty-six items by kind, and which of the twenty-seven public ones the family actually reaches |
| `lifecycle/` | The two irreversible moments in a ring's life, and the occupancy states a caller programs against between them |
| `non_functional_requirement/` | "Swappable" given a threshold and a reading |
| `pattern/` | The reusable composition shape, its two rules, and its characteristic bug |
| `pitfall/` | One trap for callers of the surface, one for readers of the coverage number |
| `type/` | One public enum with build-dependent arity, and one concept deliberately not a type |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

## What Makes This Crate Different From Its Siblings

`ring_core` composes; every other crate in the family implements. Three facts
follow from that and shape everything above:

**1. It is the only crate that is two programs rather than one.** The
`crossbeam` cargo feature means `Storage`, `Backend`, and every `match` over
them have three arms in one build and two in the other. This is not a detail —
it broke the family's Validation Machinery, which was single-configuration until
this crate arrived:

| Reading | Default build | `--features crossbeam` |
|---|---|---|
| `ring_core` line coverage | 93/119 (78.2%) | 120/120 (100%) |

The 26 differing lines are the `Crossbeam` arms, which the compiler deleted and
tarpaulin counted as missed. Both G1 and G2 were amended as a result — the fix
was to the instrument, not to the crate
(→ [`pitfall/002`](pitfall/002_feature_gated_code_reads_as_uncovered.md)).

**2. It is the only crate with a non-sibling dependency.** `crossbeam-queue`
behind the feature flag, which is why this is also the only crate whose
`workaround/` has an instance
(→ [`workaround/001`](workaround/001_crossbeam_queue_as_interim_backend.md)).

**3. It is the first crate whose bugs live in the seams rather than the
mechanisms.** The crate shipped one build with a `try_push` that panicked on any
full multi-producer ring — not because the MPSC ring was wrong, but because its
`push` takes the record by value while this surface promises to hand it back.
Measured under that bug restored: **4 integration failures, 0 of 7 doc-test
failures.** Every doc example builds an SPSC ring, which is the right shape for
documentation and the wrong shape for verification
(→ [`algorithm/001`](algorithm/001_backend_dispatch_and_the_refusal_seam.md),
[`pattern/001`](pattern/001_uniform_surface_over_unequal_backends.md)).

## How to Read These in Order

| If you want to | Start at |
|---|---|
| Use the crate | [`api/001`](api/001_producer_surface.md), [`api/002`](api/002_consumer_surface.md), then [`pitfall/001`](pitfall/001_free_capacity_carries_two_contracts.md) — the one trap the signatures cannot warn you about |
| Know what is guaranteed across backends | [`invariant/002`](invariant/002_uniform_delivery_across_backends.md), which states the three exclusions as well as the promise |
| Add a fourth backend | [`pattern/001`](pattern/001_uniform_surface_over_unequal_backends.md), then [`algorithm/001`](algorithm/001_backend_dispatch_and_the_refusal_seam.md)'s seam check |
| Reconcile this with `ring_handle` | [`integration/002`](integration/002_handle_surface_divergence.md) — it names the obligation and the single blocking crate |
| Understand the coverage number | [`pitfall/002`](pitfall/002_feature_gated_code_reads_as_uncovered.md) |

## Verification

Every claim in these instances that can be a command is one, and the commands
live in [`tests/manual/readme.md`](../tests/manual/readme.md) as stages C1–C9
rather than being scattered here. That plan was run in full on 2026-08-28;
**six of nine stages were wrong as first written**, and the corrections changed
two gate scripts, one test, and two of these documents.

### Related Crates

| Crate | Relationship |
|-------|--------------|
| [`ring_spsc`](../../ring_spsc/docs/readme.md) | Backend 1, and the crate whose zero-RMW assertion [`invariant/001`](invariant/001_no_atomic_of_its_own.md) exists to protect |
| [`ring_mpsc`](../../ring_mpsc/docs/readme.md) | Backend 2, and the source of the by-value `push` signature the refusal seam works around |
| [`ring_handle`](../../ring_handle/docs/readme.md) | Specifies this surface one layer up; the divergence is `ring_handle`'s to reconcile |
| [`ring_shutdown`](../../ring_shutdown/readme.md) | Owns the liveness flag this crate deliberately does not copy |
| [`ring_stats`](../../ring_stats/readme.md) | Where instrumentation belongs — composed *around* this crate, never inside it |
| [`ring_bench`](../../ring_bench/readme.md) | Will measure the backends against each other; `Backend` exists partly so its results can be labelled |
