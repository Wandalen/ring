# invariant

Three properties matter for a batch claim protocol: no two claims may overlap, a
thread's own claims must come back in the order it asked for them, and the
sequences outstanding must never exceed the ring's free space. The suite asserts
the first two, carefully, under four threads and five hundred batches each.

The third is the only one that can be violated, and it is the one with no test.
Both contention tests call the ungated entry point; every one of the thirteen
references to the gated one sits above the contention section, on a single
thread. The result is a suite that is neither weak nor wrong and is blind in
exactly one direction.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_disjointness_is_free.md) | Disjointness Is Free | Why no two claims overlap through either entry point, and why that makes the test unable to fail |
| [002](002_ascending_not_contiguous.md) | Ascending, Not Contiguous | Per-thread order, the gap distribution the `<=` tolerates, and the bound nothing asserts |

## Cheap Invariants and Expensive Ones

Disjointness costs one atomic instruction. `fetch_add` returns a distinct old
value to every caller, so the sequences handed out are unique whatever the
interleaving — and the gated entry point ends in a call to the ungated one, so it
inherits the property intact. Twenty runs through each path, sixteen thousand
sequences apiece, produced zero collisions.

The capacity bound costs a compare-exchange loop, and the crate does not pay it.
Two loads and a comparison decide whether there is room, and an unsynchronised
`fetch_add` acts on that decision — so the property that is expensive to hold is
the property that is not held, and the one that is free is asserted twice.

## The Suite Measures the Right Things About the Wrong Entry Point

Both contention tests are well constructed. The disjointness test checks
individual sequences through a `HashSet` rather than comparing ranges, which
catches an off-by-one at either end, and cross-checks the final cursor position.
The ordering test asserts `end() <= start()` on adjacent pairs, which is exactly
the per-thread ordering requirement and no more — three quarters of those pairs have
another thread's claim wedged between them, so anything stronger would flake.

Neither runs `claim_gated`. The gate is the part of this crate that can be wrong,
and it is the part the threads never touch.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two contention tests --'
command grep -n 'fn concurrent_batch_claims_never_overlap\|fn a_threads_own_batches_stay_in_its_issue_order' ring_batch/tests/batch_test.rs
echo '  -- and the entry point they call --'
command grep -n 'scope.spawn' -A 3 ring_batch/tests/batch_test.rs | command grep -E 'claim\(|claim_gated\('
echo "  -- gated references, all of them above line 316 : $( command grep -c claim_gated ring_batch/tests/batch_test.rs || true ) --"
echo '  -- the untested bound --'
command grep -m1 -A5 -F '  let at = producer.load( Ordering::Acquire );' ring_batch/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA22 | `ring_batch` | n/a — observation | Disjointness comes from `fetch_add` alone and survives the gated path unchanged — 16000 sequences per run, zero claimed twice, through both entry points over 20 runs each |
| BA23 | `ring_batch` | **misleading doc** | The disjointness test calls its property "the property a claim protocol must never violate"; it is the property this protocol cannot violate, so both of its witnesses are blind to the crate's actual failure |
| BA24 | `ring_batch` | n/a — observation | The ordering assertion's `<=` is load-bearing: only 39,469 of 159,680 adjacent pairs were contiguous, and the widest gap between one thread's own claims was 680 sequences |
| BA25 | `ring_batch` | n/a — coverage | The capacity bound — the only invariant that can actually break — is asserted by no test; both threaded tests call `claim`, and all 13 `claim_gated` references sit above the contention section |
