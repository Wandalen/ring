# Integration: Why the Constant Lives Here

### Scope

- **Purpose**: Answer the question one declared consumer invites — why a crate at all, rather than a `const` at the top of `ring_cursor` — and report what the evidence actually says about whether the separation worked.
- **Responsibility**: State the alternative placements, the cost each carries, and the one measurement that tests the argument.
- **In Scope**: Placement of `CACHE_LINE`, `CacheAligned`, and `on_distinct_lines` within the family.
- **Out of Scope**: The edges themselves, which are [`integration/001`](001_one_dependency_one_consumer.md); the ownership restriction the placement is supposed to secure, which is [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md).

### The Question

`ring_align` has one consumer (→ [`integration/001`](001_one_dependency_one_consumer.md)).
A crate holding one constant, one 3-line struct, and one 1-line function, for
the benefit of exactly one other crate, has to answer why it is not just three
items at the top of that crate.

### Alternative Placements

| # | Placement | Cost |
|---|-----------|------|
| J1 | In `ring_cursor`, beside the cursors that use it | The concept becomes a detail of the cursor implementation. Anything else wanting the constant must depend on `ring_cursor`, which pulls `ring_types`, `ring_seqno`, and `ring_atomic` with it — four crates to reach a `usize` |
| J2 | In `ring_types`, with the other shared vocabulary | Defensible, and the closest call. `ring_types` is already a universal dependency, so the constant would be reachable everywhere for free. It loses the negative-control test, which needs a home where an *unpadded* struct is a legitimate thing to build (→ [`pitfall/002`](../pitfall/002_size_of_proves_nothing_about_addresses.md)) |
| J3 | In `ring_core`, as an implementation detail of the ring | Wrong direction — the constant is a platform fact, not a ring fact, and `ring_core` is downstream of the cursors that need it |
| J4 | **Its own crate** | Chosen |

**J2 is the alternative that would probably work**, and the argument against it
is weak enough to state plainly: `ring_types` is about the family's domain
vocabulary — capacities, sequence numbers, the error enum — and 64 is a fact
about the hardware. Keeping a hardware fact out of a domain-vocabulary crate is
a tidiness argument, not a technical one.

**And the family's own evidence runs against the tidiness argument.**
`ring_types` is a universal dependency and `ring_align` is not:

```sh
cd "$(git rev-parse --show-toplevel)"
# excludes an untracked, in-progress workspace-restructuring manifest
# (`ring/Cargo.toml`) and trybuild's scratch/build-output copies under
# `target/`/`-target_gate/` — none of the three is a real crate.
grep -rl 'ring_types' --include=Cargo.toml . \
  | command grep -vE '^ring/Cargo\.toml$|/target/|/-target' | wc -l   # 32 — every crate
grep -rl 'ring_align' --include=Cargo.toml . \
  | command grep -vE '^ring/Cargo\.toml$|/target/|/-target' | wc -l   # 2  — one consumer
```

Live output:

```
32
2
```

**Correction (2026-09-28):** the first line above used to read `33`. Commit
`ce60ae6e8` ("Remove unused dependencies from Cargo.toml files") dropped
`ring_align`'s own `ring_types` dependency, and `ring_align` was one of the
33 counted — so the universal-dependency count drops to 32 precisely because
of the crate this document is about. It does not weaken the argument below:
`ring_types` remains reachable from every crate that still needs it,
`ring_align` remains reachable from exactly one.

`ring_types` already runs this exact pattern for `RingError` — "one enum rather
than one per crate […] declared once at tier 0" (`ring_types/src/error.rs:3-7`)
— and that instance has **no duplicates anywhere in the family**, while this
one has one. The difference is not governance; it is that reaching `RingError`
costs nothing and reaching `CACHE_LINE` costs a manifest edit
(→ [`pattern/002`](../pattern/002_one_owner_for_a_magic_number.md) § What
Separates the Two Instances).

### What the Separation Was Supposed to Buy

`ring_cursor`'s readme states the intended payoff:

