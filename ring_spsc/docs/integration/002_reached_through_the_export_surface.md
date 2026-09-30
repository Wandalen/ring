# Integration: Reached Through the Export Surface

### Scope

- **Purpose**: State this crate's position relative to the family's five-crate Contract — that its capability is externally available while the crate itself is not — and account for what that indirection costs and buys.
- **Responsibility**: The boundary, the mechanism by which the capability reaches a consumer, the gate that enforces it, and the obligations the indirection transfers.
- **In Scope**: The export boundary; the `ring_factory` → `ring_handle` path; gate G5; what a consumer can and cannot express.
- **Out of Scope**: The dependencies beneath (→ [Family Dependency Seam](001_family_dependency_seam.md)); `ring_handle`'s own design.

### System Description

The family restricts external visibility to five crates: `ring_factory`,
`ring_handle`, `ring_tls`, `ring_flush`, `ring_types`.
**`ring_spsc` is not one of them**, and yet this crate's own API is
nonetheless externally-facing in practice — an apparent contradiction the
family's own layering rules address head-on.

**The resolution: the Contract stands unchanged, and "externally-facing" means
reachable through the surface, not separately importable.** A consumer that
wants an SPSC ring asks `ring_factory` for one and receives `ring_handle`
values; it never names `ring_spsc` in its own manifest.

```text
consumer
   │  depends on: ring_factory, ring_handle          (2 of the 5)
   ▼
ring_factory ──build( cfg )──▶ ( Producer, Consumer )   [ring_handle types]
   │
   ▼
ring_core ──── selects backend ────▶ ring_spsc  or  ring_mpsc  or  crossbeam
```

**The consumer's manifest is where the Contract is enforced, not its call
sites.** `bench_harness/gate/declared/ring/export_surface.txt` declares the
five names, and gate `G5` fails the build if a crate outside the family names
any other `ring_*` crate as a dependency. The check is on the dependency
declaration — mechanical, not a review judgement.

**What the indirection buys is the freedom the rest of this crate's docs keep
spending.** Twenty-eight of the 33 crates are freely refactorable because
nothing outside can name them. Every "undecided" marker in this crate — the
three producer shapes, the borrow-versus-copy drain, the cursor initialization
value — is affordable *because of this Contract*. The Contract's entire value
is that it is narrow — a five-crate seam is what makes 28 crates freely
refactorable.

**The contrast with [`ring_tls`](../../../ring_tls/docs/integration/001_family_dependency_seam.md)
is exact and instructive.** That crate is *on* the export list, so the same
class of open question — what its append signature looks like — is a public
contract question there and a refactor question here. Two sibling crates, the
same uncertainty, different prices, decided entirely by which side of a
five-name list they fall on.

### Integration Points

| # | Seam | Direction | What crosses it | What it requires |
|---|------|-----------|-----------------|------------------|
| E1 | [`ring_core`](../../../ring_core/readme.md) | → This crate | Backend selection | That this crate's surface is one of three interchangeable ones |
| E2 | [`ring_factory`](../../../ring_factory/readme.md) | → `ring_core` | `RingConfig`, including `producer_count` | That the factory chooses SPSC when `producer_count == 1` and rejects mismatches |
| E3 | [`ring_handle`](../../../ring_handle/readme.md) | → consumer | `Producer` and `Consumer` values | That the handle split enforces this crate's cardinality invariant |
| E4 | Gate `G5` | ↔ | The declared export surface | That no outside crate names `ring_spsc` |
| E5 | Gate `g3_features.sh` | ↔ | `tests/spsc_test.rs`'s own module-documentation citation | That the crate→feature edge exists as a test citation, never as prose |

**E3 carries this crate's most important obligation and it is a transfer, not a
delegation.** [Exactly One Producer, Exactly One Consumer](../invariant/001_exactly_one_producer_one_consumer.md)
cannot be enforced here — no runtime check distinguishes a second producer
thread from the first. The handle split is where it becomes true, and
`ring_handle` will only build the split correctly if it knows the requirement.
**A `ring_handle` implemented without reading this crate's invariant has no
reason to withhold `Clone`**, and the resulting `Producer: Clone` would compile,
pass its own tests, and silently permit the data race.

**E5 is how this crate is counted as delivered at all.** The acceptance table
is explicit that "a crate claims a feature by testing it, never by asserting it
in prose," so nothing in these docs — including this instance — contributes to
this crate's own capability being marked Reached. Only `tests/spsc_test.rs`'s
own citation does.

