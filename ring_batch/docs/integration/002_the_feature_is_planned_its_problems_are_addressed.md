# Integration: The Feature Is Planned, Its Problems Are Addressed

### Scope

**Purpose:** Record the crate's single outgoing edge, the originating requirement
and hard-problem records above it, and the fact that the performance claim those
records exist to make is measured nowhere in the repository.

**Responsibility:** `ring_tls` as the one dependent, this crate's own originating
requirement and its two hard problems, and `ring_bench`'s reach.

**In Scope:** `ring_tls/src/lib.rs:44`; `ring_bench/`;
this crate's own originating requirement and its two hard-problem records.

**Out of Scope:** The four incoming edges are
[`integration/001`](001_four_edges_in_two_written_down.md). The measured cost
curve itself is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_one_batch_actually_buys.md).

---

## One Edge Out, and the Requirement Above It

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the one dependent, and what it takes --'
command grep -rl '^ring_batch = ' --include=Cargo.toml . | sed -E 's|/Cargo.toml||' | tr '\n' ' '; echo
command grep 'use ring_batch' ring_tls/src/lib.rs
echo '  -- and who depends on it --'
command grep -rl '^ring_tls = ' --include=Cargo.toml . | sed -E 's|/Cargo.toml||' | sort | tr '\n' ' '; echo
echo '  -- the feature this crate is built for, and its hard problems --'
command grep -m1 'Status' docs/feature/177_batch_claim_and_batch_drain.md
command grep -m1 'Status' docs/hard_problem/118_merging_thread_local_writes_into_one_stream.md
command grep -m1 'Status' docs/hard_problem/122_batch_size_against_tail_latency.md
echo '  -- what the benchmark crate names --'
echo "    ring_batch in ring_bench : $( command grep -rc ring_batch ring_bench/src/lib.rs || true )"
echo "    ring_tls   in ring_bench : $( command grep -rc ring_tls ring_bench/src/lib.rs || true )"
```

Live output:

```
  -- the one dependent, and what it takes --
ring_tls 
use ring_batch::{ claim, BatchClaim };
  -- and who depends on it --
ring_bench ring_flush ring_testkit smoke_ring_write_path 
  -- the feature this crate is built for, and its hard problems --
- **Status:** present
- **Status:** addressed
- **Status:** addressed
  -- what the benchmark crate names --
    ring_batch in ring_bench : 0
    ring_tls   in ring_bench : 2
```

---

### BA20 — Every Feature This Family Implements Was Marked in One Block, After the Fact

The two hard problems above this crate's own originating requirement are both `addressed`, and when this
instance was filed the feature itself read `planned`. That looked backwards — a
feature is what addresses a hard problem, so the parent cannot be done while the
child is not.

It was not backwards. The two words live on different axes: `addressed` means a
feature has been designated for the problem, which is a statement about the
status field; `planned` means the feature has not been built, which is a statement
about code. The status-field fact was recorded and the code-side one was not.

**That has since changed, in one stroke, for all twenty-two features at once.**
The census below reads the family's own range exactly, and the rest
only for which value still dominates — the aggregate tally it used to print
was a population maintained elsewhere that keeps moving, and it went stale twice without
anything here changing:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- status of every feature the 33 crates cite --'
for f in $( command grep -rhoE 'docs/feature/[0-9]+_[a-z0-9_]+\.md' ring_*/src/lib.rs | sort -u )
do
  command grep -m1 '^- \*\*Status:\*\*' "$f"
done | sort | uniq -c
# The aggregate tally is a moving population maintained elsewhere — it keeps
# shifting independently of this crate — so this prints the invariant the contrast rests on rather than a
# count that goes stale between two runs of this gate.
echo '  -- against every feature file, which value still dominates --'
command grep -h -m1 '^- \*\*Status:\*\*' docs/feature/*.md | sort | uniq -c | sort -rn \
  | awk 'NR==1{ printf "    majority value: %s\n", $NF } END{ printf "    distinct values: %d\n", NR }'
echo '  -- and features 167-188 as one contiguous block --'
for f in docs/feature/1[678]*.md
do
  n=$( basename "$f" | cut -d_ -f1 )
  [ "$n" -ge 167 ] && [ "$n" -le 188 ] && command grep -m1 '^- \*\*Status:\*\*' "$f"
done | sort | uniq -c
echo "  -- and the code that implements them --"
echo "    ring_* crates with a src/lib.rs : $( ls ring_*/src/lib.rs | wc -l )"
echo "    ring_* crates with tests        : $( ls -d ring_*/tests 2>/dev/null | wc -l )"
```

Live output:

```
  -- status of every feature the 33 crates cite --
     16 - **Status:** present
  -- against every feature file, which value still dominates --
    majority value: planned
    distinct values: 2
  -- and features 167-188 as one contiguous block --
     22 - **Status:** present
  -- and the code that implements them --
    ring_* crates with a src/lib.rs : 33
    ring_* crates with tests        : 33
```

