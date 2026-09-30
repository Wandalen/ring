# Integration: Family Dependency Seam

### Scope

- **Purpose**: State what this crate composes from its eight declared dependencies and what it contributes itself, so that "the ring" is not read as a monolith when it is an assembly.
- **Responsibility**: Name each dependency's contribution, the direction of each integration point, the export boundary above this crate, and the assumptions across each seam — three of which were measured in a later revision and found not held by anything.
- **In Scope**: The eight declared dependencies, the family's five-crate export surface, and this crate's position between them.
- **Out of Scope**: What a consumer outside the family does with the result (→ [Prospective Consumer Adoption](002_prospective_consumer_adoption.md)); the mechanism each dependency implements, which is that crate's own documentation.

### System Description

`ring_mpsc` is a composition point, not a leaf. Its `Cargo.toml` declares
eight dependencies, and the crate's own contribution is the **multi-producer
policy** binding them: many claimants against one consumer, sequence-stamped,
totally ordered. The generic ring machinery — buffers, cursors, claim,
gating — is not this crate's; the decision that claims are contended and
consumption is not is.

**The publication half, however, *is* this crate's, and that is a change from
what this instance originally recorded.** The manifest was scaffolded with
seven path dependencies before any of them existed, and this instance
documented what each would contribute. In a later revision the crate actually called them,
and three of those contributions turned out not to exist — measured directly
and ruled on at the family level. `ring_publish` and `ring_consume` are gone;
`ring_atomic`, `ring_slot` and `ring_types` are new. The seam table below is
the corrected one; the paragraph after it records what was wrong, because
that is the more useful artifact than a table that was quietly fixed.

This is a change from the crate's original shape and worth stating because
the earlier shape is still documented elsewhere.
[`../../readme.md`](../../readme.md) records it: this crate was originally
filed as a standalone leaf with no dependencies, and a later, family-wide
dependency forest superseded that filing. A reader arriving from that
original framing expects a self-contained ring implementation and will not
find one.

| Layer | Crates | This crate's relationship |
|-------|--------|---------------------------|
| **Above** — external surface | `ring_factory`, `ring_handle`, `ring_tls`, `ring_flush`, `ring_types` | This crate is **not** among them; it is internal to the family and reaches the outside only through what these re-export |
| **This crate** | `ring_mpsc` | Owns the MPSC policy: contended claim, single-consumer drain, sequence-stamped slots |
| **Below** — composed from | `ring_atomic`, `ring_store`, `ring_claim`, `ring_config`, `ring_cursor`, `ring_gating`, `ring_slot`, `ring_types` | Supplies the mechanism this crate arranges — *not* the publication mechanism, which this crate owns |

**Being internal is the design's main freedom, and it is easy to miss.**
Because no external caller names `ring_mpsc`, every open question in this
crate's other instances — stamp width, capacity constraint, guard-versus-
three-calls, borrow-versus-copy — could be settled *after* consumers exist,
without a breaking change to anything outside the family. The five exported
crates absorb it. That was a structural argument rather than procrastination,
and it is worth keeping on the record now that a later revision has cashed it out: those
questions were all closed at implementation time, each in its own instance,
and none of the choices needed a consumer to justify it. The freedom was real
and went unused — which is the good outcome, not a wasted argument. It is
still there for the next question.

### Integration Points

