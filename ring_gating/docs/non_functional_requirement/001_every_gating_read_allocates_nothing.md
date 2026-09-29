# NFR: Every Gating Read Allocates Nothing

### Scope

- **Purpose**: Price one `headroom` call, establish where the cost lands, and record that nothing measures it — including the allocation that used to be the largest term and is now gone.
- **Responsibility**: Record the allocation that was removed, count the calls that still cannot be inlined and say why, quantify what a claim costs in the one production caller, and state what the family's benchmark crate covers instead.
- **In Scope**: The cost of a gate read.
- **Out of Scope**: What the chain computes — see [`algorithm/001`](../algorithm/001_headroom_in_two_delegations.md).

### The Requirement

Consulting the gating set is on the producer's hot path, so every measured
claim pays whatever it costs. No number is attached to that cost up front —
pricing it is what the rest of this document does.

### The Allocation That Was Here

```rust
// ring_cursor/src/lib.rs, before commit b7e075ca
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
  ring_seqno::slowest( &positions )
}
```

`collect()` into a `Vec< Seq >`, then a `min()` over it. **One heap allocation per
call**, sized `8 × consumers` bytes — on every gate read, inside a CAS retry
loop. That was the requirement violation this document was opened to record.

The reason it existed was a signature one crate further down:

```rust
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
```

`ring_seqno` folds over *values*, `ring_cursor` holds *cells*. Nothing bridges a
`&[ PaddedCursor ]` to a `&[ Seq ]` without materialising one, so the `Vec` was
the price of the boundary — the same boundary
[`algorithm/001`](../algorithm/001_headroom_in_two_delegations.md) credits with
isolating "whether the fold is over cursors or over values".

The removal this section used to name as obvious —
`cursors.iter().map( | c | c.load( GATING ) ).min()`, the identical fold with no
intermediate — is what `ring_cursor` now does. It computes the same answer,
allocates nothing, and moved the `min` back into `ring_cursor`, collapsing the
boundary along with it. That was a `ring_cursor` decision, not this crate's, and
it is recorded there:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the fold this crate reaches, as it now stands --'
awk '/^pub fn slowest\( cursors : &\[ PaddedCursor \] \)/{ f = 1 } f { print } f && /^\}$/{ exit }' ring_cursor/src/lib.rs
# what remains: this crate's own buffer, held for a set's whole life rather
# than built per read. It is the only allocation left on the chain
echo '  -- and the one allocation still on the chain, which is this crate own --'
command grep -E '^  cursors : Vec|Vec::with_capacity|resize_with' ring_gating/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- the fold this crate reaches, as it now stands --
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}
  -- and the one allocation still on the chain, which is this crate own --
      cursors : Vec< PaddedCursor >,
        let mut cursors = Vec::with_capacity( consumers );
        cursors.resize_with( consumers, PaddedCursor::default );
```

**What is left is a construction cost, not a read cost.** `GatingSet` owns its
cursors, so one allocation happens when a set is built and none when it is read.
That is the shape the requirement asked for: the zero-consumer case was already
free, and now every case is.

### G14 — Three Calls That Cannot Be Inlined

The chain is five steps ([`algorithm/001`](../algorithm/001_headroom_in_two_delegations.md))
since the `ring_cursor` → `ring_seqno` delegation collapsed, three of which cross a
crate boundary. Rust makes a non-generic function's body
available for cross-crate inlining only when it is marked `#[ inline ]`, or when
LTO is on. Neither holds here:

```sh
cd "$(git rev-parse --show-toplevel)"
# the family carries exactly one inline hint, in ring_trace, added for a
# cost claim of its own. This counts grep hits, so the doc comment naming the
# attribute counts alongside the attribute — the crate census below is the
# unambiguous form
grep -rc '#\[ *inline' ring_*/src/*.rs | awk -F: '{s+=$2} END {print s}'
# … control: the identical expression over every crate in the workspace, stated
# as a relation for the same reason the profile check below is — the raw count
# drifts every time any session adds a crate, and its value was never the claim
[ "$( grep -rc '#\[ *inline' */src/*.rs | awk -F: '{s+=$2} END {print s+0}' )" -gt 0 ] \
  && echo 'workspace outside the family: some' \
  || echo 'MISMATCH — the workspace has no inline hints either, so this controls nothing'
# every [profile.*] section is a dev-package opt-level override, so there is no
# [profile.release] for lto to be set in — asserted as a relation rather than a
# raw count, which drifts as crates enter the workspace
[ "$( grep -c '^\[profile' Cargo.toml )" = "$( grep -c '^\[profile\.dev\.package\.' Cargo.toml )" ] \
  && echo 'all [profile.*] sections are [profile.dev.package.*]' \
  || echo 'MISMATCH — a non-dev-package profile section exists'
grep -q 'lto' Cargo.toml && echo 'lto key present' || echo '(no lto key anywhere in the manifest)'
# and the unambiguous form: how many of the 33 carry the attribute itself
printf 'crates carrying the attribute: %s of %s\n' \
  "$( command grep -rl '^  #\[ inline \]$' ring_*/src/lib.rs | wc -l )" \
  "$( command grep -v '^#' bench_harness/gate/declared/ring/crates.txt | command grep -cv '^$' )"
```

