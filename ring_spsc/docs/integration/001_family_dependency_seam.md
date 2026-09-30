# Integration: Family Dependency Seam

### Scope

- **Purpose**: Account for the five crates this one composes and, more informatively, for the four its sibling depends on that it deliberately does not — the absences that state the crate's thesis.
- **Responsibility**: Each dependency's contribution, the assumption it carries, the absences and their justification, and what a change here reaches.
- **In Scope**: The five declared dependencies; the four absences; the composition point above.
- **Out of Scope**: The consumer above the export boundary (→ [Reached Through the Export Surface](002_reached_through_the_export_surface.md)); each dependency's own mechanism.

### System Description

`ring_spsc` declares **five** dependencies:

```toml
[dependencies]
ring_store = { path = "../ring_store" }
ring_config = { path = "../ring_config" }
ring_cursor = { path = "../ring_cursor" }
ring_slot   = { path = "../ring_slot" }
ring_types  = { path = "../ring_types" }
# ring_gating, ring_claim, ring_publish and ring_consume are NOT here,
# and that is the point
```

**The four absences are the whole design difference**, and this is a rare case
of a dependency graph stating a thesis rather than merely recording a fact.

**An earlier draft of this instance declared six and it was wrong** — a
correction worth leaving on the record rather than quietly overwriting, because
the same error was found in two other scaffolded manifests in this family. The
draft listed `ring_claim`, `ring_publish` and `ring_consume` as dependencies on
the strength of their names alone. Each was then read, and each turned out to
exist for a question single-producer does not ask:

| Absent crate | What it actually owns | Why one producer does not need it |
|--------------|-----------------------|-----------------------------------|
| [`ring_claim`](../../../ring_claim/readme.md) | The contended CAS claim loop — a compare-and-swap loop over the producer cursor | One producer's claim is a `Relaxed` load of a cursor only it writes. There is no loop, no CAS, and nothing to lose a race to |
| [`ring_publish`](../../../ring_publish/readme.md) | Publication with **holes** — the per-slot stamp and the highest-contiguous scan that make out-of-order completion visible in order | One producer completes in claim order by construction, so the published sequence *is* the claimed sequence. A stamp would record a fact the cursor already carries |
| [`ring_consume`](../../../ring_consume/readme.md) | The commit with a gating bound computed via `ring_barrier` over a consumer set | The set has one member. The bound is the peer cursor, read directly |
| [`ring_gating`](../../../ring_gating/readme.md) | The producer-side check that reads `ring_barrier`'s minimum | The minimum over one value is that value — see below |

Using them anyway would have contradicted the crate's own thesis while
appearing to honour the family's layering: the manifest would have looked
maximally composed, and the code would have paid for machinery its central
claim is that it does not need.

| Layer | Crates | This crate's relationship |
|-------|--------|---------------------------|
| **Above** — composed into | [`ring_core`](../../../ring_core/readme.md) | Composes this and `ring_mpsc` as interchangeable backends |
| **This crate** | `ring_spsc` | Owns the single-producer single-consumer configuration, and nothing else |
| **Below** — composed from | `ring_store`, `ring_config`, `ring_cursor`, `ring_slot`, `ring_types` | Five crates: storage, configuration, cursors, slot views, shared newtypes |
| **Deliberately absent** | `ring_gating`, `ring_claim`, `ring_publish`, `ring_consume` | Each solves a multi-producer problem that degenerates to nothing at this cardinality |

#### Why `ring_gating` is absent, and why that is a justification rather than a gap

The general lapping problem is real for every ring including this one: a
producer must never advance past the slowest consumer. `ring_gating` exists to
solve it in general, and the family scopes it precisely — `ring_barrier` owns
the gating set and the minimum computation; `ring_gating` owns the
producer-side check that reads it.

At this cardinality **both halves degenerate to nothing**:

- The gating *set* has one member, so there is no set — just a cursor.
- The *minimum* over one value is that value, so there is no computation.
- The producer-side *check* is `producer - consumer < CAPACITY`, one
  subtraction and one compare, performed inline
  (→ [Uncontended Claim and Publish](../algorithm/001_uncontended_claim_and_publish.md)'s
  step 3).

This is exactly what this crate means by "no minimum across cursors."
Depending on `ring_gating` here would
import a set abstraction to hold one element and a minimum function to reduce
one value — machinery whose entire cost is the indirection, since the answer is
already in hand.