| # | Seam | Direction | What crosses it | Assumption made |
|---|------|-----------|-----------------|-----------------|
| I1 | [`ring_store`](../../../ring_store/readme.md) | This crate → | Slot array allocation, indexed access, release | That indexed access is unchecked in release and does not bounds-check per element; a checked index on the drain's hot loop would be a per-element branch |
| I2 | [`ring_cursor`](../../../ring_cursor/readme.md) | This crate → | Both cursors, their atomics, and their cache-line separation | **That the separation is `ring_cursor`'s to guarantee.** This crate's [data structure](../data_structure/001_sequence_stamped_ring.md) states padding as a *contract*, but the field lives in the dependency — so the contract is stated here and enforced there, and nothing currently checks that it is |
| I3 | [`ring_claim`](../../../ring_claim/readme.md) | This crate → | The contended allocation of a sequence, fused with the free-capacity check | That the fusion is `AcqRel` — `Claimer::claim` is a `compare_exchange` loop, *not* the fetch-add this row originally assumed, because a bounded ring's headroom check cannot be fused into a fetch-add. Admissible (→ [Publication Ordering](../invariant/002_publication_ordering.md) blesses a fused claim); it costs the wait-free adjective, not correctness |
| I4 | [`ring_atomic`](../../../ring_atomic/readme.md) | This crate → | `AtomicSeq` — the *unpadded* sequence cell one stamp array element is made of | That it is unpadded. `PaddedCursor` is 64-byte aligned, which for a per-slot stamp array would cost 64× the payload array. Also that under `--cfg loom` it swaps in an instrumented cell, which is what makes the `exhaustive` model explore this crate's protocol rather than a re-implementation |
| ~~I5~~ | ~~`ring_consume`~~ | — | — | **Retired in a later revision.** `Consumer::new` requires a `ring_barrier::Barrier` over a `&[PaddedCursor]`, so it can only gate on a *cursor* frontier — which a stamped ring does not have. The ID is not reused |
| I6 | [`ring_gating`](../../../ring_gating/readme.md) | This crate → | `slowest()`/`headroom()` over the **consumer** cursors — the backpressure bound | That it bounds the claim, which it does. This row originally said "published-watermark computation"; `GatingSet` runs the opposite direction and computes no watermark at all. The first-gap publication scan is this crate's own `contiguous_end`, not a seam |
| I7 | [`ring_config`](../../../ring_config/readme.md) | This crate → | Capacity and policy parameters at construction | That capacity arrives as a runtime value, which is what makes const-generic capacity (→ [Capacity](../type/002_capacity.md)) the road not taken |
| I8 | [`ring_handle`](../../../ring_handle/readme.md) | → This crate | Whatever handle shape it exports wrapping this crate's surface | That the handle's lifetime relationship to the ring is decided *there*, which is what makes [Ring Construction and Teardown](../lifecycle/001_ring_construction_and_teardown.md)'s forbidden L5 safe or unsafe |
| I9 | [`ring_slot`](../../../ring_slot/readme.md) | This crate → | The `Slot` bound `Buffer< S >` requires, and `TypedSlot`/`BytesSlot` as the payload shapes tests instantiate | That `Slot::clear` leaves a slot `Default`-equivalent — which is what makes a claim dropped without a write publish an *empty* record rather than a stale one (→ [Producer Publish Surface](../api/001_producer_publish_surface.md)) |
| I10 | [`ring_types`](../../../ring_types/readme.md) | This crate → | `Seq`, `Capacity`, `RingError` | That `Seq` is a plain `u64` newtype with wrapping arithmetic, so `UNSTAMPED = Seq( u64::MAX )` is a value a real sequence reaches only after 2⁶⁴ publications (→ [Sequence Number](../type/001_sequence_number.md)) |

**I2 through I6 were all the same shape and the same risk: this crate stated a
property that another crate must actually provide.** Cache-line separation,
fetch-add rather than CAS, `Release` rather than `Relaxed`, plain store rather
than compare-exchange, first-gap watermark rather than highest-published — five
correctness- or performance-critical properties, each documented here and
implemented elsewhere. This instance's original text then said: "Every one of
them can be violated by a sibling change that looks locally reasonable, and
none of them is currently checked by anything."

**Three of the five were not violated by a later change. They were never true.**
This is the first revision that calls these siblings, so it is the first time the
assumptions were checkable rather than merely stated. Checked against each
dependency's source:

| Seam | Assumed | Actually |
|------|---------|----------|
| I2 `ring_cursor` | Cache-line separation | **Held.** `CursorPair::on_distinct_lines` exists, and `the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines` now asserts it from this crate |
| I3 `ring_claim` | A genuine fetch-add, not a CAS loop | **Not held.** It is a CAS loop, by construction — the free-capacity check is fused into the claim. Admissible, but the wait-free adjective built on it is not |
| I4 `ring_publish` | "The stamp store and its `Release` ordering" | **Not held — the machinery does not exist.** `Publisher` holds one `PaddedCursor` and no per-slot state anywhere. `ring_publish` declines this crate's problem *in writing*, calling it "`ring_mpsc`'s problem at S5" |
| I5 `ring_consume` | The drain walk and cursor advance | **Not usable.** Gates on a cursor frontier a stamped ring does not have |
| I6 `ring_gating` | Published-watermark computation | **Not held.** Computes the slowest *consumer*, which runs the opposite direction from a publication watermark |

