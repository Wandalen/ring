# Decision: The Constant Is Not Conditional

### Scope

- **Purpose**: Record why `CACHE_LINE` is a single unconditional `64` rather than a value selected per target architecture, given that the crate's own doc comment names a platform where 64 is wrong.
- **Responsibility**: State the alternatives, the reason the unconditional form won, the cost it accepts, and the condition that reopens the question.
- **In Scope**: The `cfg`-per-architecture alternative and the runtime-query alternative.
- **Out of Scope**: What the wrong value costs, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md); the port itself, which is [`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md).

### The Decision

```rust
pub const CACHE_LINE : usize = 64;
```

One value, no `cfg`, no feature flag, no runtime query. The crate's own doc
comment states the choice and concedes the case against it in the same
sentence (`ring_align/src/lib.rs:27-30`):

> 64 on x86-64 and on AArch64's common configuration. Apple Silicon uses 128,
> and a value too small is the failure that matters — two cursors 64 bytes
> apart still share a 128-byte line — so a future port raises this rather than
> making it conditional per crate.

**Note what the doc comment actually rules out: conditionality *per crate*.**
It does not rule out `cfg`-per-architecture in this crate — that alternative is
open, and this instance exists to record why it has not been taken.

### Alternatives

| # | Alternative | Why it lost |
|---|-------------|-------------|
| A1 | `#[ cfg( target_arch = "aarch64" ) ]` selecting 128 | Correct on Apple Silicon, **wrong on every other AArch64** — Graviton, most Android, the Raspberry Pi 4 all use 64. `target_arch` is the wrong axis; the line size is a microarchitecture property and Rust exposes no `cfg` for it. Would double the padding cost on every server ARM target to fix one desktop one |
| A2 | `cfg( target_vendor = "apple" )` plus `target_arch = "aarch64"` | Narrower and closer to correct, but still a proxy — it encodes "Apple's ARM chips have 128-byte lines" as a build-time fact, which is true today and is not a guarantee. It also does not survive the next vendor that ships 128 |
| A3 | A Cargo feature (`cache-line-128`) | Moves the decision to whoever builds, which is the right *place* — but a feature that changes a type's layout is not additive, so two crates in one graph enabling it differently is a silent ODR-style hazard rather than a build error. Cargo's feature unification makes this actively dangerous for a layout constant |
| A4 | Query the line size at runtime | Cannot work. The value is needed by `#[ repr( align( … ) ) ]`, which is resolved at compile time and requires a literal (`E0693`, → [`type/001`](../type/001_cache_line.md)). A runtime value could only be *checked* against the compiled one, not used |
| A5 | **One unconditional constant, raised by hand at port time** | Chosen. Wrong on one known platform, right on every platform the family currently targets, and wrong in exactly one place |

**A4 is the constraint that shapes the whole decision** and it is easy to miss.
Even if a perfect runtime answer were available, the padding is baked into the
type's layout by an attribute that will not accept a computed value. Anything
adaptive would have to be a *check* that panics on a machine the binary was not
built for — which is a different feature, and a worse one than being wrong
quietly.

### What the Decision Costs

On a 128-byte-line machine every guarantee in this crate still holds as stated
and buys nothing: two wrapped fields are 64 bytes apart, on different 64-byte
units, in the same real line. Every test passes. The contention the crate
exists to remove is present.

That cost is accepted because it is **bounded and locatable**: one `const` line
and one `repr` literal, both in this file, both named by
[`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md). The
alternatives above each trade that for a cost that is unbounded (A1, A2 —
wrong on targets nobody tested), unsound (A3), or impossible (A4).

**The decision is only as good as the ownership claim it rests on.** "Wrong in
exactly one place" is true if and only if
[`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md) holds
— and it does not: `ring_mpsc` carries a second copy. So today the honest
statement is *wrong in exactly two places, one of which nobody would think to
look at.* The decision is still the right one; its premise needs repair.

### Reopening Conditions

| # | Condition | Response |
|---|-----------|----------|
| E1 | The family targets Apple Silicon for a benchmark or a shipped build | Raise the constant to 128 and the `repr` literal with it. Everything gets bigger and nothing else changes |
| E2 | The family must run on two line sizes **in one binary** | The decision genuinely fails. The response is not a bigger constant but a different design — dynamic padding, which means abandoning `repr( align )` and hand-computing padding, which means `unsafe` and a workspace lint exemption |
| E3 | A benchmark shows the 64→128 doubling costs measurably in cache footprint | Reopens A1/A2 with data. Not before — the current position is the conservative one and pays for itself |

**E2 is the one that would actually invalidate this decision** rather than
adjust it, and it is worth naming because nothing about the current design
survives it. Everything in this crate assumes the line size is known when the
type is compiled.

### AL13 — Unconditional Against a Family That Uses `cfg` Freely

```
  -- cfg-gated alternatives in this crate --
0
  -- control: the family does gate on cfg elsewhere --
ring_atomic/src/lib.rs
ring_bench/src/lib.rs
ring_core/src/lib.rs
```

Zero conditionals here, three crates using them one directory over. The absence
is a choice rather than an unavailable tool.

**Finding.** What the choice buys is one edit site for a port. What it sells is
correctness on Apple Silicon, where the real line is 128 and this constant is
knowingly wrong — a trade made explicitly and recorded, which is the difference
between this and an oversight.

---

### AL14 — The Direction of the Error Is the Whole Risk, and It Is Not Visible Here

A `CACHE_LINE` that is too *large* wastes memory and still separates. One that
is too *small* silently stops separating anything. 64 on a 128-byte-line machine
is the second case.

**Finding.** The declaration at `src/lib.rs:36` shows a number and a type; it
does not show that the two directions of error have wildly different costs. The
module doc says so in prose a few lines up, and nothing in the code or the tests
encodes it — which is why a port that lowers the value for a target with smaller
lines would pass every check while removing the guarantee entirely.

`tests/align_test.rs` now pins the direction as a floor, distinct from the
existing test that pins the exact value:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_align
cargo test --test align_test cache_line_must_not_shrink_below_the_current_known_minimum 2>&1 \
  | command grep -E '^test .+ \.\.\.|^test result:' | sed -E 's/; finished in .*/; finished/'
```

Live output:

```
test cache_line_must_not_shrink_below_the_current_known_minimum ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished
```

**Disposition:** applied — added
`cache_line_must_not_shrink_below_the_current_known_minimum` to
`tests/align_test.rs`, asserting `CACHE_LINE >= 64` with a message naming the
asymmetry and pointing at `docs/pitfall/001`. A deliberate raise (E1) still
passes; only a decrease trips it, which is the one direction of change this
crate's own decision record treats as unreviewed-and-dangerous rather than
merely costly. This encodes the asymmetry in the tests, closing the specific
gap the finding names, without touching the runtime-query alternative (A4)
this same file already rules out. The crate's 9 unit tests plus 7 doctests
re-verified passing (`cargo test --all-features`, 2026-09-04). Now prints:
`test cache_line_must_not_shrink_below_the_current_known_minimum ... ok`

### Decisions

| File | Relationship |
|------|--------------|
| [002_the_predicate_takes_integers.md](002_the_predicate_takes_integers.md) | The other choice with a live alternative |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_why_the_constant_lives_here.md](../integration/002_why_the_constant_lives_here.md) | Why one place, given that only one crate consumes it |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_one_constant_for_the_whole_family.md](../invariant/002_one_constant_for_the_whole_family.md) | The premise "wrong in exactly one place" depends on, and its live violation |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_constant_across_a_platform_port.md](../lifecycle/002_the_constant_across_a_platform_port.md) | E1 executed step by step |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_constant_too_small_buys_nothing.md](../pitfall/001_a_constant_too_small_buys_nothing.md) | The accepted cost, in full |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_cache_line.md](../type/001_cache_line.md) | The declaration, and the `E0693` bind that rules A4 out |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/src/lib.rs:27-30` | The doc comment stating the decision and conceding its cost |
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The requirement the constant serves |

### Tests

| File | Relationship |
|------|--------------|
| `src/lib.rs` doctest | `assert_eq!( ring_align::CACHE_LINE, 64 )` — pins the chosen value, so a change is deliberate rather than incidental |
| `tests/manual/readme.md` | M1 is the human check that the value is right for the machine, which no automated test can be |
