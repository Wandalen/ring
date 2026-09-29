# Non Functional Requirement: What Contention Costs

### Scope

- **Purpose**: Derive what a claim costs at the memory-traffic level, state what scales with what, and record that none of it is measured anywhere in the family.
- **Responsibility**: Separate what the source establishes from what would need a benchmark, and locate the one number this crate actually measured.
- **In Scope**: Instruction and cache-line cost of `claim` / `claim_up_to`, how it scales with producers and consumers, and the family's benchmark coverage of it.
- **Out of Scope**: The properties that hold regardless of load — see [`non_functional_requirement/001`](001_nothing_allocates_nothing_waits_nothing_is_unsafe.md).

### The Cost, Derived From the Source

Everything below is read off the implementation. **None of it is measured** —
see CL37.

An uncontended `claim( n )` against a set of `C` consumers:

| Step | Operation | Memory traffic |
|------|-----------|----------------|
| `self.claimed()` | one `Acquire` load of the claim cursor | 1 line, read |
| `self.consumers.headroom( current )` | `slowest()` over `C` cursors, then arithmetic | `C` lines, read |
| `compare_exchange` | one `AcqRel` RMW on the claim cursor | 1 line, **read for ownership** |

So the floor is `C + 2` cursor touches and one atomic RMW, on `C + 1` distinct
cache lines — every cursor in the family is a `PaddedCursor`, 64 bytes, one per
line by construction.

Each retry adds a full `headroom` re-read plus another RMW, because the gate is
the loop condition and is deliberately re-read every iteration
([`algorithm/001`](../algorithm/001_the_gate_inside_the_retry.md)):

| Attempts | Cursor reads | Atomic RMWs |
|---------:|-------------:|------------:|
| 1 | `C + 1` | 1 |
| 2 | `2C + 1` | 2 |
| *k* | `kC + 1` | *k* |

Two things scale, and they are independent:

- **Producers** scale the retry count `k`. Every successful exchange invalidates
  the claim-cursor line in every other producer's cache, so `N` producers
  ping-pong one line among `N` cores. This is inherent to a shared monotonic
  cursor, not to this design — `fetch_add` would move the same line just as
  often ([`decisions/001`](../decisions/001_compare_exchange_rather_than_fetch_add.md)).
- **Consumers** scale the per-attempt cost `C`. `headroom` reads every
  registered consumer cursor, and nothing in its documentation says so
  ([`item/002`](../item/002_the_seven_of_the_claimer.md)). These lines are
  read-shared and cheap until a consumer advances, which invalidates one line
  for every producer at once.

The multiplication is the part worth naming: cost per claim is roughly
`k × C`, and both factors rise with load. Eight producers against eight
consumers is not eight times the traffic of one-against-one.

The `Claimer` itself contributes one more line. It is 128 bytes for 72 bytes of
fields — the `PaddedCursor` forces 64-byte alignment, and the 8-byte reference
that follows it occupies a second line
([`data_structure/001`](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md)).
Only the first is ever written, which is the point of the padding: the reference
sharing a line with the cursor would put a read-only field in a line invalidated
on every successful claim.

### CL37 — The Family Has a Benchmark Harness, and This Crate Is Not in It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
ls -d ring_*/benches 2>/dev/null | wc -l                     # → 0
command grep -rl '\[\[bench\]\]' ring_*/Cargo.toml | wc -l   # → 0
command grep -rl 'criterion'     ring_*/Cargo.toml | wc -l   # → 0
# split, because a citation in a corpus is not an edge in a build
echo '  -- ring_bench naming ring_claim, in code and manifest (expect 0 -- see prose below) --'
printf '    hits: %s\n' "$( command grep -rl 'ring_claim' --include=*.rs --include=Cargo.toml ring_bench/ | wc -l )"
echo '  -- and in its documents --'
command grep -rl 'ring_claim' --include=*.md ring_bench/ | sort
```

Live output:

```
0
0
0
  -- ring_bench naming ring_claim, in code and manifest (expect 0 -- see prose below) --
    hits: 1
  -- and in its documents --
