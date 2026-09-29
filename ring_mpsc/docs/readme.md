# docs

Design documentation for `ring_mpsc`, as typed doc definitions.

> **Reading note added 2026-08-27 — the instances below cite a deleted
> predecessor crate on purpose.** That predecessor was deleted on 2026-08-26
> and carried no surviving mechanism of its own, so it has **no successor
> crate**. Its working mailbox mechanism is nonetheless the measured baseline
> this crate's adoption gate is written against — the thing a winning ring has
> to beat — so every reference to it below is a deliberate citation of a
> predecessor, not a stale name awaiting a rename. Repointing them at
> `ring_mpsc` would make this crate contrast against itself and destroy the
> comparison; they are left exactly as written.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The producer's claim-and-publish procedure and the consumer's batch drain, step by step |
| `api/` | The two asymmetric caller surfaces — many lock-free producers, one batch-shaped consumer |
| `data_structure/` | The ring's field-level shape — slots, per-slot stamps, and the two cache-line-separated cursors |
| `decisions/` | Architecture Decision Records for open trade-offs this crate could not close unilaterally |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | The eight sibling crates beneath this one and the prospective consumers above it |
| `invariant/` | The producer/consumer contract any winning ring mechanism must hold |
| `item/` | The four names that leave the crate, and the five public ordering constants |
| `lifecycle/` | The ring's four phases, a producer's shorter cycle nested inside them, and the slot and occupancy states both move through |
| `non_functional_requirement/` | The measured adoption gate — quality thresholds a benchmark can pass or fail |
| `pattern/` | How a declared channel binds to a ring instance and to a compile-time slot type |
| `pitfall/` | Traps that arrive bundled with the ring pattern, and what each costs the wrong workload |
| `type/` | The value types the correctness arguments are written in — sequence and capacity |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

The `invariant/` and `non_functional_requirement/` instances document the
**contract grain** the family itself decided —
sequence-numbered publication, single-consumer total order, adoption gated on
a measured verdict. The `algorithm/`, `data_structure/`, `pattern/`, and
`pitfall/` instances document the **mechanism grain** that follows from
naming this crate a sequence-numbered ring: a lock-free
compare-exchange claim, a per-slot sequence stamp, a batch drain, a
channel-to-ring binding rule, and the costs each carries. They specify the
shape, never which candidate wins — that verdict stays the family's own.

**Four of the five details the verdict was said to still own have since
closed, and the fifth is not this crate's.** The list read "capacity, stamp
width, watermark detection, backpressure policy, and whether the priority
classes are separate rings or one tagged ring." As of a later revision:

| Detail | State |
|---|---|
| Stamp width | Closed — `ring_types::Seq` is a `u64` newtype, fixed for the family (→ [`type/001`](type/001_sequence_number.md)) |
| Backpressure policy | Closed — `Fail`, because Block and Drop are three-line callers of it and Overwrite violates the total order (→ [`non_functional_requirement/002`](non_functional_requirement/002_bounded_capacity_backpressure.md)) |
| Capacity constraint | Closed — power-of-two required *in this crate*; the general relaxation stays a `ring_types` question (→ [`type/002`](type/002_capacity.md)) |
| Watermark detection | Closed by construction — the per-slot stamp scan; the producer-advanced-watermark alternative was ruled out because `ring_publish` declines to maintain one, and an availability array remains a live unmeasured alternative (→ [`algorithm/002`](algorithm/002_batch_drain_by_cursor_swap.md)) |
| Capacity *value*, and separate-versus-tagged rings | Still open, and correctly not this crate's — a per-instantiation configuration value and a consumer-side traffic-class decision |

What closed them was implementing the crate, not running the benchmark. The
distinction matters: none of the four was a *performance* question in the end,
which is why the adoption verdict never needed to arrive first. The two that
remain open are the two that genuinely depend on something outside this
crate.

The `api/`, `lifecycle/`, `type/`, and `integration/`
instances sit at a third, **surface grain**: what a caller touches, when, and
across which crate boundary. They were added after the first two grains
because several questions the mechanism instances left open turned out not to
be mechanism questions at all — the guard-versus-three-calls shape, the
drain's borrow-versus-copy return, the teardown drop contract, and the
five cross-seam properties this crate states but a sibling implements. Each
is recorded in exactly one place and referenced from the others.

No `format/` directory exists: nothing in this crate is a byte layout other
programs must agree on, and the ring's in-memory field layout is
`data_structure/`'s.

**`item/` exists, and the two reasons recorded here against it were overtaken
in different ways.** The first was an ordering constraint —
`item_des.rulebook.md` catalogs Rust items defined in the crate's own source
tree, `src/lib.rs` declared none, so any instance would have been invented
rather than documented, and the directory "becomes writable the moment the
crate has an implementation, not before." The implementation landed and that
reason simply came due.

The second outlived the first and was overruled rather than expiring. It
argued that a per-item catalog with File Usage, Crate Usage, and Caller/Callee
Trees would be written 33 times across this family, and that a reader
navigating the family goes through the export surface
([`ring_handle`](../../ring_handle/readme.md),
[`ring_factory`](../../ring_factory/readme.md)) rather than through each
internal crate's item list — so `item/` belonged *there* and not here. The
family answered by writing all 33, and what the two instances here carry is
the part that argument conceded a per-crate list would not duplicate:
[`item/001`](item/001_eighty_items_and_the_four_names_that_leave_the_crate.md)
records which names actually leave the crate, and
[`item/002`](item/002_five_public_ordering_constants.md) the five ordering
constants a caller can name.

Measured:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'declarations in src/lib.rs:       %s\n' \
  "$( command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' ring_mpsc/src/lib.rs )"
printf 'instances in docs/item:           %s\n' \
  "$( ls ring_mpsc/docs/item/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'family crates carrying docs/item: %s of %s\n' \
  "$( ls -d ring_*/docs/item 2>/dev/null | wc -l )" "$( ls -d ring_*/ | wc -l )"
```

Live output:

```
declarations in src/lib.rs:       50
instances in docs/item:           2
family crates carrying docs/item: 33 of 33
```

### Related Crates

Not yet a dependency of any crate — adoption is future work, gated on a
measured verdict that does not exist
(→ [`non_functional_requirement/001`](non_functional_requirement/001_measured_before_adopted.md)).
Implemented is not adopted, and the gate is deliberately not weakened by the
crate having become real. Doc references run in both directions across a crate
boundary once real; the rows below name the two prospective consumers this
crate was factored out to serve.

| Crate | Relationship |
|-------|--------------|
| the prospective consumer | Would lift its documented single-threaded intent-submission serialisation point |
| ~~the deleted predecessor crate~~ | Prospective consumer — would replace the predecessor's internal CAS-stack mailbox mechanism. **Deleted 2026-08-26**, and it has no successor. |
