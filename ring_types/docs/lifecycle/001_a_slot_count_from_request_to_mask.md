# Lifecycle: A Slot Count From Request to Mask

### Scope

- **Purpose**: Trace a slot count's arc from an unvalidated number in a config or a manifest row through validation into a fold, naming which crate owns each phase and where the number stops being a number.
- **Responsibility**: State the lifecycle phases, phase transitions, dependencies, and cleanup requirements.
- **In Scope**: Six phases across four crates; the one that can refuse; what survives after the value is consumed.
- **Out of Scope**: The two-test validation as an algorithm (→ [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)); the fold's implementation, owned by `ring_index`.

### Lifecycle Phases

| # | Phase | Owner | What exists |
|---|-------|-------|-------------|
| L1 | **Requested** | The caller | A `usize` — a literal, a config field, a sweep row, or an unset field defaulting to zero |
| L2 | **Validated** | **This crate** | `Capacity::new` accepts or refuses. The only phase that can fail |
| L3 | **Held** | Any of 17 crates | A `Capacity` travelling by value, `Copy`, 8 bytes |
| L4 | **Read as a count** | `ring_store`, `ring_config`, others | `get()` — the number back out, for allocation or reporting |
| L5 | **Read as a mask** | `ring_index` | `mask()` — `get() - 1`, unchecked, total |
| L6 | **Folded** | `ring_index` | `seq & mask` produces a `SlotIndex`; the `Capacity` is not consumed |

**L2 is the only phase this crate owns and the only one that can refuse.**
Everything before it is a caller's `usize` and everything after it is a value
whose validity is settled.

**L5 and L6 are the phases the whole arc exists to make safe.** `mask()` is
`self.0 - 1` with no check, and the fold is `&` rather than `%` — both are
correct only because L2 rejected zero and every non-power-of-two.

**L3 has no duration and no owner.** A `Capacity` is `Copy` and 8 bytes, so it
is duplicated rather than moved; there is no single value travelling, but many
identical ones. This is why the phase list is a shape rather than a timeline —
several L3 copies coexist, and any of them may be at L4, L5 or L6 concurrently.

### Phase Transitions

| # | From → To | Trigger | Can fail |
|---|-----------|---------|:--------:|
| X1 | L1 → L2 | `Capacity::new( slots )` | — |
| X2 | L2 → ✗ | `slots == 0` | **Yes** — `RingError::CapacityZero` |
| X3 | L2 → ✗ | `!slots.is_power_of_two()` | **Yes** — `RingError::CapacityNotPowerOfTwo( slots )` |
| X4 | L2 → L3 | Both tests pass | No |
| X5 | L3 → L3 | Copy into another crate, struct, or call | No |
| X6 | L3 → L4 | `get()` | No |
| X7 | L3 → L5 | `mask()` | No |
| X8 | L5 → L6 | `seq & mask` in `ring_index` | No |
| X9 | L3 → ∅ | The holder drops | No |

**X2 and X3 are the only fallible transitions in the entire crate.** Every other
edge here, and every operation on `Seq`, `SlotIndex`, `WaitKind` and
`OverflowPolicy`, is total.

**X4 is irreversible in the useful direction.** There is no transition from L3
back to L1 — the original request is not stored, and there is nothing to
compare against. Unlike `ring_config`'s clamping setters, that costs nothing
here, because L2 *rejects* rather than corrects: a caller who asked for 100 got
an error naming 100, not a silent 128.

**X6 and X7 are not exclusive and do not consume.** A `Capacity` can be read as
a count and as a mask any number of times, in any order, from any copy. There is
no state machine inside the value.

**X9 is trivial, and stating so is the point.** Dropping a `Capacity` runs no
destructor and releases nothing — it is a `usize` behind a newtype.

### Dependencies

| Crate | Phase | Relationship | Items |
|-------|-------|--------------|------:|
| **This crate** | L2 | Owns validation; declares `Capacity` and both refusal variants | 22 |
| [`ring_config`](../../../ring_config/readme.md) | L1→L2 | Holds a capacity in the record a ring is built from | 13 |
| [`ring_index`](../../../ring_index/readme.md) | L5, L6 | The one crate that folds — takes `Capacity` and `Seq`, returns `SlotIndex` | 3 |
| [`ring_store`](../../../ring_store/readme.md) | L4 | Sizes storage from `get()` | 19 |
| 14 further crates | L3, L4 | Hold or read a capacity | — |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rlE '\bCapacity\b' ring_*/src | command grep -v '^ring_types/' \
  | cut -d/ -f2 | sort -u | tr '\n' ' '
