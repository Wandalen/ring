# non_functional_requirement

Everything countable about this crate is in 001, and everything uncountable is in
002. The split is exact: five of the six methods cost a fixed number of atomic
operations, and the sixth costs however long a peer producer takes to finish
writing a slot — a quantity built out of three terms, none of which this crate
can see.

The other thing both files record is that none of the numbers is a measurement.
They are counts read off the source, and they will stay that way, because every
path through the family's benchmark apparatus routes around this crate for the
same reason every path through the family does.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [What a Publication Costs](001_what_a_publication_costs.md) | PB29, PB30 — bytes, atomics and allocations per instance and per operation, the five benchmark candidates none of which reaches here, and the core-only property that is true in fact and claimed nowhere |
| 002 | [What the Spin Costs](002_what_the_spin_costs.md) | PB31, PB32 — one iteration exactly, the three unknowable terms the total depends on, the seven crates whose tests look at a clock, and the longest deliberate spin in the repository |

### Per Operation

| Method | Atomic ops | Ordering | Cost |
|--------|-----------:|----------|------|
| `new` / `default` | 0 | — | one 64-byte zeroed write |
| `cursor` | **0** | — | a field offset — `const`, compiles away |
| `published` | 1 load | `Acquire` | O(1) |
| `is_published` | 1 load | `Acquire` | O(1) — the comparison is free relative to the load |
| `try_publish` | 1 RMW | `Release` / `Acquire` | O(1) |
| `publish` | *k* RMW | same | **not a number** |

Per instance: 64 bytes, 64-aligned, of which 8 are live and 56 are padding; zero
heap allocations, ever. The last row of the table is the entire subject of 002.

### The Three Terms of *k*

```
iterations  ≈  ( time until the predecessor publishes )  /  ( cost of one iteration )
```

| Term | Determined by | Visible to this crate |
|------|---------------|:---------------------:|
| Time until the predecessor publishes | the peer's payload write — arbitrary caller code | **no** |
| Cost of one iteration | CAS latency under contention: core count, interconnect, cache state | **no** |
| Predecessors ahead of you | claims taken and not yet published | **no** |

None is a parameter, a field, or an argument — `Publisher` holds eight bytes and
the frontier encodes none of them. That is why there is no budget: a budget would
have to be supplied by a caller who also cannot compute it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the benchmark candidates, and the crates the harness declares
command grep -m1 -A24 -F 'pub enum Candidate' ring_bench/src/lib.rs
grep -n 'path = ' ring_bench/Cargo.toml
ls -d ring_*/benches 2>/dev/null

# what this crate names from core, and what the family declares
grep -nE '\bstd::' ring_publish/src/lib.rs
grep -ohE 'core::[a-z_:]+' ring_publish/src/lib.rs | sort -u
grep -rln 'no_std' ring_*/src/*.rs
for f in ring_*/src/lib.rs; do grep -qE '\bstd::' "$f" || echo "$f"; done

# which crates' tests look at a clock
grep -rlnE 'Instant|Duration|elapsed' ring_*/tests/*.rs
grep -rnE 'Instant|Duration|elapsed|sleep|yield_now' ring_publish/tests/*.rs
```

| | Value |
|--|------:|
| `size_of` / `align_of` | 64 / 64 |
| Live state | 8 bytes |
| Padding | 56 bytes (87.5%) |
| Heap allocations | **0**, ever |
| Methods costing zero atomics | 1 |
| Methods costing exactly one | 4 |
| Methods whose cost is not a number | **1** |
| Benchmark candidates in `ring_bench` | 5, or 6 with `crossbeam` |
| …reaching `ring_publish` | **0** |
| Path dependencies `ring_bench` declares | 9 |
| …that are this crate | 0 |
| `benches/` directories in the family | **0** |
| `std::` paths in `src/lib.rs` | **0** |
| `core::` paths | 2 |
| Crates naming no `std::` path in `lib.rs` | 25 of 33 |
| Crates declaring `#![no_std]` | **3** |
| Crates whose tests look at a clock | 7 |
| …this crate | no |
| Scheduling primitives in this crate's tests | 1 — `yield_now`, at 3 sites, all progress nudges |
| Longest deliberate spin | 1 000 `yield_now` rounds, as a safety probe |
| Most producers ever run at once | 4 |
| Largest run | 20 000 items through 16 slots — untimed |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB29 | `ring_bench` | n/a — coverage | The family built a benchmark crate with five candidates and nine path dependencies, and this crate is in none of them; there is no `benches/` directory anywhere in the 33 |
| PB30 | family | n/a — observation | 25 of 33 crates name no `std::` path and three declare `#![no_std]`; absence of the path is not compatibility, since `Vec`/`Box`/`String` arrive through the prelude unprefixed, and the property is settled by compiling for a bare-metal target rather than by grepping |
| PB31 | `ring_publish` | n/a — observation | The iteration count is a function of three terms and not one is a parameter, a field, or an argument; the worst case is a peer's payload write rather than a queue depth, because *k* predecessors write concurrently |
| PB32 | family | n/a — observation | Seven crates' tests look at a clock and this one names no clock type at all, which is correct: with no budget there is nothing a clock could assert, and any duration promised would be a claim about caller code |