**E2 is the seam where a real mismatch can arrive.** `RingConfig` carries
`producer_count`; `ring_factory`'s own reached-test requires `Factory::build( cfg )`
to return a handle pair whose observable behaviour matches every field,
asserted one field at a time. For `producer_count` that means selecting MPSC when it
exceeds 1 — never handing a >1 config to this crate
(→ [Family Dependency Seam](001_family_dependency_seam.md)'s S6).

### Error Handling

| Failure | Meaning | Consequence |
|---------|---------|-------------|
| An outside crate names `ring_spsc` | The Contract is breached | **Build failure** at gate G5 — loud, mechanical, and the intended outcome |
| `ring_factory` selects SPSC for `producer_count > 1` | E2's mismatch | **Silent data race.** The worst failure in this table, and entirely above this crate |
| `ring_handle` exposes a `Clone` producer | E3's obligation not transferred | **Silent data race**, and it passes every test either crate writes in isolation |
| `tests/spsc_test.rs` omits the feature citation | E5 | The capability reports as unclaimed while the code is complete — a false negative, which is the safe direction |
| The crossbeam backend cannot express this surface | E1 | **Build failure** under `--features crossbeam` — caught by the dual-configuration run |

**Rows two and three are the pair worth dwelling on.** Both are silent data
races, both originate in another crate, and both stem from an obligation this
crate can only *state*. That is the structural cost of being internal: the
crate that knows the invariant is not the crate that can enforce it, and the
distance between them is where the failure lives.

**Row four is a false negative and that is deliberate.** A gate that greps for
a citation cannot be fooled into reporting Reached by a crate that merely
describes itself well — which is exactly the failure mode
`bench_harness`'s own [invariant/001](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)
(gate non-vacuity) exists to prevent.

### Compatibility Requirements

1. **This crate must never appear in an outside manifest.** Gate G5 enforces
   it; the requirement is stated here so a future author does not mistake this
   crate having an API of its own for licence to export it directly.
2. **Signature changes are family-internal and cost a refactor, not a break.**
   `ring_core` and `ring_handle` are the full blast radius. This is the
   requirement every "undecided" marker in this crate's docs depends on.
3. **The cardinality invariant must be restated where it is enforced.**
   `ring_handle`'s docs carry the non-`Clone` requirement; this crate's carry
   the reason. Neither is sufficient alone — E3's failure is precisely the two
   being written independently.
4. **`producer_count` mismatches are rejected at the factory** (E2), not
   coerced and not honoured.
5. **The claiming test must cite this crate's own capability textually**,
   since that citation is the only crate→feature edge the family records.
6. **Version movement is lockstep.**

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | Its Compatibility Guarantee 1 is Requirement 2; the surface a consumer never sees directly |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Same, for the consumer end |

### Integrations

| File | Relationship |
|------|--------------|
| [001_family_dependency_seam.md](001_family_dependency_seam.md) | The seams beneath; its S6 and S7 are E2 and E1 seen from below |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | The obligation E3 transfers, and the reason `ring_handle` withholds `Clone` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_ring_construction_and_teardown.md](../lifecycle/001_ring_construction_and_teardown.md) | Why its L1 lives at `ring_factory` rather than here |
| [../lifecycle/002_producer_consumer_pairing.md](../lifecycle/002_producer_consumer_pairing.md) | The pairing E3 hands out |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) | The test E5 requires the citation to live in |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) | Why the family's own authors, not consumers, are the exposed population — a direct consequence of this indirection |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | E5's rule — a crate claims a feature by testing it, never by asserting it in prose |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | Cites this crate's own capability in its module documentation and again on the reached-test — E5, without which it reports unclaimed |
| `tests/spsc_test.rs` | **Not yet written, and it belongs to [`ring_factory`](../../../ring_factory/readme.md), not here** — `Factory::build` does not yet exist. E1 and E2 are untested until it does; what this crate can assert on its own is the half above the seam, which `with_config_takes_the_capacity_and_ignores_the_rest` covers |

### SP20 — The Export Position Is Measurable and It Is Two Crates

Seven lines in two crates is the whole of this crate's external contract. That
is what "internal" means here, stated as a number rather than as a policy.

It is also identical to `ring_mpsc`'s external reach — the same two consumers,
the same names — which is worth recording because the two crates are otherwise
described as occupying different positions in the family.

### SP21 — `ring_core` Reaches This Crate Only Through Enum Variants

```sh
cd "$(git rev-parse --show-toplevel)"
cat ring_core/src/*.rs | grep -vE '^\s*(//|///|//!)' | grep 'ring_spsc'
```

Live output:

```
    Spsc(ring_spsc::Ring<TypedSlot<T>>),
            false => Storage::Spsc(ring_spsc::Ring::with_config(config)),
    Spsc(&'a mut ring_spsc::Ring<TypedSlot<T>>),
    Spsc(ring_spsc::Producer<'a, TypedSlot<T>>),
    Spsc(ring_spsc::Consumer<'a, TypedSlot<T>>),
```

Five lines, five variant declarations or constructor calls. No method of this
crate is named anywhere in `ring_core`'s source — the calls happen through the
`Producer`/`Consumer` values after the `match`, so the method names never appear
in text.

**A grep for a method name of this crate across the family therefore returns
nothing, and that is not evidence the method is unused.** It is the one place in
this corpus where the reach measurement's usual technique does not work.