```

Live output:

```
ring_align ring_barrier ring_batch ring_store ring_claim ring_config ring_core ring_cursor ring_debug ring_gating ring_index ring_mpsc ring_seqno ring_shutdown ring_spsc ring_trace ring_wait 
```

**`ring_index` is three items and is the narrowest crate in the family.** That
is deliberate: it owns L5→L6, the one operation whose correctness depends on
L2's guarantee, and keeping it tiny keeps the guarantee's consumer surface tiny
with it.

**Nothing in this list re-validates.** No consumer re-checks `Capacity` with an
`is_power_of_two` call — the one hit outside this crate is `ring_align`'s
unrelated cache-line-size assertion, not a re-check
(→ [`../invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md), TY34)
— the seventeen consumers take L2's word for it, which is the return on
performing the check in a type rather than at each use
(→ [`invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md)).

### Cleanup Requirements

**There are none, at any phase, and each "none" has a different reason.**

| Phase | Cleanup | Why |
|-------|---------|-----|
| L1 | None | A `usize` |
| L2 | None | The `Err` path allocates nothing — `RingError` is `Copy`, 24 bytes |
| L3–L6 | None | `Capacity` is `Copy`, has no `Drop` impl, and owns no resource |
| L6 output | None | `SlotIndex` is `Copy` over a `usize` |

**The absence of `Drop` anywhere in this crate is a design property worth
stating.** A tier-0 vocabulary crate whose types own a resource would impose a
destructor on 31 downstream crates and make every `Copy` derive impossible.
Every type here is `Copy`, which forbids `Drop` outright — the two are mutually
exclusive in Rust — so the guarantee is structural rather than reviewed.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'impl Drop' ring_types/src/ | wc -l
```

Live output:

```
0
```

*(Counted rather than listed per file: the installed `grep` groups multi-file
output in its own order, so a per-file listing would not reproduce
byte-for-byte across environments. A total is order-independent.)*

**One consequence for the family's tick path:** a failed `Capacity::new`
allocates nothing and drops nothing, so validation can occur inside a
latency-sensitive setup path without a heap interaction. That is not currently
needed — construction happens at startup — but it is the property that makes
`RingError`'s `Copy` constraint coherent across the whole error surface
(→ [`non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | L2 in full — the two tests, their order, and the `const` context |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | X2 and X3 as the crate's one error channel |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) | L6's output, and why it reaches only three crates |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) | What X4 establishes and X5 preserves |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_function/001_capacity_new.md](../item/associated_function/001_capacity_new.md) | X1–X4 |
| [../item/associated_function/002_capacity_get.md](../item/associated_function/002_capacity_get.md) | X6 |
| [../item/associated_function/003_capacity_mask.md](../item/associated_function/003_capacity_mask.md) | X7 |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_an_error_from_construction_to_display.md](002_an_error_from_construction_to_display.md) | Where X2 and X3's output goes next |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_newtype_that_makes_a_check_unnecessary.md](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md) | Why L2 is one phase rather than fifteen |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_a_capacity_request_through_validation.md](../lifecycle/003_a_capacity_request_through_validation.md) | The same arc as states, with the reachability question X4 settles |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_capacity.md](../type/001_capacity.md) | The type these phases move |

### Sources

| File | Relationship |
|------|--------------|
| [`src/capacity.rs`](../../src/capacity.rs) | L2, L4, L5 — the whole owned portion of the arc |
| [`ring_index/src/lib.rs`](../../../ring_index/src/lib.rs) | L6, the fold; line 88 folds a whole run |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ X1–X4, X6 and X7 are all asserted — `capacity_accepts_powers_of_two` covers the success arc, `capacity_rejects_zero_and_non_powers_of_two` both refusals. ❌ **X8 is not tested here and should not be** — the fold belongs to `ring_index`, and testing it from this crate would duplicate that crate's suite while depending on a crate this one must not depend on |

### TY39 — Four of the Six Phases Belong to Crates That Never Re-Validate

The phase list is a description of trust transfer. After phase two the value is
never checked again anywhere in the family — the thirty-one measured `.get()` and
`.mask()` reads
(→ [`../invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md), TY35)
are all consumption.

Recorded here because the lifecycle reads as six equal steps and is in fact one
gate followed by five uses.