> The padding *decision* is not here — it is `ring_align`'s single
> `CACHE_LINE` — and keeping it there is what stops the family from acquiring
> two independent answers to the same question.

Three things follow from a separate crate that would not from J1:

1. **A name for the concept.** `ring_align` is greppable, and a developer
   wondering where padding is decided has somewhere to look. Inside
   `ring_cursor` the constant would be findable only by someone already reading
   the cursors.
2. **A place for the negative control.** `tests/align_test.rs` builds an
   unpadded two-field struct and asserts it *does* share a line. That test is
   the reason the padded assertion is evidence rather than a tautology
   (→ [`pitfall/002`](../pitfall/002_size_of_proves_nothing_about_addresses.md)),
   and it is a strange thing to find in a crate whose job is to be padded.
3. **A dependency edge that means something.** `ring_cursor → ring_align` in a
   manifest is a statement that this crate's layout is decided elsewhere. A
   `const` at the top of a file states nothing to anyone not reading the file.

### Whether It Worked

**It did not prevent the duplicate.** `ring_mpsc` recomputes the constant from
a literal (→ [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md)),
and it did so while already depending on `ring_cursor`, the one crate that has
the real constant.

So the honest reading is that the separation and the co-location would have
produced the same outcome here, for the same reason: `ring_cursor` re-exports
only `ring_atomic::SeqCell` (`ring_cursor/src/lib.rs:69`), so a
downstream crate reaches `CACHE_LINE` through neither placement without editing
its own manifest. **The barrier that mattered was the manifest edit, and both
placements have it.**

That does not make J4 wrong — points 1 and 2 above are real and J1 forfeits
both. It makes the *stated* payoff overclaimed: a separate crate makes the
ownership legible, and legibility is not enforcement. The enforcement gap is
Q4 in [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md),
and it is unbuilt under every placement.

**A re-export would have closed the specific gap that bit.** Had `ring_cursor`
re-exported `CACHE_LINE` alongside `SeqCell`, `ring_mpsc` would have had the
constant in scope with no manifest change at all, and the literal would have
been a strictly worse choice rather than the path of least resistance. That is
a change to `ring_cursor`, so it is recorded here rather than made.

### AL23 — A Crate Justified as Shared Infrastructure Currently Serves One Caller

```
ring_cursor/Cargo.toml:12:ring_align = { path = "../ring_align" }
```

One declared consumer, across 33 crates.

**Finding.** The argument for this crate's existence is therefore about the
*shape* of the dependency graph — that the number has exactly one home and
cannot be answered twice — rather than about reuse that has actually happened.
That is a real argument and it is the one this document makes; it is worth
stating that it is the only one available, because "shared infrastructure"
normally implies sharing, and here the sharing is potential rather than
observed.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_reading_surface.md](../api/001_the_reading_surface.md) | The name reached three crates the dependency edge did not — placement's real effect |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_the_constant_is_not_conditional.md](../decisions/001_the_constant_is_not_conditional.md) | "Wrong in exactly one place" — the property this placement is meant to secure |

### Integrations

| File | Relationship |
|------|--------------|
| [001_one_dependency_one_consumer.md](001_one_dependency_one_consumer.md) | The one-consumer measurement that raises the question |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_one_constant_for_the_whole_family.md](../invariant/002_one_constant_for_the_whole_family.md) | Q4, the enforcement no placement supplies |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_owner_for_a_magic_number.md](../pattern/002_one_owner_for_a_magic_number.md) | The general pattern, and the general limit — an owner is not a gate |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_size_of_proves_nothing_about_addresses.md](../pitfall/002_size_of_proves_nothing_about_addresses.md) | Point 2 — the negative control that needs this crate to exist |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/readme.md` | The stated payoff of the separation |
| `ring_cursor/src/lib.rs:69` | The single re-export, and the gap a second one would have closed |
| [`../../../README.md`](../../../README.md) | The 33-crate decomposition this placement is one instance of |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | The negative control that argues for a separate home |
| `tests/manual/readme.md` | No check tests the placement; the measurement in § Whether It Worked is the evidence |
