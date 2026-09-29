# pitfall

Two traps, and neither can fire today — both sit behind the same fact, that no
production path calls into the half of this crate where they live. That makes this
the cheapest possible moment to write them down.

The first is a variant no shipping build can construct, which is also the sole
witness holding the crate's safety test non-vacuous. The second is a refusal
counted through a method called `record_drop`, on a code path that mutates shared
state before returning `Err`.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_variant_a_default_build_cannot_reach.md) | The Variant a Default Build Cannot Reach | Both build configurations, and what each does before resolving |
| [002](002_counting_a_refusal_through_the_drop_counter.md) | Counting a Refusal Through the Drop Counter | The unconditional record and the `Err` path that has already written |

## Unreachable, and Load-Bearing

`EvictedOldest` needs `OverflowPolicy::DropOldest`. With default features that
policy is rejected at construction; with `crossbeam` on, the publish path handles
it with `force_push` and returns before the resolution site. So `would_resolve`
sees `DropNewest` or `Fail` in production, never `DropOldest`.

The variant is nonetheless the only value for which `accepted_incoming()` is true,
which makes it the sole satisfier of the antecedent in the crate's safety test and
the only case separating the two predicates. Remove `DropOldest` from
`OverflowPolicy::ALL` and that test goes green and empty in one commit.

## A Loss That Is Not One

`lost_an_item` excludes `Refused` and a test asserts it. `resolve` records a drop
for `Fail` anyway, before the branch, and the event is summed into a total
documented as items lost.

The per-policy counters do stay separate, so nothing is unrecoverable — the
conflation is in `dropped_total` and in the recorder's name. What is missing is one
sentence saying the call counts an event rather than a loss, and one saying that
`Err` arrives with a counter already moved, so retrying double-counts.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the policy neither production path delivers --'
command grep -n 'DropOldest => Resolution::EvictedOldest' ring_overflow/src/lib.rs
command grep -n 'return Err( RingError::PolicyUnsupported )' ring_core/src/lib.rs
echo '  -- and the record that precedes every branch --'
command grep -m1 -F '  stats.record_drop( policy, 1 );' ring_overflow/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV29 | `ring_overflow` | **latent hazard** | `EvictedOldest` requires `OverflowPolicy::DropOldest`, which the default build rejects at construction with `PolicyUnsupported` and the `crossbeam` build handles via `force_push` and an early `return Ok( () )` before the resolution site — so one of three exported variants is unconstructible in every shipping configuration, a fact `ring_core` records for its own `match` and this crate now records at the variant's own declaration |
| OV30 | `ring_overflow` | **latent hazard** | The production-unreachable `EvictedOldest` is the only value for which `accepted_incoming()` is true, making it the sole satisfier of the antecedent in `no_resolution_overwrites_unread_data_silently`, the only case distinguishing the two predicates, and one of two arms the agreement test covers — so removing `DropOldest` from `OverflowPolicy::ALL` on the grounds that no default backend supports it would have emptied the crate's central safety test without turning it red; the witness count added under OV24 turns it red instead |
| OV31 | `ring_overflow` | **misleading doc** | `lost_an_item` excludes `Refused` and `a_refusal_loses_nothing` asserts it, while `resolve` calls `record_drop` for `OverflowPolicy::Fail` — landing in a distinct `failed` counter, so `dropped( Fail )` stays honest, but summed by `dropped_total()` under "Items lost across every policy" — leaving two reachable statements about the same event that contradict each other, with neither doc comment referencing the other |
| OV32 | `ring_overflow` | **latent hazard** | `resolve`'s first statement is an atomic `fetch_add`, so the `Err( RingError::Full )` path returns with shared state already mutated — contradicting the usual reading that `Err` means no effect — and while the counting contract does say "whichever branch is taken", the `# Errors` section directly below mentions no write, so a caller retrying a refused publish double-counted the same full-ring event with nothing warning against it; `# Errors` now states the write and the non-idempotence, pinned by its own test |
