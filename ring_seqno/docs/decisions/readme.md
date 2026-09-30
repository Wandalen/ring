# decisions

Choices that were live when this crate was written, with the alternatives and
what each would have cost.

### Overview Table

| ID | Name | Chosen | Alternative |
|----|------|--------|-------------|
| 001 | [`None` Rather Than Zero for an Empty Set](001_none_rather_than_zero_for_an_empty_set.md) | `Option< Seq >` from `slowest` | `Seq::ZERO` as an identity |
| 002 | [Saturating Rather Than Signed](002_saturating_rather_than_signed.md) | Every backward pair reads `0` | `i64`, or `Option`, or a panic |

### Both Are About the Same Refusal

Each decision is a refusal to invent a value for a state the arithmetic cannot
express.

001 refuses to pick an identity for an empty fold, because the two consumers
need opposite ones — and it is the decision that turned out right, provably, when
`ring_gating` resolved `None` to full capacity and `ring_barrier` resolved it to
zero.

002 refuses to represent direction, because a signed distance would push a case
onto every caller that no caller can act on. It is the decision that is right in
five places and costs something in one.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ13 | `None` for an empty set | n/a — observation | `GatingSet::headroom` and `Barrier::available` resolve the same `None` to opposite values, which is the empirical proof that an identity would have been wrong |
| SQ14 | Saturation's two directions | n/a — doc gap | Saturation makes a swapped `may_claim` or `free_slots` return a permissive answer while a swapped `pending` or `laps_between` returns an inert one — a distinction the source draws nowhere |
| SQ15 | `laps_backward_read_zero` | **misleading doc** | The test's rationale says a swapped caller "gets an obviously-wrong answer instead of a plausible one", but zero laps is exactly the value that reads as room to publish — it documents the permissive failure as though it were the safe one |
| SQ16 | The two integer widths | **latent hazard** | `may_claim` compared in `u64` while `free_slots` subtracted in `usize`, so the two readings that must agree did their arithmetic in different types — the mechanism that made a narrow-target divergence possible at all. `free_slots` now subtracts in `u64` too and narrows only its already-bounded result ([`non_functional_requirement/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) § SQ37) |

### Regenerate

Where the saturation and the borrowed span arithmetic sit:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'saturating_sub\|distance_to' ring_seqno/src/lib.rs
```

Live output:

```
    earlier.distance_to(later) / capacity.get() as u64
    consumer.distance_to(producer) < capacity.get() as u64
    let in_flight = consumer.distance_to(producer);
    (capacity.get() as u64).saturating_sub(in_flight) as usize
    consumer.distance_to(producer)
```