The lesson is not "check your assumptions" — it is more specific than that.
**Every one of the three that failed was an assumption about a crate that did
not exist yet when the assumption was written.** I2, which was about a crate
that already existed, held. The scaffolded manifest and the scaffolded seam
table were written in the same act and were wrong in the same way, and the
ruling on it at the family level records that this is the fourth scaffolded
manifest found wrong — and the first found to *under*-declare as well as
over-declare, which is the harder
direction to notice, since an unused dependency is at least visible as a name
nothing mentions.

**This is the concrete cost of the 33-crate decomposition**, and it is still
not an argument against it — it is the item decomposition adds to the test
plan, and the mitigation still holds: each surviving property is testable from
this crate, and the tests below now assert them across the seam rather than
trusting it.

**I8 runs the other direction and is the only one that does.** Everything
else is this crate consuming a sibling; `ring_handle` consumes this crate.
Doc references run in both directions across a crate boundary once real, so
this row will eventually be matched by a row in `ring_handle`'s own
integration docs — it is not there yet, and this instance is the first half
of a pair.

### Error Handling

Nothing crosses these seams as an error value; every seam is a direct call.
The failure modes are therefore compile-time or silent, and they divide
cleanly:

| Failure class | Example | How it surfaces |
|---------------|---------|-----------------|
| **Compile-time** | A dependency changes a signature | Build failure — loud, fine, no discussion needed |
| **Silent correctness** | The publication ordering weakened to `Relaxed` | Nothing fails to build; the ring publishes torn payloads under contention on weakly-ordered hardware, and would pass every test on x86 |
| **Silent performance** | I2's padding removed | Nothing fails; throughput degrades under producer count, which is exactly the axis a benchmark run at low producer count would miss |

**The middle row is the one that required a real answer, and it now has one.**
Its example no longer names I4, because publication is not across a seam any
more — it is `PUBLISH` in this crate's own `src/lib.rs`, which is a strict
improvement: the ordering and the payload write it releases are now in one
file, reviewable together.

The original reasoning here was that a `Relaxed` regression is invisible on
x86 and appears only on ARM, "a target this workspace's browser and mobile
ambitions make live rather than hypothetical." **That premise was wrong in the
direction of caution: this workspace's own host is
`aarch64-unknown-linux-gnu`** (check with `rustc -vV | grep host`). ARM is not
a future target to be validated on eventually; the development machine is one.

So the mutation was run rather than reasoned about. Weakening `PUBLISH` to
`Relaxed` and running the parity test sixty times: **14 failures out of 60**,
with `sequence Seq(2234) was drained as published but its slot was empty`.
Restored to `Release`: 60 of 60 pass. The loom model fails deterministically
under the same mutation, in 0.010s.

Both halves of the answer this row asked for therefore exist — an explicit
assertion at the site, and a model-checker run — and neither had to wait for
the family's own harness.
The 23% hardware detection rate is why the loom half is not optional: three
runs in four would still report green against a genuinely broken publish.

### Compatibility Requirements

1. **This crate may not be depended on from outside the family.** The export
   surface is `ring_factory`, `ring_handle`, `ring_tls`, `ring_flush`,
   `ring_types`. A crate outside `ring_*` taking a direct `ring_mpsc`
   dependency has bypassed the boundary that makes every open question above
   non-breaking, and defeats it for everyone.
2. **The surviving cross-seam properties are requirements on the dependencies,
   not preferences.** I2's cache-line separation, I3's `AcqRel` fusion, I9's
   `Default`-equivalent clear, I10's wrapping `Seq` — each is load-bearing for
   a stated contract in this crate, and a sibling that changes one has made a
   breaking change to this crate without touching its signature. The list is
   shorter than it was: publication moved *into* this crate, which removes a
   seam rather than documenting one.