**Finding.** Sixteen features are cited by name from the family's source files,
and all sixteen now read `present` — as does every one of the twenty-two in the
`167`-`188` range, which is this family's own feature-tracking block. The original reading was
that "the status field records what was decided and never learns what was built." It
has now learned, once, for the whole family simultaneously.

The manner of the learning is the finding. Nothing about `ring_batch`
specifically was checked: `167`-`188` is a contiguous range flipped as a unit,
and it contains features whose crates differ sharply in how finished they are.
One sibling feature record (`ring_benchmark_harness`) reads `present` alongside this crate's own, and
`ring_bench` has never heard of `ring_batch` — which is the subject of
[`BA21`](#ba21--the-headline-number-is-a-performance-claim-and-the-benchmark-crate-has-never-heard-of-this-one)
immediately below. A field that says `present` for both a crate with 31 passing
tests and a claim nothing has measured is reporting that the range was flipped,
not that the feature was built.

So the axis distinction that opened this finding survives intact, one level up.
`addressed` is a statement about the status field, and `present` has turned out to be
one too — it records that a contiguous range was declared done, not that any particular
feature under it was built. `ring_stats`
[`lifecycle/002` § ST31](../../../ring_stats/docs/lifecycle/002_implemented_tested_and_still_planned.md)
takes the same observation from the other end, and finds the same block.

**Disposition:** applied — the census embedded above already measures the
live state: this crate's own originating requirement and the whole 167-188 range now read
`present`, closing the planned/addressed mismatch this instance was filed
against. Nothing further to change in this file; the finding's own prose
narrates the resolution and the census is the proof.
Now prints: `22 - **Status:** present`

---

### BA21 — The Headline Number Is a Performance Claim, and the Benchmark Crate Has Never Heard of This One

This crate's own originating requirement states the claim the crate has to make true, and quotes it back in
the module comment:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the claim --'
command grep -m1 -A3 -F '//! `docs/feature/177_batch_claim_and_batch_drain.md` states the claim this crate' ring_batch/src/lib.rs
echo '  -- the crate that measures claims like it --'
command grep -E '^ring_' ring_bench/Cargo.toml | sed 's/ = .*//' | tr '\n' ' '; echo
echo "  -- ring_batch in ring_bench code and manifest : $( command grep -rl ring_batch ring_bench/src ring_bench/tests ring_bench/examples ring_bench/Cargo.toml 2>/dev/null | wc -l ) files --"
echo "  -- ring_batch in ring_bench documents         : $( command grep -rl ring_batch ring_bench/docs 2>/dev/null | wc -l ) files --"
```

Live output:

```
  -- the claim --
//! `docs/feature/177_batch_claim_and_batch_drain.md` states the claim this crate
//! has to make true: "the memory fences that make the handshake correct are paid
//! per operation, not per item, so a batch of sixty-four costs roughly what a
//! single item costs."
  -- the crate that measures claims like it --
ring_factory ring_flush ring_mpsc ring_spsc ring_stats ring_tls ring_core ring_slot ring_types 
  -- ring_batch in ring_bench code and manifest : 0 files --
  -- ring_batch in ring_bench documents         : 2 files --
```

**Finding.** The claim is a number — sixty-four items for roughly the price of
one — and the family has a crate whose entire purpose is producing numbers.
`ring_bench` depends on `ring_tls`, and `ring_tls` is the crate that calls
`claim`, so the batch path is two edges away and reachable. No executable file
under `ring_bench/` mentions `ring_batch` — not source, tests, examples or
manifest. Two of its documents do, both in passing and neither measuring
anything.

So the crate names, in its own module comment, the one claim it exists to make
true, and nothing in the repository measures it. The curve recorded in
[`non_functional_requirement/001`](../non_functional_requirement/001_what_one_batch_actually_buys.md)
came from a throwaway probe written for this corpus, not from the benchmark crate
that should own it — which means the number can drift arbitrarily far without any
test, benchmark, or check noticing.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_four_edges_in_two_written_down.md) | The four edges in, and the two documents that undercount them |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_one_batch_actually_buys.md) | The curve the benchmark crate does not measure |
| [`api/002`](../api/002_two_claim_functions_one_caller.md) | Which of the twelve items `ring_tls` actually reaches |
| [`lifecycle/002`](../lifecycle/002_the_empty_claim_as_a_first_class_state.md) | How `ring_tls` calls `claim`, and what it does not check first |

### Sources

| Fact | Where |
|------|-------|
| The one dependent and its import | `ring_tls/src/lib.rs:44` |
| `ring_tls`'s four dependents | Census above |
| This crate's own originating requirement and its two hard problems | Census above |
| Sixteen cited features, all `planned` | Census above |
| `ring_bench`'s dependency list | `ring_bench/Cargo.toml` |
| The claim quoted in the module comment | `ring_batch/src/lib.rs:7-10` |

### Tests

| Test | Covers |
|------|--------|
| `the_sequences_returned_are_contiguous` | The contiguity half of the feature's claim |
| *(to create)* | The cost half — a benchmark in `ring_bench`, which does not depend on this crate |
| *(to create)* | That a cited feature's status matches whether its crate exists — checkable, checked nowhere |
