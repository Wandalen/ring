# Integration: One Dependency and the Backends Beneath It

### Scope

- **Purpose**: Account for this crate's single declared dependency and for what that one name conceals — three interchangeable ring implementations whose contracts differ in ways this crate's uniform surface cannot express.
- **Responsibility**: The seam, what crosses it, the assumption it carries, and the failure classes it admits.
- **In Scope**: The `ring_core` dependency; the backends beneath it; the crates that reach this one from above.
- **Out of Scope**: The export boundary (→ [On the Export Surface](002_on_the_export_surface.md)); each backend's own mechanism.

### System Description

`ring_handle` declares **one** dependency:

```toml
[dependencies]
ring_core = { path = "../ring_core" }
```

**One dependency is a low number for a crate on the export surface, and it is
achieved by delegation rather than by simplicity.** Everything a handle can do,
`ring_core` does; everything `ring_core` does, one of three backends does.

| Layer | Crates | This crate's relationship |
|-------|--------|---------------------------|
| **Above** | [`ring_factory`](../../../ring_factory/readme.md), [`ring_poll`](../../../ring_poll/readme.md), [`ring_flush`](../../../ring_flush/readme.md), [`ring_shutdown`](../../../ring_shutdown/readme.md) | Construct handles, loop over them, decide when to publish, close the ring |
| **This crate** | `ring_handle` | Partitions capability. Adds no mechanism |
| **Below** | `ring_core` | The composition point |
| **Beneath that** | [`ring_spsc`](../../../ring_spsc/docs/readme.md), [`ring_mpsc`](../../../ring_mpsc/docs/readme.md), a `crossbeam` queue | Three backends, selected by `ring_core`'s own backend-swap build flag |

#### What one dependency conceals

**The three backends do not have identical contracts, and this crate presents
one surface over all of them.** That is the seam's central problem and it is not
solvable by anything in this crate.

| Property | `ring_spsc` | `ring_mpsc` | `crossbeam` |
|----------|------------|-------------|-------------|
| Producers permitted | Exactly 1 | N | N |
| `free_capacity()` | **Binding** — `n` reported means `n` pushes will succeed | **Advisory** — another producer may take the room | Advisory |
| Drain order | The exact publication sequence | A multiset with per-producer order | Implementation-defined |
| RMW in the publish path | **Zero** | Present | Present |

**`free_capacity()`'s row is the concrete hazard.** One signature on
[`Producer`](../api/001_producer_surface.md), two contracts, and the caller
cannot see which one it has — the backend was chosen by `ring_factory` from a
config field the caller may not have set. Code written against an SPSC ring that
relies on the binding form breaks when the same code is handed an MPSC ring,
with no signature change and no compiler error.

