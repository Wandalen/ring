# Integration: The Two Crates That Declined

### Scope

- **Purpose**: Record why this crate has no dependents, in the words of the two crates that considered depending on it and chose not to — for opposite reasons.
- **Responsibility**: Quote each declining rationale, show that they are not the same objection, trace the ruling that made one of them binding, and check whether the escalation condition that ruling set has since been met.
- **In Scope**: `ring_spsc`'s and `ring_mpsc`'s stated grounds; the ruling that settled the question; the state of `ring_core`.
- **Out of Scope**: The graph facts themselves — see [`integration/001`](001_ten_crates_name_it_and_none_depends_on_it.md).

### Two Refusals, Opposite Directions

Both candidate consumers wrote down why they are not using this crate. Neither
says it is wrong. Each says it solves a problem the declining crate does not
have — and they mean *different* problems, at the two ends of the producer-count
range this crate sits in the middle of.

| | `ring_spsc` (one producer) | `ring_mpsc` (many producers) |
|--|---------------------------|------------------------------|
| Where | `src/lib.rs:20-22` | `src/lib.rs:15-21` |
| The objection | the wait never happens | the wait must never happen |
| In its words | "spins until the frontier reaches this range's start, because producer B may finish before producer A. **At one producer, the frontier is always already there.**" | "spins until the *predecessor* producer has published, which makes one producer's progress depend on another's — **the one coupling the contended-claim feature exists to remove.**" |
| So the crate is | unnecessary | unacceptable |
| Ruled by | the crate's own thesis, `:8-30` | `ring_mpsc`'s own design choice — a per-slot stamp array instead of a shared cursor |
| What replaced it | nothing — a plain store | a per-slot `Seq` stamp array |

`ring_spsc:8-13` frames its whole dependency list this way: *"`ring_gating`,
`ring_claim`, `ring_publish` and `ring_consume` are all absent, and that absence
**is** the crate's thesis rather than an omission. Each of those exists to answer
a question that only has an answer worth computing when producers can overtake
one another."* This crate's row in that list is the third of four. At one
producer the `compare_exchange` in `try_publish` can never fail, so the loop in
`publish` can never spin, so the whole crate collapses to a store — which is what
`ring_spsc` writes instead.

`ring_mpsc` reaches the opposite conclusion from the same mechanism. Its
producers *can* overtake one another, which is exactly when this crate starts
spinning — and a spin on a peer producer is the coupling that crate's
contended-claim design exists to eliminate.

### PB4 — This Crate's Own Module Doc Is What Ruled Against It

The module documentation anticipated `ring_mpsc` **by name** before that crate
called it, at `src/lib.rs:36-40`:

> That works, and it is what a high-contention multi-producer ring eventually
> needs; it is deliberately not here, because it is `ring_mpsc`'s problem at S5
> and putting it in the primitive would make the primitive untestable without a
> second producer.

So the primitive's own scope statement decided the mechanism question — stamps
or a published cursor — before it was ever asked: the dependency structure
answers it against itself. That is an unusual and healthy
outcome: the boundary held under contact rather than being renegotiated. It is
also why the zero in the dependent column is a *result* rather than an absence —
somebody read this crate's documentation, found it said "not for you", and
believed it.

The alternative `ring_mpsc` rejected is the strongest statement anywhere of what
depending on this crate would have bought: using `Publisher` anyway and
accepting the spin. It needs no new field, reuses a tested crate, and makes the
drain a `ring_consume::Consumer` over a one-member barrier — the whole handshake
for free. It was rejected because a publishing producer would then block on a
*peer producer's* progress, which is the one thing the contended-claim design
exists to avoid, and because it would contradict three instances to satisfy one.

"The whole handshake for free" is not faint praise, and the rejection is not on
the mechanism's correctness. It is on one property — a producer waiting for a
peer — which is precisely the property
[`decisions/001`](../decisions/001_refused_rather_than_reordered.md) records this
crate as having chosen deliberately.

### PB5 — An Escalation Condition Was Met and the Decision It Called For Is Not Written

At one point neither `ring_publish` nor `ring_consume` was used by any crate in
the family. That was a real finding rather than a resolved one: both are
complete, tested, and exercised by their own suites, and `ring_core` was named
as the next crate that could adopt them. Whether it does settles whether the
family carries two primitives with no consumer.

`ring_core` has since been implemented. It does not adopt them:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_core/Cargo.toml \
  | grep -oE '^ring_[a-z_]+'
# has any ruling after 124 revisited the question it left open?
after=$( for d in docs/decision/[0-9]*.md; do
  n=$( basename "$d" | cut -d_ -f1 )
  [ "$n" -gt 124 ] && grep -l 'ring_publish' "$d"
  true
done )
[ -n "$after" ] && echo "$after" || echo '(no ruling after 124 names ring_publish)'
# control: the identical expression with the bound lowered by one
for d in docs/decision/[0-9]*.md; do
  n=$( basename "$d" | cut -d_ -f1 )
  [ "$n" -ge 123 ] && grep -l 'ring_publish' "$d"
  true
