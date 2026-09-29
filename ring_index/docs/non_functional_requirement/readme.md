# non_functional_requirement

What the crate costs, as opposed to what it computes. There are only two costs
worth measuring in 17 lines of code: how many cycles the fold takes against the
division it replaces, and how much `run` allocates. Both were measured; both
contradict something the crate says about itself.

The measurements exist only because they were taken for this document, with a
throwaway crate built outside the workspace and deleted afterward. Nothing in
the repository re-runs them, so every number here is a snapshot rather than a
guarantee — see
[`integration/002`](../integration/002_the_feature_it_implements_half_of.md)
IX14 for why the family's one measurement crate does not cover this.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_what_the_fold_costs.md) | What the Fold Costs | Mask vs modulo, three runs, against the module comment's 20–40 cycle claim |
| [002](002_the_one_allocation_and_the_zero_callers.md) | The One Allocation and the Zero Callers | `of` and `aliases` at zero allocations; `run` at one per call, exactly sized |

## The Claim Is Right in Direction and Wrong in Magnitude

A runtime-divisor `%` measured 6.1–6.2 cycles across three runs; the fold
measured 0.9. That is a 7× ratio, on a machine that is `aarch64` — while the
module comment states 20–40 cycles and names x86. The constraint is worth its
cost; the number written down to justify it was not measured anywhere.

The more useful measurement is the one that looks like a refutation. `% 1024`
with a literal divisor costs 0.000 ns, because the compiler emits the mask
itself. Benchmarked that way the constraint looks worthless. It is not, because
capacity in this family always arrives at runtime from a `RingConfig` and is
opaque to the optimizer at every real call site.

## Two Functions Belong on a Hot Path and One Does Not

`of` and `aliases` allocate nothing across 10,000 calls each. `run` allocates
once per call, `8 × count` bytes, with `capacity == len` — the tightest a
`Vec`-returning function can be, and still disqualifying for a lock-free write
path.

The cost is never actually paid: nothing in the family calls `run`. The one
crate with the use case wrote a non-allocating iterator instead, which is why
the profile nobody ran would have shown nothing anyway.

### Regenerate

The probe crate is not kept in the repository. To reproduce: build a crate
depending on `ring_index` and `ring_types` with a counting global allocator,
time `of` against `%` with the capacity behind `std::hint::black_box`, and count
allocations across `of`, `aliases`, and `run` at several counts. The quoted
outputs in each instance name the section they came from.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '//! constraining capacity to a power of two is that this fold is then a bitmask' ring_index/src/lib.rs
uname -m; rustc --version
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX21 | `ring_index` | **misleading doc** | The fold is ~7× cheaper than a runtime-divisor modulo, not the 22–44× implied by "20–40 cycles"; the cited architecture is not the one this builds on |
| IX22 | `ring_index` | **measured cost** | A constant-divisor `%` compiles to the same mask and costs 0.000 ns — the naive benchmark of this claim measures nothing, and the comment does not warn of it |
| IX23 | `ring_index` | **measured cost** | `run` allocates once per call at exactly `8 × count` bytes, which is optimal for its return type and still rules it out of every path the crate exists to serve |
| IX24 | `ring_index` | n/a — observation | The allocation is never paid: `run` has no caller, so it has never appeared in a profile |