**The drain-order row is worse in the same way and better in one respect:** it
is worse because ordering assumptions are made silently and everywhere; it is
better because [`ring_spsc`'s own docs](../../../ring_spsc/docs/pitfall/001_spsc_correctness_does_not_transfer.md)
already treat this as a named pitfall with seven properties that fail to
transfer. This crate is the surface across which that transfer happens.

**The RMW row is where this crate can do damage rather than merely permit it.**
`ring_spsc` asserts zero read-modify-writes across a run
(→ [its no-lock invariant](../../../ring_spsc/docs/invariant/002_no_lock_in_the_path.md)),
and every publish reaches it through a `Producer` method. A counter added here
— in the crate that is not in `ring_spsc`'s dependency list — makes that
assertion fail with nothing in `ring_spsc`'s own dependency tree to blame
(→ [Delegating an Operation to the Backend](../algorithm/002_delegating_to_the_backend.md)).

### Integration Points

| # | Seam | Direction | What crosses it | Assumption made |
|---|------|-----------|-----------------|-----------------|
| A1 | [`ring_core`](../../../ring_core/readme.md) | This crate → | Every publish, drain, and liveness read | That its surface is uniform across backends, and that this crate may present it unchanged |
| A2 | [`ring_factory`](../../../ring_factory/readme.md) | → This crate | A validated `RingConfig`, already spent | That `producer_count` mismatches were rejected **there** — this crate cannot see the field |
| A3 | [`ring_poll`](../../../ring_poll/readme.md) | → This crate | The loop that retries after a refusal | That the non-parking constraint is satisfied by this surface, which `ring_poll` claims and this crate must deliver |
| A4 | [`ring_shutdown`](../../../ring_shutdown/readme.md) | ↔ | `close()`, `reset()`, `drain_all()`, and the liveness flag both handles read | That the flag is authoritative and singular — no handle-local copy (→ [Ring Liveness Through a Handle](../lifecycle/004_ring_liveness_through_a_handle.md)) |
| A5 | [`ring_flush`](../../../ring_flush/readme.md) | → This crate | Publish calls timed by a policy | That flushing is a policy above the handle, not a handle method |
| A6 | [`ring_stats`](../../../ring_stats/readme.md) | **Absent** | — | That instrumentation lives there and not in a forwarding method |
| A7 | [`ring_batch`](../../../ring_batch/readme.md) | **Absent** | — | That batch buffering lives there, where the buffer's lifetime is explicit |

**A6 and A7 are the two conspicuous absences, and both are absences from a hot
path that runs through this crate.** Stats and batching are exactly the things
one adds at a chokepoint, this crate is the chokepoint, and both belong
elsewhere. Recording them as seams-that-are-not is more useful than omitting
them, because the argument for adding them is made at this crate's boundary.

**A2 is a one-way seam with no back channel.** By the time a handle exists the
config is spent. A ring built for four producers, handed out as one `Producer`,
is indistinguishable here from a correct SPSC pair — same types, same size, same
methods (→ [`ring_spsc` integration/001](../../../ring_spsc/docs/integration/001_family_dependency_seam.md)).

**A3's obligation runs opposite to its dependency arrow, which is why it is easy
to lose.** `ring_poll` depends on this crate and *claims* the row for the
non-parking constraint; this crate is only named in that row and claims
nothing itself. So the constraint is imposed
downward while the test lives upward
(→ [Nothing Reachable From a Handle Can Park](../invariant/002_no_parking_operation_is_reachable.md)).

### Error Handling

| Failure class | Example | How it surfaces |
|---------------|---------|-----------------|
| **Compile-time** | `ring_core` changes an operation's signature | Build failure — loud |
| **Silent contract drift** | A caller relies on `free_capacity()`'s binding form and is handed an MPSC ring | **Nothing fails.** Occasional refused pushes that look like ordinary contention |
| **Silent ordering assumption** | Code validated against `ring_spsc` runs against `ring_mpsc` | Nothing fails until a reordering matters; the seven-property table in `ring_spsc`'s pitfall enumerates the ways |
| **Silent misconfiguration** | A `producer_count > 1` config produced an SPSC backend (A2) | **Data race**, above this crate and invisible here |
| **Silent cost** | A counter added in a forwarding method (A6) | `ring_spsc`'s zero-RMW assertion fails, pointing at a crate that is not in its dependency tree |
| **Cross-crate constraint loss** | A blocking method added here (A3) | Deadlock in a tick; caught only if `ring_poll`'s compile-fail case targets this surface |

**Five of six rows are silent, and that is the honest summary of this seam.**
The one loud failure is a signature change. Everything else this crate can get
wrong, it gets wrong quietly, in another crate, at a distance from the edit.

### Compatibility Requirements

1. **The handle surface stays uniform across all three backends.** A method that
   only makes sense for one backend either goes on a backend-specific type or
   does not exist. `ring_core`'s own requirement that "the identical test suite" pass
   against both backends is the enforcement.
2. **`free_capacity()` is documented as a lower bound**, since that is the
   weakest of the three contracts and the only one true of all of them
   (→ [Producer Surface](../api/001_producer_surface.md)'s guarantee 3).
3. **No forwarding method acquires state or an atomic operation** — A6's and A7's
   absences, stated as a requirement rather than an observation.
4. **The liveness flag is read, never cached** (A4).
5. **`producer_count` validation stays at `ring_factory`** (A2), with this crate
   documenting that it cannot check it.
6. **No type in this crate's public surface names a crate outside the five-name
   export list** (→ [On the Export Surface](002_on_the_export_surface.md)).
7. **Version movement is lockstep** across the family's 33 crates.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | What crosses A1 per operation, and requirement 3's forbidden additions |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | Where the `free_capacity()` contract split is visible to a caller |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Where the drain-order difference is visible — and where it is not |

### Integrations

| File | Relationship |
|------|--------------|
| [002_on_the_export_surface.md](002_on_the_export_surface.md) | The seam above, and requirement 6's origin |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_no_parking_operation_is_reachable.md](../invariant/002_no_parking_operation_is_reachable.md) | A3's constraint, and why its detector lives in another crate |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_split_move_and_drop.md](../lifecycle/001_split_move_and_drop.md) | A2 and A4 as ordering dependencies rather than seams |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_liveness_through_a_handle.md](../lifecycle/004_ring_liveness_through_a_handle.md) | The states A4 drives |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | Declares exactly one dependency |
| [`ring_core/docs/non_functional_requirement/001_backend_swap_is_a_build_flag.md`](../../../ring_core/docs/non_functional_requirement/001_backend_swap_is_a_build_flag.md) | The three backends, and requirement 1's identical-suite enforcement |
| [`ring_factory/readme.md`](../../../ring_factory/readme.md) | A2's validation, which happens before a handle exists |
| [`ring_poll/readme.md`](../../../ring_poll/readme.md) | `ring_core` as the composition point (A1), and this crate's obligation to `ring_poll`'s non-parking claim (A3) |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `the_in_house_backends_behave_alike`, `the_crossbeam_backend_behaves_alike` | The same handle test passes against every `ring_core` backend — requirement 1, which is the only mechanism that keeps the surface uniform. Covering the third needed a `crossbeam` feature on *this* crate, forwarded to `ring_core/crossbeam`; without it requirement 1 would have been tested against two backends out of three and reported as met. What actually crosses backends is five assertions, covering one of the four properties the integration table lists as divergent — `free_capacity()`'s binding-vs-advisory split, the producer-count limit, and RMW counts are asserted nowhere in the suite (→ HD21) |
| **not written** | Requirement 3 — a publish through `Producer` performs no atomic operation the backend call does not. Asserting it needs a counting shim substituted for the real backend, and `ring_core` exposes no seam to substitute at: `Backend` is a private enum selected inside `Ring::new`. The nearest available evidence is `the_wrapper_costs_nothing`, which measures that the handle adds no *state* — a weaker claim, since a wrapper of the same size can still issue an extra fence |

