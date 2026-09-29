# integration

Two edges in, one edge out, and three lines of contact. `ring_overflow` depends on
`ring_types` for the policy and the error and on `ring_stats` for the counters;
exactly one crate in the workspace declares `ring_overflow`, and its entire use of
this crate is an import, a `match` scrutinee, and one arm.

The two instances here record what crosses those edges and what does not. The
consumer takes the pure half and leaves the effectful one, which makes the
`ring_stats` dependency real, correct, and untraversed by any shipping build.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_consumer_one_import_one_site.md) | One Consumer, One Import, One Site | Every edge in both directions, and the three lines of use |
| [002](002_the_stats_edge_and_the_function_nobody_imports.md) | The Stats Edge and the Function Nobody Imports | A whole dependency for one statement with no production caller |

## What the Consumer Takes

`ring_core/src/lib.rs:80` imports `would_resolve` and `Resolution`. It does not
import `resolve`, `lost_an_item`, or `accepted_incoming`. Of three variants it
names one, and of four functions it calls one.

This inverts the division of labour the source describes. `would_resolve` is
documented as being for "a factory validating a configuration, a test tabulating
the mapping — rather than handling a real full-ring event." The one line that calls
it is handling a real full-ring event.

## The Dependency That Goes Nowhere

`ring_stats` is declared, imported, and used exactly once: `record_drop` at
`src/lib.rs:199`, inside `resolve`. The sole consumer names `resolve` zero times
and `would_resolve` twice.

So the crate's strongest documented promise — exactly one counter incremented per
call, so a stats read accounts for every full-ring event — is kept by the code and
unreachable by the system. `ring_stats` records the same break from the reading
side; this crate owns the writer that never runs.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- both dependencies out --'
sed -n '/^\[dependencies\]/,/^\[/p' ring_overflow/Cargo.toml | command grep '^ring_'
echo '  -- every crate that declares this one --'
for f in */Cargo.toml ; do command grep -l '^ring_overflow' "$f"; done 2>/dev/null | sed -E 's|/Cargo.toml||'
echo '  -- and the whole of its use --'
command grep -n 'ring_overflow\|would_resolve\|Resolution' ring_core/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV17 | `ring_overflow` | n/a — observation | The crate declares two dependencies, is declared by exactly one crate, and that consumer's entire use of it is three lines — one `use`, one `match` scrutinee, one arm — so four exported functions yield one import, three exported variants yield one name, and two exported predicates yield none |
| OV18 | `ring_overflow` | **misleading doc** | `ring_core` imports `would_resolve` and not `resolve`, so the pure half carries all production traffic and the effectful half carries none — inverting the source's own description of `would_resolve` as being for "a factory validating a configuration, a test tabulating the mapping — rather than handling a real full-ring event", which is precisely what its one caller is doing |
| OV19 | `ring_overflow` | **latent hazard** | The whole `ring_stats` dependency exists for `stats.record_drop( policy, 1 )` at `src/lib.rs:199` inside `resolve`, and the sole consumer names `resolve` zero times against `would_resolve` twice — so every execution of the crate's counter write happens under `cargo test`, and the edge is real, correctly declared, correctly used, and traversed by nothing in a shipping build — declined, since `ring_core` states it adds no atomic of its own so `ring_spsc`'s zero-RMW assertion holds, and wiring the edge would break that |
| OV20 | `ring_overflow` | **misleading doc** | `resolve` promises "Exactly one counter is incremented per call, whichever branch is taken — so a stats read accounts for every full-ring event, not only the lossy ones", a guarantee kept by the code and unreachable by the system, since the caller producing real full-ring events uses the counter-free half — leaving both crates individually consistent and jointly hollow, with the missing piece in a third that neither one's documentation can see |
