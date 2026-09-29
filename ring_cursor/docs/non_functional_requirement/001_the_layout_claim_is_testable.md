# Non Functional Requirement: The Layout Claim Is Testable

### Scope

- **Purpose**: State the requirement that this crate's performance claim be checkable without a performance measurement, and show the four-layer arrangement that satisfies it.
- **Responsibility**: Separate what each layer establishes, show why the cheap layers are what make the expensive one meaningful, and name the layer that does not exist yet.
- **In Scope**: The five layout tests, the two manual checks, and the benchmark they are a precondition for.
- **Out of Scope**: The clauses themselves, which is [`invariant/001`](../invariant/001_one_cursor_one_line.md).

### The Requirement

> The claim "these two cursors do not share a cache line" must be verifiable in a
> unit test, on any machine, in microseconds, without contention.

This is unusual for a performance-motivated design. The *reason* `PaddedCursor`
exists is throughput under contention, and throughput under contention is exactly
what a unit test cannot measure. The requirement says: separate the claim into a
part that is a fact about the program and a part that is a fact about a machine,
and make the first one carry as much as possible.

### The Four Layers

| Layer | Establishes | Instrument | Cost | Machine-dependent? |
|:-----:|-------------|------------|------|:------------------:|
| 1 | `size_of`, `align_of` | `a_padded_cursor_occupies_exactly_one_cache_line` | Compile-time constants | No |
| 2 | Real addresses are separated | `two_cursors_in_one_struct_…`, `every_cursor_starts_on_a_line_boundary`, `the_gap_survives_…` | Runtime, deterministic | No |
| 3 | The **stride** across a slice | `an_array_of_cursors_gives_each_its_own_line` | Runtime, deterministic | No |
| 4 | Throughput improves | `ring_bench` | A contended benchmark | **Yes** |

Layers 1–3 are 22 tests' worth of file for five tests, run in microseconds, and
give **no** information about speed. Layer 4 is the claim anyone actually cares
about.

### Why the Cheap Layers Earn Their Place

A benchmark that shows no improvement has two explanations, and they need
different responses:

| Explanation | What to do |
|-------------|------------|
| The layout is not what we think it is | Fix the code |
| The layout is right and separation does not help *here* | Accept it, or change the workload |

**Without layers 1–3 the benchmark cannot distinguish them.** With them, the
first explanation is already excluded before the benchmark runs, so a null result
means the second — which is a finding rather than a mystery.

That is the whole argument for the layering, and the test file states it:

> What this suite establishes is that the layout the measurement will be taken on
> is actually the layout claimed.

### Layer 3 Is the One That Is Not Redundant

Layers 1 and 2 are close to the same statement. Layer 3 is not:

| Counter-example | Layer 1 | Layer 2 | Layer 3 |
|-----------------|:-------:|:-------:|:-------:|
| `size_of` 8, `align_of` 64 | partly ✅ | ✅ for two fields | **❌** — array elements land 8 apart |

`ring_gating::GatingSet` holds a `Vec< PaddedCursor >` and reads every element on
every producer claim. **The array case is the one that actually runs in the hot
path**, and only layer 3 covers it.

### The Structural Layer the Table Omits

Layers 1–4 all observe *values*. Two defects produce correct values by an
incorrect route, and need a fifth kind of check that reads the source:

| Check | Reads | Catches |
|-------|-------|---------|
| `tests/manual/readme.md` M1 | `grep` for a literal `64` in `src/` | The constant being forked — [`pitfall/001`](../pitfall/001_the_obvious_implementation_forks_the_constant.md) |
| `tests/manual/readme.md` M2 | The struct declaration, by eye | A field hidden in the padding — `size_of` stays 64 either way |

**Both are humans running commands, and that is the honest cost of the
requirement.** A claim about *how* a value was obtained cannot be asserted by a
test that observes the value. M1 and M2 exist because the alternative is no check
at all, not because a better instrument was declined.

### What Layer 4 Would Need

It does not exist yet. What it would need, beyond the benchmark itself:

| # | Precondition | Status |
|---|--------------|--------|
| B1 | The layout is what is claimed | ✅ layers 1–3 |
| B2 | A contended workload with a known-false baseline — an unpadded pair to compare against | Not built |
| B3 | A machine whose cache line is actually 64 bytes | Assumed, unchecked — `ring_align`'s own `pitfall/001` owns this |
| B4 | Variance control — a loaded CI box produces noise larger than the effect | Not addressed |

**B2 is the interesting one.** Measuring the padded version alone shows a number,
not an improvement. The comparison needs a deliberately-unpadded pair built for
the benchmark, which means the benchmark crate has to construct something this
crate makes impossible — and that is a design consequence worth knowing before
someone tries.

### What the Requirement Does Not Ask For

| # | Not required | Why |
|---|--------------|-----|
| R1 | That the tests run under contention | Layer 4's job; a contended unit test is a flaky unit test |
| R2 | That timing appear in any assertion here | A timing assertion fails on a loaded machine for reasons unrelated to the code |
| R3 | That the layout hold on every target | `ring_align::CACHE_LINE` owns the target question |

**R2 is a deliberate exclusion and the test file says so out loud.** It is worth
recording as a requirement rather than an omission, because "add a timing
assertion" is a change someone will eventually propose.

### CU33 — The Requirement Is Met; the Property It Buys Is Measured Elsewhere

```
55:fn a_padded_cursor_occupies_exactly_one_cache_line()
```

Three assertions across two tests establish the layout. What the layout is *for*
— throughput that does not collapse as producer count rises — is not measured
here at all.

**Finding.** That measurement lives in `ring_bench` and is a different crate's
requirement. So this crate can satisfy its layout requirement completely while
the padding fails to deliver anything, and no instrument in this crate would
report it. The requirement is testable because it was written as a property of
bytes rather than of behaviour, which is exactly why it is met.

---

### CU34 — The Crate's Only Test File Observed No Allocation, and a Second One Was Needed to Fix That

```
occurrences of 'alloc' in tests/cursor_test.rs: 0
```

That zero still stands, and for as long as `cursor_test.rs` was the crate's only
test file it described the whole crate: neither requirement in this definition
had an allocation-observing test, the one that is met being checked by `size_of`
and the one that was broken by nothing at all. What closed the gap is a second
file — `tests/allocation_test.rs` — rather than a change to this one.

**Finding.** The zero was the honest summary of the split while it lasted.
`size_of` is a compile-time question a unit test can ask; "did this call reach
the allocator" is a runtime question needing an instrument, and the instrument
was recorded here as forbidden — which it never was. Gate G6 scans `src/` only,
so a test-only counting allocator was always within reach; see
[`002`](002_the_gating_read_allocates_nothing.md) CU35.

---

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_cursor_one_line.md](../invariant/001_one_cursor_one_line.md) | The three clauses layers 1–3 verify |
| [../invariant/002_the_number_64_never_appears_here.md](../invariant/002_the_number_64_never_appears_here.md) | What M1 checks, stated as a restriction |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [002_the_gating_read_allocates_nothing.md](002_the_gating_read_allocates_nothing.md) | The requirement this crate broke for as long as anybody was reading it, and the measurement it was said to be unable to take |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_obvious_implementation_forks_the_constant.md](../pitfall/001_the_obvious_implementation_forks_the_constant.md) | The change all four layers pass |
| [../pitfall/002_a_reading_that_consults_one_cursor.md](../pitfall/002_a_reading_that_consults_one_cursor.md) | The other defect that needs a structural check |

### Sources

| File | Relationship |
|------|--------------|
| `bench_harness/docs/acceptance/001_feature_reached_tests.md` | Where the three clauses are stated |
| `ring_cursor/src/lib.rs:23-36` | The crate's own argument for asserting layout rather than timing |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:1-31` | The suite's statement of the layering and R2 |
| `tests/cursor_test.rs:54-72` | Layer 1 |
| `tests/cursor_test.rs:74-111` | Layer 2 |
| `tests/cursor_test.rs:113-126` | Layer 3 — the array stride |
| `tests/manual/readme.md` M1, M2 | The structural layer |