**The absence is worth contrasting with a genuinely questionable one
elsewhere.** [`ring_tls`](../../../ring_tls/docs/integration/001_family_dependency_seam.md)
records `ring_slot` as a dependency whose fit it cannot justify from the two
crates' own descriptions, and files that as an open question. This absence is
the opposite case: it *can* be justified, from this crate's own scope and
from the family's dependency-layering rules, and so it is recorded as settled. Recording both the same
way — as "notable" — would flatten a real distinction between a resolved design
choice and an unresolved one.

### Integration Points

| # | Seam | Direction | What crosses it | Assumption made |
|---|------|-----------|-----------------|-----------------|
| S1 | [`ring_store`](../../../ring_store/readme.md) | This crate → | The one-time slot allocation, indexed get/set | That it "holds no cursor and no ordering state" — this crate owns all of both |
| S2 | [`ring_cursor`](../../../ring_cursor/readme.md) | This crate → | `PaddedCursor`, load/store | That the padding contract holds — `align_of == 64`, `size_of == 64`, transitively via [`ring_align`](../../../ring_align/readme.md) |
| S3 | [`ring_slot`](../../../ring_slot/readme.md) | This crate → | `Slot`, `TypedSlot`, `BytesSlot` — the per-slot view the buffer is generic over | That a slot carries no synchronisation of its own; this crate's `unsafe` rests on the cursors alone (this crate's own design) |
| S4 | [`ring_types`](../../../ring_types/readme.md) | This crate → | `Seq`, `Capacity`, `RingError` | That `Seq` is a monotonic 64-bit counter that never wraps in practice, and that `Capacity` has already validated its power-of-two shape |
| S5 | [`ring_config`](../../../ring_config/readme.md) | This crate → | `RingConfig` — capacity, wait kind, overflow policy, batch size | That `producer_count` is a field this crate may reject rather than honour, one of several `RingConfig` carries |
| S6 | [`ring_core`](../../../ring_core/readme.md) | → This crate | The composition point; the backend swap | That the surface is expressible by the crossbeam backend too |

**The seam's real risk moved when the manifest was corrected, and it is worth
saying where it went.** While `ring_claim`, `ring_publish` and `ring_consume`
were declared, the risk was that each — being shared with `ring_mpsc` — would
offer only its contended, stamped, barrier-gated variant, so that `ring_spsc`
would perform an RMW it does not need and its measured numbers would converge
with `ring_mpsc`'s for reasons having nothing to do with ring design. Not
depending on them removes that risk structurally rather than mitigating it.

**What remains is the mirror image: the absences must stay absent.** A later
edit adding one back — for symmetry, or because a helper migrated there —
reintroduces exactly the hazard the removal eliminated, and it would still not
be visible in this crate's own tests, all of which would keep passing. Two
things catch it: the counting ordering shim asserting zero RMWs
(→ [No Lock in the Path](../invariant/002_no_lock_in_the_path.md)), and
`tests/manual/readme.md` S6, which greps the `[dependencies]` section for each
absent name and fails if one appears.

**S5 is a small but real contract question.** `RingConfig` carries
`producer_count` as a field. A config with `producer_count = 4` handed to this
crate must be rejected, not silently honoured as 1 — and the rejection belongs
at [`ring_factory`](../../../ring_factory/readme.md), which is what chooses
between the two backends. This crate's obligation is to state that it accepts
only 1.

### Error Handling

Nothing crosses these seams as an error value at runtime. The failure classes:

| Failure class | Example | How it surfaces |
|---------------|---------|-----------------|
| **Compile-time** | `ring_cursor` changes `PaddedCursor`'s API | Build failure — loud |
| **Silent performance** | An absent crate is added back and its contended claim path is called | Nothing fails. The SPSC/MPSC comparison quietly stops measuring cardinality. Caught only by the zero-RMW shim and by `tests/manual/readme.md` S6 |
| **Silent correctness** | A gating bound starts being computed via `ring_barrier` over a one-element set | Nothing fails at one producer; the crate acquires a dependency on machinery it declares no dependency on |
| **Silent misconfiguration** | A `producer_count > 1` config reaches this crate (S5) | **Data race.** The ring behaves as SPSC while multiple producers use it |
| **Composition** | `ring_core`'s crossbeam backend cannot express this surface | Build failure under `--features crossbeam` — loud, and caught by the dual-configuration run |

**The second row is the one this crate exists to prevent and the one it is
least able to detect.** Its whole contribution to the family is being the
configuration where cost is attributable; a dependency that reintroduces
contention destroys that contribution without breaking anything. The
corrected manifest makes it harder to reach by accident, not impossible —
which is why the grep in S6 exists rather than trusting the current five.

**The fourth row is the only one that corrupts data**, and its remedy is
entirely above this crate.

### Compatibility Requirements