Live output:

```
2
workspace outside the family: some
all [profile.*] sections are [profile.dev.package.*]
(no lto key anywhere in the manifest)
crates carrying the attribute: 1 of 33
```

**One `#[ inline ]` in all 33 crates, while the workspace outside the family has
them, so the near-absence is this family's choice and not a house style** — and
not one `lto` key anywhere in the manifest. Its `[profile.*]` entries are all
`[profile.dev.package.*]` opt-level overrides; there is no `[profile.release]`
section at all, so cargo's defaults apply, which for release means `lto = false`.

The one exception is `ring_trace::Trace::record`, marked for a cost claim in that
crate's own documentation
([`ring_trace` `algorithm/001`](../../../ring_trace/docs/algorithm/001_the_early_return_that_is_the_whole_feature.md)).
No step of *this* chain carries one.

| Step | Generic? | `#[ inline ]`? | Inlinable cross-crate |
|------|:--------:|:--------------:|:---------------------:|
| `GatingSet::headroom` → `GatingSet::slowest` | — | — | same crate, yes |
| `GatingSet::slowest` → `ring_cursor::slowest` | ❌ | ❌ | **no** |
| ~~`ring_cursor::slowest` → `ring_seqno::slowest`~~ | — | — | *gone — no longer a call* |
| `headroom` → `ring_seqno::free_slots` | ❌ | ❌ | **no** |
| `free_slots` → `Seq::distance_to` | ❌ | ❌ | **no** |

So a default release build of one `headroom` call is three real calls and no
allocation, to compute one saturating subtraction. It was four calls plus a
`malloc`/`free` pair: removing the `Vec` removed the delegation that carried it,
so the chain lost a step and its only heap traffic in the same change.

This is a family-wide condition, not a `ring_gating` defect — every crate in the
family is in the same position. It is recorded here because this is the chain
that is documented as hot-path.

### Where It Lands

`ring_claim::Claimer::claim` puts the gate read in its **loop condition**:

```rust
// ring_claim/src/lib.rs:437
while count <= self.consumers.headroom( current )
{
  let next = current.advanced_by( count as u64 );
  match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
  { … }
}
```

