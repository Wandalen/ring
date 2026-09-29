# decisions

Two choices, both made at a point where the obvious alternative compiles, passes
most of the suite, and is wrong. Each is stated in the crate's own module
documentation, which is unusual enough to be worth noting: `ring_gating` argues
for itself in prose before the first `pub` item.

### Overview Table

| ID | Name | Decides |
|----|------|---------|
| 001 | [`capacity` for an Empty Set](001_capacity_for_an_empty_set.md) | What `headroom` returns when nobody is reading, and why `0` deadlocks every ungated ring |
| 002 | [A `Result` Rather Than a `bool`](002_a_result_rather_than_a_bool.md) | Why the gate reports *why* it refused, and what a caller does differently with each reason |

### Both Are Stated in the Source

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A16 -F '//! ## The empty set is not a consumer at zero' ring_gating/src/lib.rs
```

| Lines | Decision |
|------:|----------|
| 22–29 | The empty-set identity |
| 31–38 | `Result` over `bool` |

Seventeen lines of module documentation for a 325-line crate whose implementation
is four one-liners and two short methods. The ratio is the point: the arithmetic
is elsewhere ([`algorithm/001`](../algorithm/001_headroom_in_two_delegations.md)),
so what is left here to get wrong is exactly these two judgements.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT15 | The `Seq::ZERO`-for-empty alternative | n/a — observation | It is caught by exactly two tests, one of which exists only to catch it — `an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap` builds both states the rejected implementation would conflate and asserts they agree while empty and differ after a lap |
| GT16 | The empty-set rule | n/a — duplication | Stated three times in one 325-line file — the module doc's "the empty set is not a consumer at zero", `new`'s "consumers of zero is legal and means ungated", and `headroom`'s "a full capacity when the set is empty". Three phrasings of one decision, none citing the others |
| GT17 | `is_configuration` / `is_transient` | n/a — coverage | The two predicates the whole `Result`-over-`bool` decision exists to feed are called 27 times from tests and **zero** times from production code in all 33 crates |
| GT62 | `GatingSet::check` | n/a — duplication | It has no caller outside this crate's tests. `ring_claim::claim` reproduces both of its branches by hand — the `BatchTooLarge { requested, capacity }` refusal at `src/lib.rs:287` and the `Full` at `:291` — and only one of the two needs to be inline: the headroom re-read must sit inside the CAS loop, the capacity test need not |
| GT18 | The error payloads | n/a — diagnostics | `BatchTooLarge { requested, capacity }` carries both numbers; `Full` carries none. The variant a caller must give up on has full detail, and the variant a caller must retry — where knowing how much room exists would inform a backoff — has nothing |
