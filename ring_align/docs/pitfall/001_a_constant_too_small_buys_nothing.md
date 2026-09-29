# Pitfall: A Constant Too Small Buys Nothing

### Scope

- **Purpose**: Record the crate's central trap — that a `CACHE_LINE` smaller than the host's real line size disables the padding entirely while every assertion in the suite continues to pass.
- **Responsibility**: State the trap, why the failure is silent rather than loud, the asymmetry between too-small and too-large, the mitigation, and the second copy of the constant that already exists in the family.
- **In Scope**: The 64-vs-128 platform split; the silent-failure mechanism; the duplicate literal in [`ring_mpsc`](../../../ring_mpsc/readme.md).
- **Out of Scope**: What the constant *is*, which is [`type/001`](../type/001_cache_line.md); how a port raises it, which is [`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md).

### The Trap

[`CACHE_LINE`](../type/001_cache_line.md) is 64. Apple Silicon's line is 128.
On that machine:

- `size_of::< CacheAligned< u64 > >()` is 64 — **the assertion passes**.
- `align_of::< CacheAligned< u64 > >()` is 64 — **the assertion passes**.
- Two wrapped fields land 64 bytes apart — **the assertion passes**.
- `on_distinct_lines( a, b )` divides by 64 and reports `true` — **the assertion passes**.
- The two fields are on **the same 128-byte cache line**, and every write by
  either still invalidates the other's copy.

**The entire test suite is green and the crate does nothing.** That is the trap:
not that the padding fails, but that nothing in the codebase is capable of
noticing it failed. The crate's own instrument — `on_distinct_lines` — is
calibrated to the same wrong number, so it agrees.

### Why the Failure Is Silent Rather Than Loud

Every check the crate can perform is *internally consistent*: the constant, the
attribute, and the predicate all say 64, and they agree with each other. The
missing fact is external — the host's real line size — and nothing in a Rust
build reads it.

This is the general shape of the trap and it is worth naming: **the crate
validates its own arithmetic, not its own premise.** A test suite of any size
built on the same constant adds no evidence about the premise, so growing the
suite does not shrink the risk.

### The Asymmetry: Too Small Is Dangerous, Too Large Is Merely Wasteful

| Constant | Host line | Result |
|---------:|----------:|--------|
| 64 | 64 | Correct |
| 64 | **128** | **Padding is decorative. Every assertion passes. False sharing continues** |
| 128 | 64 | Correct, at 2× the memory — one wasted line per padded value |

The bottom row is a cost the family explicitly accepts as irrelevant at these
counts — it costs memory that is irrelevant at these counts. The middle row is a defect that
presents as a performance mystery — the lock-free ring measuring slower than
the mutex it replaced, the outcome most likely to be blamed on the wrong thing.

**So the safe direction is up**, and that is why the module doc's stated plan is
that a future port *raises* the constant rather than making it conditional
(→ [`decisions/001`](../decisions/001_the_constant_is_not_conditional.md)).

### Mitigation

| # | Mitigation | Strength |
|---|-----------|----------|
| M1 | `tests/manual/readme.md` M1 reads the host directly: `getconf LEVEL1_DCACHE_LINESIZE` | Real, but manual — it runs when a person runs it |
| M2 | The module doc states the 128 case explicitly, at the constant's declaration | Documentation. It informs; it does not detect |
| M3 | One constant rather than a `cfg`, so a port changes one number | Real, and it is the reason the fix is cheap once the problem is known |
| M4 | An automated check comparing `CACHE_LINE` against `getconf` at build time | **Does not exist** |
| M5 | A padded-vs-unpadded throughput benchmark, which would fail to show a difference | **Does not exist here** — deferred to a future benchmark stage, and unmeasured (→ [`non_functional_requirement/001`](../non_functional_requirement/001_the_structural_claim_is_testable.md)) |

**M1 is the only mitigation that can actually detect the condition**, and it is
a human running a command. The manual plan records the reading rather than
adjusting the test, which is the right discipline: a suite that silently
retuned itself to the host would destroy the evidence that the constant was
wrong.

M4 is worth naming even though it does not exist, because it is the shape the
real fix would take — a `build.rs` that reads the host line size and fails the
build on a mismatch. It is mechanically checkable and it is not written.

### The Duplicate Already Exists

The crate's entire justification for being a crate is that the family cannot
acquire two independent answers to the same question
(→ [`pattern/002`](../pattern/002_one_owner_for_a_magic_number.md)).
[`ring_cursor`'s readme](../../../ring_cursor/readme.md) states it as achieved:

> The padding *decision* is not here — it is `ring_align`'s single
> `CACHE_LINE` — and keeping it there is what stops the family from acquiring
> two independent answers to the same question.

**It did not hold.** `ring_mpsc` carries a second copy:

```rust
// ring_mpsc/src/lib.rs, in Producer::on_distinct_lines
pub fn on_distinct_lines( &self ) -> bool
{
  let claim = self.claimer.cursor().addr();
  let consume = self.ring.consumer_cursor().addr();

  claim.abs_diff( consume ) >= 64
}
```

Verify the whole finding:

```sh
cd "$(git rev-parse --show-toplevel)"
# code only: `ring_mpsc`'s corpus names `ring_align` in prose, and a doc
# citing the crate is not the crate being used (→ `../invariant/002`)
command grep -r 'ring_align\|CACHE_LINE' --include=*.rs ring_mpsc/ | wc -l
sed -n '/\[dependencies\]/,/^\[/p' ring_mpsc/Cargo.toml # ring_align absent
awk '/pub fn on_distinct_lines/,/^  \}/' ring_mpsc/src/lib.rs   # the literal 64
```

Live output:

```
0
[dependencies]
ring_atomic = { path = "../ring_atomic" }
ring_store = { path = "../ring_store" }
ring_claim = { path = "../ring_claim" }
ring_config = { path = "../ring_config" }
ring_cursor = { path = "../ring_cursor" }
ring_gating = { path = "../ring_gating" }
ring_slot = { path = "../ring_slot" }
ring_types = { path = "../ring_types" }

# `tests/mpsc_test.rs`'s `exhaustive` module, under `RUSTFLAGS="--cfg loom"`
# only. `ring_atomic` swaps `AtomicSeq` for an instrumented one under the same
# cfg, so what loom explores is this crate's own stamp protocol rather than a
# re-implementation of it.
[target.'cfg(loom)'.dev-dependencies]
  pub fn on_distinct_lines( &self ) -> bool
  {
    let claim = self.claimer.cursor().addr();
    let consume = self.ring.consumer_cursor().addr();

    claim.abs_diff( consume ) >= 64
  }
```

Three distinct divergences, in one four-line function:

1. **A second constant.** The `64` is a literal. `ring_mpsc` does not depend on
   `ring_align`, so raising `CACHE_LINE` for an Apple Silicon port fixes
   `ring_cursor` and `ring_spsc` and leaves this one at 64 — the exact
   fragmentation the crate exists to prevent, and the reason M3 above is weaker
   than it looks.
2. **A different predicate under the same name.** This is `abs_diff >= 64`, the
   *subtraction* form; `ring_cursor` and `ring_spsc` resolve to the *division*
   form. Those disagree on real inputs
   (→ [`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md) § The Alternative That Looks Equivalent).
   Two crates in one family answer `on_distinct_lines` with two different
   questions.
