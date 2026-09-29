# integration

Where this crate meets everything else. Two edges in, one edge out, and an external
tracking record that asked for it. That is a small enough surface to state exhaustively,
which is what the first instance does; the second follows the citation upward to
the feature the crate says it implements half of.

The interesting content is not the edges that exist but the two that do not. One
crate folds sequences to slots without depending on this one, and the module
comment names a code path — the claim path — that has no dependency on it and
never needed one. Both absences are recorded here rather than in `pitfall/`,
because they are facts about the shape of the graph before they are hazards.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_dependents_and_a_third_that_did_it_again.md) | Two Dependents, and a Third That Did It Again | The manifest topology, the transitive reach through `Buffer::at`, and the duplicate fold in `ring_mpsc` |
| [002](002_the_feature_it_implements_half_of.md) | The Feature It Implements Half Of | This crate's own cited feature record, its stale status, its unmeasured claim, and the error the module comment inherited from it |

## Two Manifests Understate the Reach, and Overstate the Boundary

Only `ring_batch` and `ring_store` name `ring_index` in a manifest. But
`ring_store::Buffer::at` folds internally, so every crate that borrows a slot
through a `Buffer` — `ring_spsc` among them — folds through this crate without
an edge of its own. That is the design working: the fold arrives with the
storage type rather than as a dependency each crate must remember.

The same census overstates the boundary in the other direction. A crate that
needs an index for something that is *not* a `Buffer` has no transitive path and
must add the edge deliberately. `ring_mpsc` needed one, for a bare
`Vec< AtomicSeq >`, and wrote the fold out by hand instead.

## The Citation Runs Up, and Nothing Runs Back Down

The crate names its feature in its first doc lines. The feature names no crate,
and is still marked `planned` while three crates implement it. Nothing compares
the two, so the status stays wrong indefinitely and the feature's own promise —
that the masking claim would be measured rather than assumed — stays unkept even
though `ring_bench` exists and works, because `ring_bench` measures whole write
paths and this claim lives a level below that.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/Cargo.toml; do
  if command grep -q 'ring_index' "$c"; then echo "  ${c#ring/}"; fi
done
command grep -rn '( seq.0 as usize ) &' ring_*/src/*.rs
echo "  ring_bench mask/modulo/ring_index mentions: $( command grep -c 'mask\|modulo\|ring_index' ring_bench/src/lib.rs )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX10 | `ring_store` | n/a — observation | The fold reaches further than its two manifest edges, transitively through `Buffer::at`; `ring_store` holds the family's only written statement of the one-owner rule |
| IX11 | `ring_index` | **wrong doc** | The module comment says the fold sits on the claim path; none of the five claim/gate/consume crates depends on this one |
| IX12 | `ring_mpsc` | **latent hazard** | A second, character-identical fold at `ring_mpsc:543`, in a crate with eight `ring_*` dependencies and no edge to `ring_index` |
| IX13 | external tracking record | **misleading doc** | Marked `present` by an external batch flip rather than by any check of the three crates that implement it; the citation still runs one way only |
| IX14 | `ring_bench` | n/a — coverage | The feature promises the masking claim will be measured; the family's 1098-line measurement crate compares whole write paths and never mentions mask, modulo, or `ring_index` |
| IX15 | external tracking record | **wrong doc** | The "every claim and every read" error in the module comment is a faithful restatement of that external record's own wording |
