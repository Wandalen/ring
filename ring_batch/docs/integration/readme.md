# integration

`ring_batch` sits on four edges in and one edge out. The four in are all used —
one `use` statement each, no dead dependency — and the two documents that restate
them for a reader name four and two respectively — the module comment now names
every dependency, and the readme still omits two: `ring_atomic` and `ring_index`.
The one edge out goes to a thread-local staging buffer that takes two of the
crate's twelve public items.

Above all of it sits a feature document that says the work is planned. It is not
alone: every feature this family of 33 crates cites by name still reads
`planned`, and the number that feature exists to assert — sixty-four items for
roughly the price of one — is measured by no benchmark in the repository.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_four_edges_in_two_written_down.md) | Four Edges In, Two Written Down | The manifest against the module comment against the readme, and the citation path all 33 crates get wrong |
| [002](002_the_feature_is_planned_its_problems_are_addressed.md) | The Feature Is Planned, Its Problems Are Addressed | The single dependent, the design corpus above it, and the unmeasured headline claim |

## The Dependency List Is Written Once and Never Again

Both findings in `001` have the same shape: a statement about the crate's
surroundings, correct when written, never re-checked. The module comment's
`Depends on` sentence was true when the crate had three dependencies and has
since been corrected to name all four; the readme's copy has not, and still
names two. The citation path was true when the underlying document was a flat file and
is not true now, and nothing in the toolchain can tell — a backtick-quoted
path is inline code to `rustdoc`, and a prose list of crate names is prose.

Eleven of the 29 crates carrying that sentence understate their dependencies, and
all 33 cite the missing spec path. Both are mechanically checkable in about five
lines of shell. Neither is checked.

## Reachability Runs Out Two Crates Later

The outgoing story is narrower than it looks. `ring_tls` is the only dependent,
and it takes `claim` and `BatchClaim` — not `claim_gated`, not `drain_order`.
`ring_tls` has four dependents of its own, so the batch path is reachable from
`ring_bench`, `ring_flush`, `ring_testkit`, and `smoke_ring_write_path` at one
remove.

Reachable and reached are different things. `ring_bench` exists to measure, the
feature above `ring_batch` states a measurable claim, and no file under
`ring_bench/` mentions `ring_batch` at all.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- in --'
command grep -E '^ring_' ring_batch/Cargo.toml | sed 's/ = .*//' | tr '\n' ' '; echo
command grep -m1 -F '//! Depends on `ring_types`, `ring_seqno`, `ring_atomic` and `ring_index`.' ring_batch/src/lib.rs
command grep -m1 -F 'Depends on [`ring_types`](../ring_types/readme.md), [`ring_seqno`](../ring_seqno/readme.md).' ring_batch/readme.md
echo '  -- out --'
command grep -rl '^ring_batch = ' --include=Cargo.toml . | sed 's|ring/||;s|/Cargo.toml||' | tr '\n' ' '; echo
command grep -n 'use ring_batch' ring_tls/src/lib.rs
echo '  -- above --'
command grep -m1 'Status' docs/feature/177_batch_claim_and_batch_drain.md
echo "  -- and the family-wide checks nothing runs --"
echo "    src/lib.rs citing a spec path that does not exist : $( command grep -rl '008_ring_write_path\.md' --include=lib.rs . | wc -l ) of 33"
echo "    files under ring_bench naming ring_batch          : $( command grep -rl ring_batch ring_bench/ 2>/dev/null | wc -l )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA17 | `` | **wrong doc** | Eleven of the 29 crates with a `Depends on` sentence understated it when filed; `ring_batch`'s, `ring_mpsc`'s and `ring_publish`'s module comments have since been corrected to name every dependency — `ring_batch`'s readme has not, and still omits two |
| BA18 | `` | **wrong doc** | All 33 source files cite a path ending in `.md` for their family's own design corpus, which does not exist; all 33 readmes cite the directory that does |
| BA19 | `ring_batch` | n/a — observation | The undeclared fourth dependency has one call site, inside `drain_order`, the function no crate in the family calls |
| BA20 | `docs/feature` | **misleading doc** | All 16 features cited by the family read `present`, flipped as the contiguous 167-188 block rather than per feature — the field records that a contiguous range was declared done, not that any one feature under it was built |
| BA21 | `ring_bench` | n/a — coverage | This crate's own stated claim is a number, `ring_bench` exists to produce numbers, and no file under `ring_bench/` mentions `ring_batch` |