### HD21 — "The Identical Test Suite" Is Two Tests, and One of Them Is Feature-Gated Off

Requirement 1's enforcement is `ring_core`'s own clause that the identical test
suite pass against every backend. Measure what actually crosses backends:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- suite size, and how much of it reaches a non-default backend --'
printf '  total #[ test ]:            %s\n' \
  "$( command grep -c '^#\[ test \]' ring_handle/tests/handle_test.rs )"
printf '  naming new_crossbeam:       %s\n' \
  "$( command grep -c 'new_crossbeam' ring_handle/tests/handle_test.rs )"
printf '  behind #[ cfg( feature ) ]: %s\n' \
  "$( command grep -c 'cfg( feature = "crossbeam" )' ring_handle/tests/handle_test.rs )"
echo '  -- and what the two parity tests actually assert --'
sed -n '/^fn the_in_house_backends_behave_alike/,/^}/p' ring_handle/tests/handle_test.rs \
  | command grep -oE 'assert[_a-z]*!\( [a-z_.]+' | sed 's|^|    |'
```

Live output:

```
  -- suite size, and how much of it reaches a non-default backend --
  total #[ test ]:            19
  naming new_crossbeam:       2
  behind #[ cfg( feature ) ]: 1
  -- and what the two parity tests actually assert --
    assert_eq!( producer.try_push_batch
    assert!( producer.is_full
    assert_eq!( producer.try_push
    assert_eq!( drained
    assert!( consumer.is_empty
```

Eighteen tests; two touch a non-default backend; one of those two compiles only
under `--features crossbeam`. The parity they establish is five assertions —
batch push, full, refusal-hands-back, drain order, empty.

**Five assertions is not the identical test suite, and the difference is exactly
where the table above says the backends differ.** The contract table names four
divergent properties. Drain order is asserted. The other three are not: nothing
tests that `free_capacity()` is binding on one backend and advisory on another,
nothing tests the producer-count limit, and nothing counts RMWs. The suite
checks the property the backends agree on and skips the three the table exists
to warn about.

The default `cargo test -p ring_handle` run is narrower still: with `crossbeam`
off, one of the three backends is never constructed at all, and requirement 1
reports met on a two-of-three sample. The Tests row above already says this
about the third backend and stops one step short of the more useful sentence —
that even the two it does cover are compared on the axis where divergence was
never expected.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
command grep -m1 '^| \[`tests/handle_test.rs`\]' ring_handle/docs/integration/001_one_dependency_and_the_backends_beneath.md
```

Live output:

```
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `the_in_house_backends_behave_alike`, `the_crossbeam_backend_behaves_alike` | The same handle test passes against every `ring_core` backend — requirement 1, which is the only mechanism that keeps the surface uniform. Covering the third needed a `crossbeam` feature on *this* crate, forwarded to `ring_core/crossbeam`; without it requirement 1 would have been tested against two backends out of three and reported as met. What actually crosses backends is five assertions, covering one of the four properties the integration table lists as divergent — `free_capacity()`'s binding-vs-advisory split, the producer-count limit, and RMW counts are asserted nowhere in the suite (→ HD21) |
```

**Disposition:** applied — the Tests table's `handle_test.rs` row now names
the five assertions the parity tests actually make and states that they
cover one of the four divergent properties the integration table lists,
leaving `free_capacity()`'s contract split, the producer-count limit, and
RMW counts unasserted anywhere in the suite.
Now prints: `covering one of the four properties the integration table lists as divergent`

### HD22 — A4 Is the Table's Only Bidirectional Seam and It Has No Edge in Either Direction

The Integration Points table gives `ring_shutdown` a `↔` arrow, four operations,
and a shared flag:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- A4, from both ends --'
printf '  ring_handle -> ring_shutdown in Cargo.toml: %s\n' \
  "$( command grep -c ring_shutdown ring_handle/Cargo.toml )"
printf '  ring_shutdown -> ring_handle in Cargo.toml: %s\n' \
  "$( command grep -c ring_handle ring_shutdown/Cargo.toml )"
printf '  ring_shutdown naming ring_handle in src:    %s\n' \
  "$( command grep -rc ring_handle ring_shutdown/src/lib.rs )"
echo '  -- what ring_shutdown actually wraps --'
command grep -E '^use ring_core' ring_shutdown/src/lib.rs
```

Live output:

```
  -- A4, from both ends --
  ring_handle -> ring_shutdown in Cargo.toml: 0
  ring_shutdown -> ring_handle in Cargo.toml: 0
  ring_shutdown naming ring_handle in src:    0
  -- what ring_shutdown actually wraps --
use ring_core::{Consumer, Producer};
```

Zero in all three directions. `ring_shutdown` imports `ring_core::{ Consumer,
Producer }` — the backend's handles, not this crate's.

**The table marks six seams with a direction and A4 is the only `↔`, which reads
as the strongest coupling in the list.** It is the weakest: there is no edge.
`close()`, `reset()` and `drain_all()` do not cross this seam because the seam
is not there, and the flag "both handles read" is not readable from either
handle — that is the whole subject of
[`decisions/002`](../decisions/002_why_is_closed_is_absent.md), which rules the
dependency out on parking grounds.

Two instances in this crate's own docs therefore describe the same relationship
in opposite terms: one as a live bidirectional seam with four operations
crossing it, the other as a dependency deliberately not taken. The decision is
the later and correct one; A4 is the earlier reading it was ruled against, left in
place. The row is worth keeping — a seam ruled out is worth recording — but as
an absent seam alongside A6 and A7, which the table already knows how to write.