3. **The divergence is invisible to the compiler.** Nothing here is a type
   error. Both return `bool`, both are named the same, and both are `true` in
   the common case.

**The divergence was not blind, which makes it more interesting rather than
less.** `ring_mpsc`'s own doc comment states the situation accurately
(`ring_mpsc/src/lib.rs:844-846`):

> The two cursors are in different allocations — one in the ring's gating set,
> one in the claimer — so this is a check on `PaddedCursor`'s alignment rather
> than on their layout relative to each other.

So the author knew the two cursors are not a pair, knew `CursorPair`'s method
was therefore unavailable, and deliberately asked a narrower question. And the
narrower question was reachable: `ring_cursor` re-exports only
`ring_atomic::SeqCell` (`ring_cursor/src/lib.rs:69`), so neither
`CACHE_LINE` nor the free function arrives through the existing
`ring_mpsc → ring_cursor` edge. Reaching them meant adding a dependency; a
literal did not.

**Which leaves a fourth divergence, and it is the one that costs something.**
For its stated purpose — detecting that `PaddedCursor` lost its alignment — the
check is a very weak detector:

- **While the padding is present, it cannot fail.** Two *distinct* 64-aligned
  addresses differ by a nonzero multiple of 64, hence by at least 64. The
  assertion is true by construction, in every allocation arrangement.
- **If the padding were removed, it would usually still pass.** Two 8-byte
  atomics in different heap allocations are commonly more than 64 bytes apart
  anyway. The assertion's outcome then depends on allocator placement, not on
  the type's alignment.

The regression the doc comment names — *"a sibling change that dropped the
alignment would cost this crate a contended line on its hottest path and break
nothing that compiles"* — is the one this check is least able to see. Note that
the division form would be no better here: two different allocations usually
land on different lines regardless of padding. **The problem is the different
allocations, not the arithmetic.** A check that could detect it would have to
assert on `PaddedCursor` itself — `align_of::< PaddedCursor >() == CACHE_LINE`
— which is `ring_cursor`'s assertion to make and which `ring_cursor` already
makes.