3. **Version movement within the family is lockstep.** All 33 crates are
   path dependencies within one workspace at one version; there is no
   semver negotiation between them and no intent to support mixed versions.
4. **`ring_registry` is deliberately absent.** This crate takes no producer
   registry (→ [Producer Attachment and Detachment](../lifecycle/002_producer_attachment_and_detachment.md)'s
   Cleanup Requirement 2), and adding one later would be a behaviour change
   for anything relying on detachment being free.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_publish_surface.md](../api/001_producer_publish_surface.md) | Its lock-free guarantee rests on I3, whose CAS loop is why the guarantee is lock-free and not wait-free |
| [../api/002_consumer_drain_surface.md](../api/002_consumer_drain_surface.md) | Its sub-atomic drain cost is this crate's own, after I5's retirement — valid only under this crate's single-consumer policy |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | States the padding contract that I2 shows lives in a different crate than the statement |

### Integrations

| File | Relationship |
|------|--------------|
| [002_prospective_consumer_adoption.md](002_prospective_consumer_adoption.md) | The seam above the export surface, where this instance's boundary argument is cashed out |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_publication_ordering.md](../invariant/002_publication_ordering.md) | The ordering I4 must actually supply; the invariant is stated here and enforced there |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_ring_construction_and_teardown.md](../lifecycle/001_ring_construction_and_teardown.md) | Its Dependencies table lists the same seams phase by phase, including `ring_shutdown`, which is a lifecycle seam rather than a compile-time dependency |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_capacity.md](../type/002_capacity.md) | Its const-generic-versus-runtime question is decided by I7's direction |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | Declares the eight dependencies I1–I4 and I6–I10 cross, with a comment recording the two removed and three added |
| `src/lib.rs` | Crate root; owns the stamp array, `PUBLISH`, and `contiguous_end` — the machinery I4 and I6 were assumed to supply |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines` | I2's contract, asserted from this crate by address arithmetic rather than trusted — the one seam of the five that held |
| `tests/mpsc_test.rs::the_drain_stops_at_the_first_unpublished_sequence_not_the_highest_published` | I6's property, now checked where it actually lives: this crate's `contiguous_end`, not `ring_gating`. Its violation is the one that would silently break total order |
| `tests/mpsc_test.rs::the_orderings_are_the_ones_the_publication_invariant_names` | The former I4, asserted at the site now that it is a site rather than a seam |
| `tests/mpsc_test.rs::exhaustive::a_published_record_is_never_observed_before_the_write_that_preceded_it` | The `--cfg loom` half, which is what makes I4's ordering checkable rather than sampled at 23% |

### MP18 — Eight Path Dependencies and No External Crate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -cE '^ring_[a-z_]+ = \{ workspace = true' Cargo.toml
grep -cE '^[a-z-]+ = \{ version' Cargo.toml
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
8
0
```

Eight and zero. That is the property `non_functional_requirement/001` depends on
when it says a measurement here is a measurement of this family's own code, and
it is the reason the workaround readme's original "None" was superficially
plausible — no external dependency does mean no external constraint of the
usual kind. The language itself is still external, which is where the actual
workaround comes from
(→ [`../workaround/001`](../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md)).

### MP19 — Twelve `use` Declarations for Eight Dependencies

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -E '^use ' src/lib.rs
```

Live output:

```
use core::cell::{ Cell, UnsafeCell };
use core::marker::PhantomData;
use core::ops::{ Deref, DerefMut };
use core::sync::atomic::Ordering;
use ring_atomic::{ AtomicSeq, SeqCell };
use ring_store::Buffer;
use ring_claim::Claimer;
use ring_config::RingConfig;
use ring_cursor::{ PaddedCursor, GATING };
use ring_gating::GatingSet;
use ring_slot::{ Slot, TypedSlot };
use ring_types::{ Capacity, RingError, Seq };
```

Reading the list is the point: it shows at a glance which siblings this crate
leans on for one thing and which for several. `ring_types` and `ring_cursor`
each contribute multiple names; `ring_gating` contributes one.

That asymmetry is the seam's real shape, and a dependency count alone hides
it.
