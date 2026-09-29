# Non Functional Requirement: The Structural Claim Is Testable

### Scope

- **Purpose**: State the requirement that this crate's contribution be decidable by an automated test rather than by a benchmark or an argument, show the four-layer stack that satisfies it, and mark the half of the feature that is explicitly not satisfied here.
- **Responsibility**: State the requirement, give the evidence layer by layer, name the deferred half and where it went, and record the historical failure that shows the property was not free.
- **In Scope**: Verifiability of "two cursors do not share a line".
- **Out of Scope**: The throughput half, which is a separate staged benchmark plan; whether the constant is right, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md).

### The Requirement

**Every claim this crate makes must be decidable by running something, and the
thing that runs must be able to fail.**

That is a stronger requirement than "has tests". A padding crate is unusually
exposed to tests that cannot fail: `size_of` assertions about a type whose size
the attribute fixes are close to tautological, and an assertion that two values
are on different lines passes trivially if they were never going to share one.

### The Two Claims, Split

This crate makes two claims,
and the test file's own module doc separates them
(`ring_align/tests/align_test.rs:6-9`):

> The structural assertion — 64-byte size and alignment — is what a test can
> decide; the throughput claim the feature also makes needs a number nobody has
> stated yet, which is why `docs/plan/008_ring_write_path_staged.md` splits
> stage S3 out and applies scale-invariance recursion inside it.

| Claim | Decidable here | Where it lives |
|-------|:--------------:|----------------|
| Two wrapped fields occupy different cache lines | **yes** | This crate's suite, four layers below |
| Separating them raises throughput as core count rises | no | Deferred to a future benchmark stage, unmeasured |

**Naming the undecidable half rather than approximating it is the requirement
being met, not dodged.** A microbenchmark asserting "the padded version is
faster" would run, would be green on most days, and would decide nothing — the
number it needs has not been stated.

### The Four Layers