1. **All four absences stay absent.** Adding any of them — for symmetry with
   `ring_mpsc`, or because a shared helper migrated there — reintroduces the
   machinery whose absence is this crate's thesis. If a check must be shared,
   it belongs in a form this crate can call without a set, a minimum, a CAS
   loop or a per-slot stamp.
2. **The claim, publish and commit steps are implemented in this crate**, which
   settles what an earlier draft left open. Calling a contended variant is not
   acceptable, and the alternative chosen is that the three steps are three
   stores and two loads written here, not a shared crate offering an
   uncontended mode.
3. **The surface stays expressible by an off-the-shelf queue.** This
   constrains the borrow-versus-copy decision in
   [Consumer Surface](../api/002_consumer_surface.md).
4. **`producer_count != 1` is rejected, not coerced** — at `ring_factory`, with
   this crate stating the requirement. What this crate does instead is read
   *only* `capacity` from a `RingConfig` and ignore every other field, which is
   the behaviour a test can actually observe here: silently coercing a
   `producer_count = 8` config into a working SPSC ring would be the failure,
   and building an identical ring to a plain config's is the evidence it does
   not happen.
5. **This crate is internal.** A signature change reaches `ring_core` and
   `ring_handle` and stops (→ [Reached Through the Export Surface](002_reached_through_the_export_surface.md)).
6. **Version movement is lockstep** — 33 path dependencies at one workspace
   version.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The steps S3, S4 and S6 supply, and the inline check that replaces `ring_gating` |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | The commit S5 supplies |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Requirement 3's constraint on the borrow decision |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | The fields S1 and S2 supply |

### Integrations

| File | Relationship |
|------|--------------|
| [002_reached_through_the_export_surface.md](002_reached_through_the_export_surface.md) | The seam above S7, and Requirement 5's blast radius |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_no_lock_in_the_path.md](../invariant/002_no_lock_in_the_path.md) | The structural enforcement this dependency list provides, and the shim that backs it |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_correctness_floor_for_the_family.md](../non_functional_requirement/001_correctness_floor_for_the_family.md) | What the Error Handling table's second row destroys |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) | The `CAPACITY` bound the absent `ring_gating` would otherwise enforce |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | Declares the five, and none of the four absences |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | S6 — the two-part dependency check: the five that must be declared, and the four that must not. The only detector for the Error Handling table's second row that runs without a benchmark |
| `tests/manual/readme.md` | S2 — zero read-modify-writes and zero locks in the path, established by reading the compiled surface rather than by timing it |
| `tests/spsc_test.rs` | `with_config_takes_the_capacity_and_ignores_the_rest` — this half of Requirement 4: a config carrying `producer_count = 8` and a non-default wait kind builds the same ring as a plain one, so no field is silently honoured here. The *rejection* itself is [`ring_factory`](../../../ring_factory/readme.md)'s and remains untested for now |

### SP18 — Five Dependencies Where the Sibling Has Eight

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_spsc ring_mpsc; do
  printf '%-11s %s deps: ' "$c" "$( grep -cE '^ring_[a-z_]+ = \{ workspace = true' $c/Cargo.toml )"
  grep -oE 'ring_[a-z]+ = \{ workspace' $c/Cargo.toml | sed 's/ = { workspace//' | tr '\n' ' '; echo
done
```

Live output:

```
ring_spsc   5 deps: ring_store ring_config ring_cursor ring_slot ring_types 
ring_mpsc   8 deps: ring_atomic ring_store ring_claim ring_config ring_cursor ring_gating ring_slot ring_types 
```

The difference is not a subset relationship by accident — each of the three
missing crates answers a question that collapses at cardinality one, which is the
argument the module documentation makes in full
(→ [`../pattern/002`](../pattern/002_absence_as_specification.md)).

### SP19 — The Crate Head Cites a Path That Is a Directory

> Historical record, not independently reproducible from a standalone checkout
> (the design corpus this probed, `docs/workstream/`, is external to this
> repository and unreachable since extraction): checking whether
> `008_ring_write_path.md` existed as a file and `008_ring_write_path` existed
> as a directory under that corpus's `docs/workstream/` returned "as a file:
> no" / "as a directory: yes".

One character — a `.md` where a `/readme.md` was meant. The same line appeared at
the head of every crate in the family, so this was one defect in thirty-three
files rather than thirty-three defects.

Not fixed here: it is a family-wide source correction, and this crate's docs are
not the place to make thirty-three of them. It has since been made under
`ring_cursor`'s CU48, which carries the applied disposition for all 33 crates;
the quoted result above still reads the same way because it measures the
workstream path's own shape, which did not change — only the citations to it did.