So the finding is: a second copy of a constant, under a shared name, computing
a third question, to run an assertion that is near-vacuous for the property it
documents. The duplication is the reportable part; the vacuity is what the
duplication bought.

This is filed as a finding rather than fixed here: the fix is a source change
in `ring_mpsc` — add the dependency, or drop the method in favour of the
alignment assertion that can actually fail — and belongs to that crate's own
change, with its own test and its own bug record.

### AL41 — The One Check Against the Machine Is a Manual Procedure With a Date on It

```
ring_align/tests/manual/readme.md:17:getconf LEVEL1_DCACHE_LINESIZE
ring_align/tests/manual/readme.md:75:| 2026-08-28 | M1 | ✅ | `getconf LEVEL1_DCACHE_LINESIZE` reports 64 on this host — the constant is right here. …
```

That is the only place in 33 crates where the declared constant is compared to
the host's real line size. It is a step a person runs and records.

**Finding.** The check exists, is written down, and is not a gate — so its ✅
will keep reading ✅ on a machine nobody has re-run it on, including the Apple
Silicon machine where it is the check that matters. Turning it into a gate is a
`getconf` and a comparison; what stops that today is that no declared gate owns
the family's platform assumptions.

---

### AL42 — The Suite Is Self-Consistent at Any Value the Constant Takes

Every assertion in `align_test.rs` compares the layout against `CACHE_LINE`
rather than against the host. Set the constant to 32, or to 4096, and the whole
suite still passes.

**Finding.** A wrong constant is invisible to the tests by construction — not by
oversight, since testing against a hard-coded 64 would be worse. That is exactly
why AL41's manual step is the only detector the crate has, and why its being
manual is the finding rather than a footnote: the automated layer cannot be
extended to cover this, so the gap is structural.

**Disposition:** declined — the finding's own conclusion is that this gap is
structural rather than an oversight: any in-suite check is necessarily built on
`CACHE_LINE` itself, so it can only ever validate the crate's arithmetic
against its own constant, never the constant against the host. This crate's own
[`decisions/001`](../decisions/001_the_constant_is_not_conditional.md) already
evaluated and rejected the one alternative that could reach the host truth at
all — A4, querying the line size at runtime — because the value is consumed by
`#[ repr( align( … ) ) ]`, which requires a compile-time literal (`E0693`) and
cannot accept a runtime-computed one. The one concrete automated mitigation
named above, M4 (a `build.rs` comparing `CACHE_LINE` against `getconf` at build
time), is recorded as a shape that "does not exist," not a recommendation to
add one — and adding it would introduce a non-portable, host-only build
dependency (`getconf` is not universally available, and a `build.rs` reads the
*build* host, which is the wrong machine under cross-compilation), which is a
build-infrastructure decision heavier than this documentation pass, not a
targeted fix to this finding.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_deciding_line_membership_by_division.md](../algorithm/001_deciding_line_membership_by_division.md) | The two forms `ring_mpsc` and `ring_cursor` have independently chosen between |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_reading_surface.md](../api/001_the_reading_surface.md) | The four-crate name table this finding qualifies |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_the_constant_is_not_conditional.md](../decisions/001_the_constant_is_not_conditional.md) | Why one number, and why the safe direction is up |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_one_constant_for_the_whole_family.md](../invariant/002_one_constant_for_the_whole_family.md) | The restriction this finding is a live violation of |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_constant_across_a_platform_port.md](../lifecycle/002_the_constant_across_a_platform_port.md) | The event that turns this from latent to active |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_the_structural_claim_is_testable.md](../non_functional_requirement/001_the_structural_claim_is_testable.md) | M5 — the measurement that would catch this, and why it is nobody's yet |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_owner_for_a_magic_number.md](../pattern/002_one_owner_for_a_magic_number.md) | The pattern this crate implements and `ring_mpsc` steps outside |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_cache_line.md](../type/001_cache_line.md) | The constant, and the one place the language forbids sharing it |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | "The outcome most likely to be blamed on the wrong thing" — the failure mode this trap produces |
| [`../decisions/001_the_constant_is_not_conditional.md`](../decisions/001_the_constant_is_not_conditional.md) | The memory-cost ruling that makes over-padding acceptable and under-padding not |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | M1 is the only check capable of detecting the condition; its 2026-08-28 run recorded 64 on this host |
| `tests/align_test.rs` | Every test here passes under the trap — that is the point, not an omission |