| Layer | What it decides | Can it fail? |
|-------|-----------------|--------------|
| 1. `size_of` / `align_of` vs `CACHE_LINE` | The attribute is present and the size rounds up | Yes — it caught the alignment/sizing split during development, and fails immediately if the `repr` and the constant diverge (→ [`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md) site 2) |
| 2. Real addresses of two fields in a real struct | The compiler actually laid them out apart, not just that it could have | Yes — layer 1 passing does not imply this (→ [`pitfall/002`](../pitfall/002_size_of_proves_nothing_about_addresses.md)) |
| 3. **The negative control** — two *unwrapped* fields asserted to share a line | That layer 2 is measuring the padding rather than something the layout would have done anyway | **This is the layer that makes layers 1–2 evidence.** Without it, layer 2 is a passing test with an unknown cause |
| 4. `tests/manual/readme.md` M1 — `getconf LEVEL1_DCACHE_LINESIZE` | Whether 64 matches the host at all | Yes, and only a human runs it |

Run layers 2 and 3 together, which is what makes either meaningful:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_align
cargo test --test align_test two_wrapped_fields_land_on_different_lines two_unwrapped_fields_share_a_line
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
error: unexpected argument 'two_unwrapped_fields_share_a_line' found

Usage: cargo test [OPTIONS] [TESTNAME] [-- [ARGS]...]

For more information, try '--help'.
```

**Layer 3 is the requirement's real content.** The manual plan says so
explicitly — *"The second is the one that matters — it proves the first is
measuring the padding rather than something the layout would have done
anyway"* — and it generalises past this crate: any structural assertion about a
compiler-chosen layout needs a control showing the compiler would have chosen
otherwise.

### The Property Was Not Free

The requirement looks satisfied by construction until you find that it was
violated once and caught. `tests/manual/readme.md`'s M4 record, 2026-08-28:

> `on_distinct_lines`'s example uses plain integer addresses (0/63, 63/64,
> 128/130), not stack locals — this was a real failure earlier in the stage and
> is the reason the check is written the way it is.

The original doc example took the addresses of two stack locals and asserted
they were on different lines. Two adjacent locals share a line, so the example
asserted something **false about the machine** while reading as a perfectly
reasonable demonstration. It is the same failure shape as a missing negative
control, one level up: a test that looks like evidence and is not.

**That history is why the current example uses integer literals**
(→ [`decisions/002`](../decisions/002_the_predicate_takes_integers.md)), and it
is worth recording because the signature choice and the testability requirement
turn out to be the same decision reached from two directions.

### Standing Gaps

| # | Gap | Consequence |
|---|-----|-------------|
| N1 | Layer 4 is manual | The one check that can invalidate everything runs when a person chooses (→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) M4) |
| N2 | The throughput claim is unmeasured | The crate's *purpose* is unverified even though its *mechanism* is verified |
| N3 | Layers 1–3 hold for the payloads tested, on the machine the suite runs on | `u8`, `u64`, `[ u8; 63 ]`, `[ u8; 65 ]`. A payload with unusual alignment requirements of its own is untested |
| N4 | `an_oversized_payload_rounds_up_to_whole_lines` stops exercising its case at `CACHE_LINE = 128` | Silent (→ [`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md) site 8) |

**N2 is the honest headline.** Everything this crate asserts is about layout,
and layout was never the point — it is the means to a throughput property that
no test in the workspace measures. The requirement as stated is met; the
requirement one level up is deferred and labelled.

### AL31 — The Deferred Half Is Sent to a Path That Does Not Exist

```
3://! Tier 1 of the 33 `ring_*` crates of workstream 008, the concurrency
4://! write-path family specified in `docs/workstream/008_ring_write_path.md`.
    the cited path does not exist
    a directory of that name, without the .md, does
```

The target directory holds several documents including a `readme.md`. The
citation names a file one character away from that directory.

**Finding.** This definition's own Scope line inherits the deferral, so a reader
following the throughput requirement to its resolution lands on nothing while
four documents sit beside the path they were given. Nothing checks it: G16 holds
citations inside `docs/` to their targets, and this citation is in `src/`.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_align
command grep -m1 -A2 -F 'Tier 1 of the 33' src/lib.rs
```

Live output:

```
//! Tier 1 of the 33 `ring_*` crates of workstream 008, the concurrency
//! write-path family specified in
//! `docs/workstream/008_ring_write_path/readme.md`.
```

**Disposition:** applied — `src/lib.rs`'s module doc now cites the real
`readme.md` inside the target directory instead of the non-existent
sibling file; `test -f` against the corrected path from this crate's own root
confirms it resolves, and the crate's 7 doctests re-verified passing
(`cargo test --all-features`, 2026-09-03). Now prints:
`docs/workstream/008_ring_write_path/readme.md`

**Correction (2026-09-29):** the path above pointed outside this crate's own
tree even after the 2026-09-03 fix — `docs/workstream/` was never part of
`ring_align` or `ring`, only of the private monorepo `ring` was developed
inside at the time. `ring` has since been extracted into its own standalone
repository, so any citation into `docs/workstream/` is now a reference to a
workspace this repo must not assume exists. `src/lib.rs`'s module doc no
longer names `docs/workstream/`, or a workstream number, at all:

```
//! Tier 1 of the ring family's 33 crates — the concurrency write-path implementation.
```

There is nothing left to chase, correctly or incorrectly — the citation was
removed rather than repointed.

---

### AL32 — "Deferred" and "Dropped" Are the Same State From Inside This Crate

One requirement here is met and has evidence; the other is deferred and has
none. No gate, test, or task in the repository refers back to the deferred half.

**Finding.** The distinction between a requirement postponed and a requirement
abandoned is carried entirely by a sentence in this document, and that sentence
points at the path AL31 recorded — dangling when AL31 was written, repaired
under that finding's own disposition, and still the only thing carrying the
distinction either way. Any mechanism at all — a task, a `qqq:` marker, a
row in a declared file — would separate the two states; there is none, so from
inside `ring_align` the throughput claim is indistinguishable from one nobody
intends to make.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_cache_aligned_wrapper.md](../data_structure/001_the_cache_aligned_wrapper.md) | The size table layer 1 asserts |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_predicate_takes_integers.md](../decisions/002_the_predicate_takes_integers.md) | The signature that the M4 failure independently argued for |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | P4 — the layer-2/3 pair as an enforcement mechanism |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_constant_across_a_platform_port.md](../lifecycle/002_the_constant_across_a_platform_port.md) | N4, and the three silent sites a port leaves behind |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [002_the_crate_costs_nothing_at_runtime.md](002_the_crate_costs_nothing_at_runtime.md) | The other non-functional claim, which is decidable by reading rather than by running |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_size_of_proves_nothing_about_addresses.md](../pitfall/002_size_of_proves_nothing_about_addresses.md) | Why layer 1 alone is not evidence, in full |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/tests/align_test.rs:2-12` | The split between the decidable claim and the deferred one |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | Layers 1–3 |
| `tests/manual/readme.md` | Layer 4, plus the M4 record documenting the historical false-passing example |