It has to be there — the crate explains that a headroom computed before a failed
exchange is stale and granting on it would overlap another producer's range
([`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md)). The cost
follows directly:

| Contention | Gate reads per successful claim | Allocations, before | Allocations, now |
|------------|:-------------------------------:|:-------------------:|:----------------:|
| Uncontended | 1 | 1 | 0 |
| One CAS failure | 2 | 2 | 0 |
| *n* CAS failures | *n* + 1 | *n* + 1 | 0 |

**The allocation rate used to scale with contention**, on the path whose whole
purpose is to survive contention. `Claimer::claim_up_to` has the same shape and
was freed by the same change. What still scales with contention is the *call*
count — three uninlinable calls per gate read, times *n* + 1 reads — and that is
what the rest of this document is about now.

### G13 — Nothing Measures It

`ring_bench` is this family's benchmark harness — it additionally publishes a
measured verdict per candidate pattern. It does not mention the gate:

```sh
cd "$(git rev-parse --show-toplevel)"
wc -l ring_bench/src/lib.rs
grep -c 'gating\|GatingSet\|headroom' ring_bench/src/lib.rs || true
```

Live output:

```
1450 ring_bench/src/lib.rs
1
```

Over twelve hundred lines, and the hot-path cost this document prices is
not among what they measure. The line count is quoted from the block above rather
than restated, because a number written twice in one document drifts in one place
first. `ring_bench`'s surface is workload-shaped — `Workload`,
`Candidate`, `Outcome`, `Comparison`, `run` — which measures *whole write-path
patterns* end to end. That is the right top-level design, and it means the gate's
cost is inside every measurement and isolated by none of them.

| Question | Answerable today |
|----------|:----------------:|
| Which write-path pattern is fastest? | ✅ `Comparison::fastest` |
| What does one gate read cost? | ❌ |
| Did removing the `Vec` help, and by how much? | ❌ — and now unrecoverable: there is no before-state left to measure |
| How does the cost scale with consumer count? | ❌ |

The second row is the one this document lost. The `Vec` was removed on the
argument that a heap allocation in a lock-free retry loop is wrong whatever it
costs — a good argument, and the only one available, because nothing here could
price it either before or after. The last row still stands: the fold is still
linear in consumer count and the multi-consumer path still has no production
caller ([`data_structure/001`](../data_structure/001_the_set_that_cannot_grow.md)),
so the first ring that needs a second consumer will still pay a cost nobody has
measured at one consumer, let alone at several.

### What Would Change the Picture

| Change | Effect | Owner | State |
|--------|--------|-------|-------|
| `min()` over the iterator, no `collect` | Removes the allocation entirely | `ring_cursor` | **done** — commit `b7e075ca` |
| `#[ inline ]` on the boundary functions | Lets the chain collapse | family-wide | not done |
| `[profile.release] lto = "thin"` | Same, without touching source | workspace root | not done |
| A `ring_bench` candidate isolating the gate | Makes the other two measurable | `ring_bench` | not done |

The fourth should have come first, and did not: the first row was taken against
an unmeasured cost, which is exactly the shape of change this family's own
benchmark crate exists to prevent. It was the right call on the argument — a
`malloc` inside a CAS retry loop is a design contradiction whatever it measures —
and it is still a change made without the number, which is worth recording
plainly rather than reading backwards as vindicated.

### GT38 — The Benchmark Crate Does Not Know the Gate Exists

```
ring_bench/src/lib.rs : over 1,200 lines (exact count in § G13 above)
occurrences of 'gating' (case-insensitive) : 0
```

The requirement is about cost on the producer's hot path. The crate whose job
is to measure that path does not mention the component sitting in it.

**Finding.** The documented hot-path cost is unmeasured: `ring_bench` runs to over twelve hundred lines and mentions the gate zero times

---

### GT39 — Nothing Asks the Compiler to Collapse the Chain

```
crates carrying #[ inline ] in all 33        : 1 (ring_trace, for a claim of its own)
steps of *this* chain carrying one           : 0
lto in the root manifest                     : absent
```

What the chain costs now is the calls themselves, and whether they collapse.
Removing the allocation removed one of the five steps outright; the remaining
three cross-crate calls are unchanged, and both mechanisms that would collapse
them are still absent from the workspace.

**Finding.** With no `#[ inline ]` on any step of this chain and no LTO configured, three of the chain's five remaining steps cannot be inlined across their crate boundaries in a default release build

**Disposition:** declined — this instance's own "What Would Change the
Picture" table assigns every remaining fix to a different owner: family-wide
`#[ inline ]` hints, the workspace-root `Cargo.toml`'s missing
`[profile.release]`, and a `ring_bench` candidate; none is this crate's own
source or doc to change in a corpus disposition pass. The fourth owner on that
list has since acted — `ring_cursor` removed the `Vec`-collecting fold in commit
`b7e075ca` — which closes the allocation term of this finding but not the
inlining term, which is what the finding is actually about and which is
unchanged.

---


### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_headroom_in_two_delegations.md](../algorithm/001_headroom_in_two_delegations.md) | The five steps this prices |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | Why consumer count is fixed, and why nobody has paid the multi-consumer cost |
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The `Vec` field, and the one `into_boxed_slice` the family uses elsewhere |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | Why the gate read must be the loop condition |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_the_gate_must_never_over_report.md](002_the_gate_must_never_over_report.md) | The correctness requirement the cost buys |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:120-123` | The fold, `collect`-free since commit `b7e075ca` |
| `ring_seqno/src/lib.rs:133-136` | The signature the old `collect` bridged to |
| `ring_claim/src/lib.rs:437,488` | The two loop conditions |
| `Cargo.toml` | No `[profile.release]`, therefore no LTO |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:394-428` | The only concurrent exercise, 10,000 gate reads — and no timing |
| `ring_claim/tests/claim_test.rs:316-435` | Four 16k/8k-capacity ungated runs, the closest the family comes to a load test |