ring_bench/docs/integration/002_the_only_consumer_of_two_contract_names.md
ring_bench/docs/type/002_run_error.md
```

| | Count across all 33 crates |
|--|---:|
| `benches/` directories | **0** |
| `[[bench]]` manifest sections | **0** |
| criterion (or any bench framework) dependencies | **0** |
| Crates measuring a Tier 0–5 primitive in isolation | **0** |

There *is* a measurement crate — `ring_bench` — and it is substantial: a
`Workload` builder, six `Candidate` write paths, an `Outcome` carrying
`write_nanos` and a conservation check, and a `Comparison` that reports the
fastest. What it measures is whole rings:

| Candidate | What it is |
|-----------|------------|
| `MutexQueue` | a baseline — the design this crate refused |
| `OffTheShelf` | an external baseline |
| `ContractRing`, `TlsOverRing`, `DirectSpsc`, `DirectMpsc` | complete family write paths |

`ring_claim` appears in none of them, and cannot: a `Candidate` is an end-to-end
write path, and a `Claimer` on its own writes nothing. Its contribution to
`DirectMpsc`'s `write_nanos` is real and permanently inseparable from
publication, slot writes, and the gate.

The last two lines of the recipe are split because the flat answer changed while
this document was live. `ring_bench` names `ring_claim` in one source file — a
comment at `tests/bench_test.rs:1475` cross-referencing
`ring_claim/tests/claim_test.rs` for a shared panic-message courtesy, not a
dependency — zero manifests, and in two documents, and none of the three hits is
`ring_bench` actually depending on this crate: the source hit is a comment, and
both document hits are census *output*: a dependent list that happens to
enumerate all 31 crates depending on `ring_types`, and a file path inside a
family-wide scan. A citation in a corpus, or a comment naming a sibling test
file, is not an edge in a build, and the recipe keeps the two apart rather than
leaving it to a reader, because the corpus half will keep growing and the build
half is what the finding rests on.

The sharp part is the first row. `MutexQueue` exists as a candidate, so the
family *does* measure the mutex alternative — and this crate's founding decision
refuses a mutex for an explicitly **non**-performance reason (`:25-35`): a claim
that can block makes `WaitKind::None` unimplementable above it. So the
comparison that exists measures throughput of complete rings, and the decision
that was made was about composability. Neither answers the other, and nothing
anywhere establishes what the CAS loop costs against the lock it replaced.

That is defensible. A primitive this small, benchmarked in isolation, mostly
measures the harness; the honest number is the end-to-end one, and that is what
`ring_bench` produces. What is missing is not a benchmark — it is the sentence
saying so. Nothing in this crate records that its performance characteristics
are unmeasured by design and observable only at Tier 6.

### CL38 — The Only Number This Crate Measured Is a Property of a Test

Search the crate for a measured figure and exactly one turns up, in
`tests/manual/readme.md § C1`:

> measured at 4/5 before the consumer thread gained its `yield_now`, and 8/8
> after, which is why that yield is load-bearing rather than cosmetic.

That is a real measurement, carefully taken, and it changed the code — the
`yield_now` at `claim_test.rs:513` is there because of it. It measures **the
detection rate of a test against a deliberately broken implementation**.

| Measured in this crate | Subject | Effect |
|------------------------|---------|--------|
| 4/5 → 8/8 detection rate | a test's sensitivity | a `yield_now` added, and justified in the plan |
| — | claims per second | — |
| — | retries per claim under `N` producers | — |
| — | cost against a mutex | — |
| — | cost of `headroom` at `C` consumers | — |

So the crate's measurement culture is real but points inward: it quantifies how
well the suite detects a defect, not how fast the code runs. For a correctness
primitive that is arguably the right priority — a fast claim that hands out
overlapping ranges is worthless — and § C1's five-run requirement is a stronger
discipline than most crates apply to anything.

It is still worth naming, because the two are easy to conflate. `§ C1`'s numbers
appear in a document titled a manual *test* plan and read like performance
figures at a glance. They are not. Nothing in this crate has ever been timed.

### What Would Have to Be True to Measure It

Recorded so the gap is actionable rather than merely noted:

| Question | What it needs | Why it is not trivial |
|----------|---------------|-----------------------|
| retries per claim at `N` producers | a counter in the loop | adding one to the hot path changes what is being measured, and breaks [no-allocation / non-blocking discipline](001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) if done carelessly |
| `headroom` cost at `C` consumers | a `GatingSet` micro-benchmark | belongs in `ring_gating`, not here — leaf-proximate |
| CAS loop against a mutex | a same-shape harness for both | `MutexQueue` already exists at ring level; a primitive-level version would need a `Claimer`-shaped mutex wrapper that exists nowhere |
| false-sharing cost of the second line | `Claimer` at 128 vs a hypothetical packed 72 | the packed variant cannot be built — `PaddedCursor`'s alignment is the point |

The second row is the one that could be done today at low cost and in the right
crate. The others each require building the thing they would measure against.

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [001_nothing_allocates_nothing_waits_nothing_is_unsafe.md](001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) | The properties that hold at any load |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | Why the gate is re-read every attempt, which is what makes `k × C` |
| [../algorithm/002_two_loops_that_disagree_at_zero.md](../algorithm/002_two_loops_that_disagree_at_zero.md) | The zero-width success that issues a full compare-exchange |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | The two lines, one written |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The mutex refused for a non-performance reason |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_four_predicates_and_the_one_that_is_called.md](../integration/002_four_predicates_and_the_one_that_is_called.md) | `headroom`, the per-attempt cost |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_a_refused_claim_moves_nothing.md](../invariant/002_a_refused_claim_moves_nothing.md) | Why a refusal costs one load and no RMW |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:25-35` | The mutex refused, and on what grounds |
| `ring_claim/src/lib.rs:432-447` | The loop whose cost is derived above |
| `ring_gating/src/lib.rs:173-200` | `slowest` over `C` cursors, inside every attempt |
| `ring_bench/src/lib.rs:418-442,620-630` | The six candidates and what an `Outcome` records |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md § C1` | The crate's one measurement, and its subject |
| `tests/manual/readme.md § C6` | The contention constants, fixed so they cannot be quietly lowered |
| `tests/claim_test.rs:309` — `the_producer_cursor_occupies_its_own_cache_line` | The structural assertion standing in for a false-sharing measurement |
