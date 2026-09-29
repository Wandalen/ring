# Lifecycle: The Consolidation Cycle

### Scope

- **Purpose**: Take up the cycle `ring_tls` named and deferred — sealing, draining and resetting a staging region — with the trigger this crate supplies attached to it.
- **Responsibility**: State the phases, their transitions, their dependencies, and the cleanup each leaves behind.
- **In Scope**: One complete cycle from trigger to writable buffer; the abort path.
- **Out of Scope**: A policy's own lifetime (→ [From Configuration to the Final Drain](002_from_configuration_to_the_final_drain.md)); the epoch mechanism itself (→ [`ring_tls`'s Buffer Epoch Cycle](../../../ring_tls/docs/lifecycle/003_buffer_epoch_cycle.md)).

### Lifecycle Phases

[`ring_tls`'s consolidation cycle](../../../ring_tls/docs/lifecycle/002_consolidation_cycle.md)
names phases K1 and C2–C4 and states plainly that its K1 trigger belongs
elsewhere:

> "`ring_flush` existing is the reason this instance does not specify the …"

**This instance is the elsewhere.** The phases below are that cycle with K1
filled in.

| Phase | Name | Owner | What holds afterwards |
|-------|------|-------|----------------------|
| K1 | **Trigger** | **This crate** | A decision has been made: fire or not |
| C2 | Seal | `ring_tls`, called from here | Writer has a fresh region; sealed contents are the flusher's |
| C3 | Drain | Here, into the ring via `ring_batch` | Records are published and visible to the consumer |
| C4 | Reset | `ring_tls`, called from here | The sealed region is writable again |
| K5 | **Report** | This crate | The caller holds a [`FlushOutcome`](../type/002_flush_outcome.md); the log holds an entry |

**K1 and K5 are this crate's contribution and they bracket the rest.** The
middle three are `ring_tls`'s mechanism; without a trigger at the front and a
report at the back they are three primitives nobody sequences — which is
precisely the state `ring_tls`'s own docs describe.

### Phase Transitions

| From | To | Condition |
|------|----|-----------|
| — | K1 | The consumer calls `drive` or `drive_at_barrier` |
| K1 | C2 | The policy's trigger holds |
| K1 | K5 | The trigger does not hold → `NotTriggered`. **The common case** |
| C2 | C3 | Sealed region is non-empty |
| C2 | K5 | Sealed region is empty → `TriggeredEmpty` |
| C3 | C4 | The claim succeeded and records were copied |
| C3 | K5 | **The claim failed → `Rejected`. C4 is skipped** |
| C4 | K5 | Always |

**The C3 → K5 transition skipping C4 is the cycle's one irregularity and it is
mandatory.** Resetting after a failed claim discards records the caller is
about to be told were merely rejected — silent loss reported as backpressure
([`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md)'s O3). It
leaves the buffer sealed-and-unreset, which is a real state a writer can
encounter, not a transient.

**As built the irregularity was designed out rather than obeyed.** The claim is
checked before C2 runs, so a rejection transitions **K1 → K5 directly**, having
never sealed anything — there is no C3 → K5 edge and no skipped C4 to get wrong.
Restated against the implementation, the transition table's failure rows read:

| From | To | Condition, as built |
|------|----|---------------------|
| K1 | K5 | Trigger does not hold → `NotTriggered` (unchanged) |
| K1 | K5 | Nothing staged → `TriggeredEmpty` — this is C2's empty case, hoisted to before the seal |
| K1 | K5 | **Ring lacks room → `Rejected`.** The buffer is never touched |
| K1 | C2·C3·C4 | Room exists → one expression seals, drains and resets |

C2, C3 and C4 do not appear as separate edges because they are not separately
reachable: `try_push_batch( &mut buffer.drain() )` is all three, and the drain
iterator's own drop is C4. The phases stay in the table as the *shape* the
cycle has — `ring_tls`'s cycle is still what is being taken up — but as
implemented this crate never observes a moment between them.

**That is a stronger form of U1 than U1 asks for.** "Every seal is followed by a
drain or an explicit abort" is enforced by there being no statement in which
they could be separated, rather than by the function having no `await` or early
`?`.

**The K1 → K5 shortcut is the path taken on the overwhelming majority of
calls,** and it must be cheap for that reason — it is the only phase on the
per-append path (→ [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)).

**It is not on the per-append path as built, and that is cheaper still.** K1
runs on `drive`, not on `append`; a caller that stages a thousand records
between drives evaluates the policy once, not a thousand times.
`a_drive_that_does_not_fire_changes_nothing` pins the shortcut's cost from the
other direction — a thousand non-firing drives leave the buffer's capacity,
occupancy, the ring, and the log all unchanged.

### Dependencies

| Dependency | Phases | Why |
|------------|--------|-----|
| [`ring_tls`](../../../ring_tls/readme.md) | C2, C4 | Owns seal and reset; exposes them separately for this caller |
| [`ring_core`](../../../ring_core/readme.md) | C3 | The ring the drain publishes into |
| [`ring_batch`](../../../ring_batch/readme.md) | C3 | The contiguous claim, reached transitively through `ring_core` — **not declared here** |
| The consumer | K1 | Supplies the call, and on `OnBarrier` the fact (→ [The Driver Surface](../api/002_the_driver_surface.md)) |
| [`ring_overflow`](../../../ring_overflow/readme.md) | C3, on failure | **Not a dependency, and no longer needed for this.** With the claim checked before the seal, a failed claim has no cleanup question left; what remains is a policy over `append`'s refusal, which is a separate question (→ [`decisions/`](../decisions/readme.md)'s Pending 1) |

**`ring_batch`'s row is the one to notice.** The contiguous claim is central to
C3 — it is what makes a flush of `N` records cost one fence rather than `N` —
and this crate reaches it only transitively, through `ring_core`. Nothing here
declares the crate whose guarantee C3 depends on, and nothing tests the two
together (→ [`pitfall/002`](../pitfall/002_two_batch_sizes_that_must_not_diverge.md)).

### Cleanup Requirements

| # | Requirement | Enforced by |
|---|-------------|-------------|
| U1 | Every seal is followed by a drain or an explicit abort — never abandoned | The sequence being one non-suspending function; no `await`, no early `?` past C2 |
| U2 | A failed claim leaves records staged and recoverable | **Never claiming.** The capacity check precedes C2, so a rejection leaves the buffer in the state it was already in |
| U3 | The cycle leaves no allocation behind | **Structural argument, not measured** — see `FL29` below |
| U4 | A dropped `Flusher` does not flush | **Deliberate** — no `Drop` impl (→ [`pattern/002`](../pattern/002_driven_not_self_firing.md)) |
| U5 | Records staged at drop are lost unless `drain_final` ran | **Accepted, and it is the crate's sharpest edge** |

**U3's Enforced-by column names a structural argument, not a measurement.**
`log().is_none()` rules out the crate's one known allocation site, and this
crate ships no `#[ global_allocator ]` to observe an allocation directly. Four
sibling crates — `ring_barrier`, `ring_claim`, `ring_consume`, `ring_cursor` —
each carry one in their own `tests/allocation_test.rs`, so the instrument this
row needs exists inside the family and has simply not been wired to this
crate's tests.

**U4 and U5 are one decision seen from two sides, and it is the decision most
likely to be reversed by someone acting reasonably.** Flushing on drop would
"fix" U5 — and would put a publication point at a moment determined by drop
order ([the publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)'s
C2), fire outside any policy ([trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s
V5), and be invisible to a test that asserts and returns.

**The cost is real and is not argued away here:** a consumer that forgets
`drain_final` loses whatever was staged, silently. The design's answer is that
teardown is a phase with an explicit call
(→ [From Configuration to the Final Drain](002_from_configuration_to_the_final_drain.md)),
not that the loss does not matter.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_evaluating_a_policy_at_an_append.md](../algorithm/001_evaluating_a_policy_at_an_append.md) | K1 |
| [../algorithm/002_sequencing_seal_drain_reset.md](../algorithm/002_sequencing_seal_drain_reset.md) | C2–C4 and K5, with the ordering obligations |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) | `ring_batch`'s undeclared row, worked out |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_from_configuration_to_the_final_drain.md](002_from_configuration_to_the_final_drain.md) | The outer arc this cycle repeats inside; U5's mitigation |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_buffer_state_through_a_flush.md](../lifecycle/003_buffer_state_through_a_flush.md) | The same phases as buffer states, including sealed-and-unreset |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | K5's product |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_tls/docs/lifecycle/002_consolidation_cycle.md`](../../../ring_tls/docs/lifecycle/002_consolidation_cycle.md) | Names K1 and C2–C4 and defers the trigger here; this instance is that deferral taken up |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | `a_rejected_flush_leaves_the_policy_armed` takes the K1 → K5 rejection edge twice over, then lets the full cycle run and asserts the same four records arrive in the same order. `a_refused_final_drain_keeps_the_records` covers the shutdown path of the same edge. U5 — `dropping_a_driver_with_records_staged_publishes_nothing` asserts the sharp edge rather than describing it |
| `tests/append_cost_test.rs` | U3 — `an_unobserved_driver_never_acquires_a_log` runs sixteen full cycles and asserts `log()` is still `None`, which is the "only in instrumented builds" clause as an assertion |

### FL29 — U3 Says "No Allocation" and the Test It Cites Observes Whether a Log Exists

The requirement, its cited test, and what the test actually asserts:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- U3, and the row that cites its test --'
awk '/^### FL/{ exit } /^\| U3 \|/{ printf "    %s\n", substr( $0, 1, 118 ) }
     /^### FL/{ exit } /an_unobserved_driver_never_acquires_a_log/{ printf "    %s\n", substr( $0, 1, 118 ) }' \
  ring_flush/docs/lifecycle/001_the_consolidation_cycle.md
echo '  -- every assertion the test makes --'
awk '/^fn an_unobserved_driver_never_acquires_a_log/{ i = 1; next } i && /^\}/{ exit }
     i && /assert/ { printf "    %d: %s\n", NR, $0 }' ring_flush/tests/append_cost_test.rs
echo '  -- counting allocators in the ring family'\''s code --'
printf '    #[ global_allocator ] in ring_*/{src,tests}: %s\n' \
  "$( command grep -rl 'global_allocator' ring_*/src ring_*/tests 2>/dev/null | wc -l )"
echo '  -- which crates carry one, and whether this one is among them --'
command grep -rl 'global_allocator' --include='*.rs' 2>/dev/null | sed 's/^/    /' | LC_ALL=C sort
```

Live output:

```
  -- U3, and the row that cites its test --
    | U3 | The cycle leaves no allocation behind | **Structural argument, not measured** — see `FL29` below |
    | `tests/append_cost_test.rs` | U3 — `an_unobserved_driver_never_acquires_a_log` runs sixteen full cycles and asserts 
  -- every assertion the test makes --
    170:     assert_eq!( flusher.drive(), FlushOutcome::Flushed { count : 4 } );
    171:     assert!( flusher.log().is_none(), "a driver acquired a log it was never given" );
    175:   assert_eq!( consumer.try_recv_batch( &mut landed ), 64 );
    176:   assert_eq!( landed.len(), 64, "sixty-four records were staged and flushed" );
  -- counting allocators in the ring family's code --
    #[ global_allocator ] in ring_*/{src,tests}: 4
  -- which crates carry one, and whether this one is among them --
    ring_barrier/tests/allocation_test.rs
    ring_claim/tests/allocation_test.rs
    ring_consume/tests/allocation_test.rs
    ring_cursor/tests/allocation_test.rs
```

U3 requires that a consolidation cycle leave no allocation behind. The test cited
for it runs sixteen cycles and asserts, sixteen times, that `log()` is `None` —
plus that sixty-four records were drained and that sixty-four landed. Three kinds
of assertion, and not one of them observes an allocation.

**The proxy is a good one and it is not the claim.** `log().is_none()` establishes
that the `Option< FlushLog >` was never populated, which rules out the crate's
*known* allocation site. It says nothing about the `Vec` inside a claim, about
`ring_batch`, or about any allocation introduced later by a path the test does
not run. U3's Enforced-by column reads as though the requirement were measured;
what is measured is one field's emptiness.

**Four `#[ global_allocator ]` declarations exist in the ring family, and none
of them is in this crate.** `ring_barrier`, `ring_claim`, `ring_consume` and
`ring_cursor` each carry one in their own `tests/allocation_test.rs`. So U3's
gap is not that the family cannot observe an allocation — four of its crates
can — but that the crate making the claim has not adopted the instrument its
siblings already built.

**This paragraph asserted the opposite until it was corrected, and the recording
directly above it had been refuting it for days.** The text here said no
`#[ global_allocator ]` existed anywhere in the ring family, while the evidence
block in this same section counted four. The probes landed on 2026-09-04; the
recipe was regenerated and reported the new number faithfully, and the prose
beneath it was never re-read. When a document's own evidence disagrees with
its conclusion, the evidence is the half that was measured.

The reusable shape: **a proxy assertion inherits the wording of the property it
stands in for.** Nothing in the row says "as far as the log is concerned," so a
reader checking U3 finds a test, sees it pass, and stops — which is the correct
response to a row that does not distinguish measuring a thing from measuring the
one part of it that was easy.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
command grep -m1 'not a measurement' ring_flush/docs/lifecycle/001_the_consolidation_cycle.md
```

Live output:

```
**U3's Enforced-by column names a structural argument, not a measurement.**
```

**Disposition:** applied — U3's Enforced-by cell now says "Structural
argument, not measured" instead of naming the test as though it observed an
allocation, and a new paragraph states precisely what `log().is_none()`
does and does not rule out. Wiring an external allocation probe
into this crate's tests would be the thorough fix; it is a new test
dependency and instrumentation decision, not a docs-corpus correction, so it
is named here and left undone. Now prints: `not a measurement`

**Correction (2026-09-29):** the census this section drew on originally
scanned outside the ring family, into the private monorepo `ring` was
developed inside, and the remediation suggestion above named a specific
external crate's allocation probe by path. `ring` has since been extracted
into its own standalone repository, which cannot see that monorepo or name
that crate, so the recipe above is now scoped to `ring_*/` alone — the four
in-family `#[ global_allocator ]` sites remain the complete, accurate answer
to "which crates carry one," and the suggested fix is now simply "adopt the
same instrument the four sibling crates already use," with no external
reference required.