done
```

Live output:

```
ring_config
ring_mpsc
ring_overflow
ring_slot
ring_spsc
ring_types
(no ruling after 124 names ring_publish)
docs/decision/123_ring_shared_slot_storage_unsafe_sited.md
docs/decision/124_ring_mpsc_publication_stamped_not_cursor.md
```

`ring_core` declares `ring_config`, `ring_mpsc`, `ring_overflow`, `ring_slot`,
`ring_spsc`, `ring_types` — the two backends and their support, neither of which
reaches this crate, because both are the crates that already declined it.
Nothing has since revisited this crate's consumer question.

So the open condition has been met, and **the decision it called for has not
been written.** This is a documentation gap with a
named owner and a named trigger, not a discovery — the question was raised with
its own follow-up in mind, and the follow-up is outstanding. The crate is not broken by it; it is
complete, covered, and correct. What is missing is the record of whether a
correct, complete, uncalled primitive should be kept, folded into `ring_claim`,
or retired.

`ring_consume` is in the identical position, by the same decision, on the same
schedule. Its only dependent edge in the whole family is
`ring_publish/Cargo.toml:17` — this crate's own `[dev-dependencies]`, for
the reached-test. Two primitives whose sole remaining mutual consumer is one
test file, in one of them.

### PB6 — The Declining Crate Reproduced the Constant It Declined

`ring_mpsc` did not take the crate, but it took the crate's ordering decision —
independently, with near-identical justifying prose.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'const PUBLISH' */src/*.rs
```

Live output:

```
ring_mpsc/src/lib.rs:pub const PUBLISH: Ordering = Ordering::Release;
ring_publish/src/lib.rs:const PUBLISH: core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
```

| | `ring_publish:60-67` | `ring_mpsc:224-237` |
|--|---------------------|---------------------|
| Name | `PUBLISH` | `PUBLISH` |
| Value | `Ordering::Release` | `Ordering::Release` |
| Applied to | a cursor `compare_exchange` (`:164`) | a per-slot stamp `store` |
| Paired with | `GATING`, the consumer's `Acquire` | `OBSERVE`, its own `Acquire` (`:234`) |
| Rationale | "that pairing is the entire happens-before edge between a producer's slot writes and a consumer's reads of them" | "everything the producer wrote into the slot before this store is visible to a consumer that observes the stamp" |
| The x86/aarch64 sentence | `:64-66` | `:212-214` — verbatim, plus a trailing cross-reference |
| Has a doctest asserting the value | **no** | **yes** (`:216-220`) |

The two constants are the same decision reached twice, which is the expected and
correct outcome — the ordering is a property of publication, not of a mechanism,
so a crate that reimplements the mechanism should land on the same ordering. What
is worth recording is the last row. `ring_mpsc`'s copy is asserted by a doctest;
this crate's is not asserted by anything automatic, and its coverage is
`tests/manual/readme.md § P3` (a grep, run by hand) and § P1 (a mutation, run by
hand). See
[`pattern/002`](../pattern/002_the_named_ordering_constant.md) for the
family-wide census of all eleven named `Ordering` constants, and
[`non_functional_requirement/002`](../non_functional_requirement/002_what_the_spin_costs.md)
for why the automated suite cannot supply that assertion.

### What Would Give This Crate a Consumer

Three routes, in ascending order of how much would have to change:

1. **A bounded multi-producer ring that accepts the coupling.** `ring_mpsc`
   rejected the spin for its contended-claim design's sake; a variant that does
   not carry that constraint would find "the whole handshake for free" a good
   trade.
2. **`ring_core` gaining a fourth backend** that is neither `ring_spsc` nor
   `ring_mpsc` — the composition point exists and its backend set is a cargo
   feature away from growing.
3. **A consumer outside this family.** Nothing about `Publisher` is
   ring-specific: it is a monotone contiguous frontier over `Seq`, and the two
   crates that declined it did so on producer-count grounds that another domain
   need not share.

None is scheduled. The honest reading is that the family built a correct
primitive one stage before it knew which shape of publication it wanted, then
learned the answer once `ring_mpsc` existed and did not need this shape.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The surface neither crate called |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | The property `ring_mpsc` declined over, chosen deliberately here |
| [../decisions/002_a_plain_spin_rather_than_a_wait_kind.md](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) | The spin `ring_spsc` says never runs and `ring_mpsc` says must never run |

### Integrations

| File | Relationship |
|------|--------------|
| [001_ten_crates_name_it_and_none_depends_on_it.md](001_ten_crates_name_it_and_none_depends_on_it.md) | The graph facts these rationales explain |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_the_spin_costs.md](../non_functional_requirement/002_what_the_spin_costs.md) | What the coupling actually costs, measured |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_named_ordering_constant.md](../pattern/002_the_named_ordering_constant.md) | The constant both crates declared, and the family-wide census |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_conflating_the_two_cursors.md](../pitfall/002_conflating_the_two_cursors.md) | The failure mode `ring_mpsc`'s stamp array avoids differently |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:29-40,60-67` | The refusal-not-reordering argument, naming `ring_mpsc`, and the constant |
| `ring_spsc/src/lib.rs:8-30` | Absence as thesis; this crate's row at `:20-22` |
| `ring_mpsc/src/lib.rs:13-28, 224-237` | The deliberate non-use, and the reproduced constant |
| `ring_mpsc/Cargo.toml:8-11` | The removal, recorded in the manifest |
| `ring_core/Cargo.toml:14-21` | The named next adopter, which did not adopt |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md § P1` | The mutation check that covers the ordering the automated suite cannot |
| `tests/manual/readme.md § P3` | The ordering is named once, and it is `Release` |
| `tests/manual/readme.md § P4` | Publication is refused, never reordered — the rejected alternatives are absent from the code |
