# Decision: Gating Is Fixed, Not a Parameter

### Scope

- **Purpose**: Record why the gating reads take no `Ordering` argument, and measure the constant's claim to be the family's single statement of that decision against the family.
- **Responsibility**: Give the decision, the alternatives, the four-site measurement, and the structural reason the count is four rather than one.
- **In Scope**: `GATING` at `src/lib.rs:71-89`; every non-doc `Ordering::Acquire` in `ring_*/src/`.
- **Out of Scope**: What `Acquire` guarantees, which is [`type/001`](../type/001_gating.md); the `SeqCell` half of the surface that *does* take a parameter, which is [`api/001`](../api/001_the_surface_that_forwards.md).

### The Decision

`CursorPair`'s three readings and the `slowest` fold load at
`Ordering::Acquire`, named once as `pub const GATING`, and no caller may choose
otherwise. The argument, from `src/lib.rs:45-51`:

> A caller asking "may I claim?" is about to overwrite a slot on the answer; a
> `Relaxed` load there would let it act on a stale barrier and overwrite a slot
> the consumer had not finished with. There is no legitimate second option to
> offer, so offering one would only be a way to get it wrong.

**This is the opposite of what `ring_atomic` does one tier down**, and the
contrast is the crate's central tension. `ring_atomic::SeqCell` refuses to
choose, because for a bare cell the caller genuinely has a choice and a
defaulted `SeqCst` would make every benchmark meaningless. `PaddedCursor`
inherits that refusal through its own `SeqCell` impl. `CursorPair` overrides it.
One crate, two opposite ordering policies, and both are argued in the module
documentation because a reader who meets only one concludes the other is a bug.

### Alternatives

| # | Alternative | Why it lost |
|---|-------------|-------------|
| A1 | `fn free_slots( &self, order : Ordering )` | Offers a choice with exactly one correct answer. Every caller writes `Acquire`; the one that writes `Relaxed` has a data race the type system endorsed |
| A2 | Default to `Acquire`, allow an override | Same defect, plus a silent one — an override is invisible at the call site that omits it |
| A3 | `SeqCst` throughout | Correct and slower. It also destroys the measurement `ring_bench` exists to take: a benchmark of a padded ring under `SeqCst` measures the fence, not the padding |
| A4 | Inline `Ordering::Acquire` at each site | What the constant exists to prevent — and, per the measurement below, what the family does anyway at three sites out of four |

### The Constant's Claim

`GATING`'s own doc comment states the claim being measured:

> The ordering **every gating read in the family** uses. … Each writing
> `Ordering::Acquire` inline would be the same argument made independently in
> several places, which is how a family ends up with one crate relaxed and the
> rest not.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Ordering::Acquire' --include=*.rs ring_*/src/ \
  | grep -vE ':[[:space:]]*(///|//!|//)'
```

Live output:

```
ring_batch/src/lib.rs:    let at = producer.load(Ordering::Acquire);
ring_batch/src/lib.rs:    let behind = consumer.load(Ordering::Acquire);
ring_cursor/src/lib.rs:pub const GATING: Ordering = Ordering::Acquire;
ring_debug/src/lib.rs:const OBSERVE: Ordering = Ordering::Acquire;
ring_mpsc/src/lib.rs:pub const OBSERVE: Ordering = Ordering::Acquire;
ring_shutdown/src/lib.rs:        self.closed.load(Ordering::Acquire)
```

Six hits, in four independent statements of the same decision:

| Site | Form | Can it reach `GATING`? | What it is |
|------|------|:----------------------:|------------|
| `ring_cursor:89` | `pub const GATING` | — | the constant itself |
| `ring_batch:321`, `:322` | **inline, twice** | **no** — `ring_batch` does not declare `ring_cursor` | the two gating loads of `claim_gated` |
| `ring_debug:73` | `const OBSERVE` (private) | yes — declares `ring_cursor`, imports `CursorPair` | a diagnostic's read |
| `ring_mpsc:250` | `pub const OBSERVE` | yes — imports `GATING` at `:189` | the consumer's read of the *producer* cursor |
| `ring_shutdown:82` | inline | n/a | `self.closed.load(…)` on a `bool` — not a cursor read |

**The claim is four times over.** Not falsified in the way it feared — every one
of the four chose `Acquire`, so the family does not have "one crate relaxed and
the rest not" — but the argument it exists to make once has been made four
times, and nothing would notice if a fifth site chose differently.

**The count is four and not more** because no crate writes the fully-qualified
spelling of this ordering. Three of them write fully-qualified constants of
*other* orderings — `ring_publish:67` and `ring_consume:81` declare
`const … : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release`,
and `ring_claim:76` declares `::AcqRel` — so a grep written against the short
form alone would miss a
crate that later declared `core::sync::atomic::Ordering::Acquire`. Verified:

```sh
cd "$(git rev-parse --show-toplevel)"
# not one crate names this ordering in its fully-qualified form …
grep -r 'atomic::Ordering::Acquire' --include=*.rs ring_*/src/ \
  | grep -vE ':[[:space:]]*(///|//!|//)' \
  || echo '(no matches — Acquire is never written fully qualified)'
# … control: the identical expression for the orderings that are
grep -r 'atomic::Ordering::\(Release\|AcqRel\)' --include=*.rs ring_*/src/ \
  | grep -vE ':[[:space:]]*(///|//!|//)' | LC_ALL=C sort
```

Live output:

```
(no matches — Acquire is never written fully qualified)
ring_claim/src/lib.rs:const CLAIM_SUCCESS: core::sync::atomic::Ordering = core::sync::atomic::Ordering::AcqRel;
ring_consume/src/lib.rs:const COMMIT: core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
ring_publish/src/lib.rs:const PUBLISH: core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
```

Not one crate in the family names `Acquire` directly in library code, while
three name a different ordering that way.

Widening the lens from one ordering to all of them shows the same pattern at
family scale: **ten ordering constants across six crates, with three names —
`PUBLISH`, `OBSERVE`, `OWN` — each declared twice by two different crates.**
Every colliding pair agrees on its value, and nothing compares them. The full
census is in [`type/001`](../type/001_gating.md) § What It Pairs With.

### Why `ring_batch` Is the Sharp One

This crate's own module documentation cites `ring_batch::claim_gated` as
supporting evidence — *"the same choice `ring_batch::claim_gated` makes, for the
same reason"* (`src/lib.rs:46-47`). The cited crate makes that choice by writing
`Ordering::Acquire` inline, twice, under its own eight-line justification:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A10 -F '/// operation — the barrier read is a load, not a fence on the claim path.' ring_batch/src/lib.rs
```

Live output:

```
/// operation — the barrier read is a load, not a fence on the claim path.
///
/// `order` governs the advance only. The two gating reads are always
/// `Ordering::Acquire`, chosen here rather than left to the caller, because
/// they are not free choices: the whole point of reading the consumer's
/// position is to establish that its writes happened-before this claim, and a
/// `Relaxed` load would let a producer act on a stale barrier and overwrite a
/// slot the consumer had not finished with. This crate leaves the *advance*
/// ordering open because it genuinely varies with the protocol built on top;
/// the gating loads do not vary, so pretending they were a parameter would
/// offer a caller a choice with exactly one correct answer.
```

> the gating loads do not vary, so pretending they were a parameter would offer
> a caller a choice with exactly one correct answer.

**The same conclusion, reached independently, written down separately.** The
crate held up as proof that the decision is family-wide is the crate that
duplicates it.

And it cannot easily do otherwise. `claim_gated` is generic:

```rust
pub fn claim_gated< P : SeqCell, C : SeqCell >( producer : &P, consumer : &C, … )
```

It never handles a `PaddedCursor`. Reaching `GATING` means adding `ring_cursor`
to `ring_batch`'s manifest solely to import an `Ordering`, which would make a
crate that is deliberately generic over `SeqCell` depend on one concrete
implementor of it.

### The General Rule This Refines

`ring_align`'s `pattern/002` derived a reachability rule from a two-instance
sample: single ownership of a constant holds where the owning crate is already
reachable, and degrades to convention where reaching it costs a manifest edit.
`ring_cursor` is the counter-sample that sharpens it.

| Crate | Consumers | Duplicates of its constant |
|-------|----------:|---------------------------:|
| `ring_align` (`CACHE_LINE`) | 1 | 1 |
| `ring_cursor` (`GATING`) | **10** | **3** |

```sh
cd "$(git rev-parse --show-toplevel)"
# excludes Cargo.toml itself: that workspace manifest lists every
# crate as a member path (including ring_cursor), which is not the same
# claim as a manifest declaring ring_cursor as a dependency
command grep -rl 'ring_cursor' --include=Cargo.toml | command grep -v '^Cargo.toml$' | wc -l   # 11, incl. itself
```

Live output:

```
11
```

**Reachability is necessary and not sufficient.** Two of the three duplicates —
`ring_debug` and `ring_mpsc` — *could* have used `GATING`; both are consumers.
They wrote their own constant because they were naming a different question:
`ring_mpsc::OBSERVE` is the consumer reading the *producer* cursor, where
`GATING` is the producer reading the *consumer* cursor, and `ring_mpsc`
documents the distinction explicitly against its own `OWN` (`Relaxed`).

So what defeats single ownership here is not distance but **granularity**: a
constant named for one direction of one relationship cannot be reused for the
other direction without lying about what it names, even when it holds the same
value. Four constants, one value, four honest names.

### What Would Reopen It

| # | Condition | Effect |
|---|-----------|--------|
| E1 | A fifth site chooses `Relaxed` | The predicted failure, arriving. Nothing detects it today — see the gap below |
| E2 | `GATING` moves to `ring_atomic` | `ring_atomic` owns `SeqCell`, which every one of the four sites already depends on. It is the one placement reachable from all of them, including `ring_batch` |
| E3 | A target where `Acquire` and `SeqCst` cost the same | Removes A3's objection and makes the choice a non-decision |

**E2 is the alternative worth stating plainly.** The constant is placed with the
padded cursor, but the thing it actually governs is *any* `SeqCell` read used as
a barrier — which is `ring_atomic`'s concept, not this crate's. Placing it there
would make `ring_batch`'s duplicate unnecessary without adding a single manifest
edge, since `ring_batch` already declares `ring_atomic`.

### The Detection Gap

| # | Check | Status |
|---|-------|--------|
| F1 | `tests/manual/readme.md` M3 | Reads **this crate only** — expects exactly one non-doc `Ordering::Acquire`, and gets it. Correct, and blind to the other three crates |
| F2 | The compiler | Never. A second constant with the same value is not a warning |
| F3 | A test | Cannot, per-crate. A family-wide test would have to depend on all four crates |
| F4 | The family-wide grep above | Works, and is not wired to anything |

M3's scope is the finding in miniature: a per-crate check cannot see a
family-wide property, and every crate in this family checks itself.

**Recorded, not fixed.** E2 moves a public constant between crates and touches
four manifests plus every import of `GATING`; it belongs to a change with its
own tests and verification run, not to a documentation pass.

### CU13 — "Three Call Sites" Undercounts Both Ways

```
89:pub const GATING : Ordering = Ordering::Acquire;
functions that read it: slowest, free_slots, pending, may_claim
individual loads that name it: 7
```

The doc comment argues the constant is "named rather than written inline at three
call sites". Four functions read it, and because three of them read both cursors,
seven individual loads name it.

**Finding.** The figure was right when one reading existed and was not revisited
when two more were added. It is a number in a doc comment that no check recomputes
— the same failure shape as CU5's "six manifests", one item away.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'seven individual loads' ring_cursor/src/lib.rs
```

Live output:

```
/// Named rather than written inline at each of the seven individual loads
```

**Disposition:** applied — reworded `src/lib.rs:71-76` to name both counts
exactly: "each of the seven individual loads across four functions", replacing
the undercounting "three call sites". Verified with `cargo test --release -p
ring_cursor` (the `GATING` doctest still passes).
Now prints: `seven individual loads`

---

### CU14 — The Justification for `pub` Names Two Crates That Do Not Import It

| Named in the doc comment | Imports `GATING` |
|--------------------------|------------------|
| `ring_gating` | no |
| `ring_barrier` | no |

The five that do are `ring_claim`, `ring_consume`, `ring_mpsc`, `ring_publish`
and `ring_spsc`. None appears in the argument for the item they depend on.

**Finding.** The reasoning is still sound — a crate reading a cursor set to decide
safety does need the ordering — but it is illustrated with the two crates that
turned out to take `slowest` instead, and never mentions the five that took the
constant. The doc comment describes an intended user base, not the observed one.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'all import it for exactly that' ring_cursor/src/lib.rs
```

Live output:

```
/// `ring_mpsc`, `ring_publish` and `ring_spsc` all import it for exactly that
```

**Disposition:** applied — reworded `src/lib.rs:78-83` to name the crates that
actually import `GATING` — `ring_claim`, `ring_consume`, `ring_mpsc`,
`ring_publish` and `ring_spsc` — replacing the two, `ring_gating` and
`ring_barrier`, that take `slowest` instead and never import the constant.
Verified with `cargo test --release -p ring_cursor` (the `GATING` doctest
still passes).
Now prints: `all import it for exactly that`

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_surface_that_forwards.md](../api/001_the_surface_that_forwards.md) | The half of the surface that took the opposite decision, in the same crate |
| [../api/002_the_surface_that_decides.md](../api/002_the_surface_that_decides.md) | The half this decision governs |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_slowest_fold.md](../algorithm/001_the_slowest_fold.md) | The fold whose loads this fixes |
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | The six loads this fixes |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | The ten consumers the reachability count comes from |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_fold_two_questions.md](../pattern/002_one_fold_two_questions.md) | The other thing this crate names once for the whole family, which did hold |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_gating.md](../type/001_gating.md) | The constant as a contract |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:38-51` | The module-level argument, including the `ring_batch` citation |
| `ring_cursor/src/lib.rs:71-89` | The constant and its claim |
| `ring_batch/src/lib.rs:306-322` | The duplicate that the claim cites as support |
| `ring_debug/src/lib.rs:65-73` | `OBSERVE`, argued on diagnostic grounds |
| `ring_mpsc/src/lib.rs:239-287` | `OBSERVE`, `COMMIT`, `OWN` — the direction-specific naming |

### Tests

| File | Relationship |
|------|--------------|
| `src/lib.rs:85-88` | `GATING`'s doctest — asserts the value, not the uniqueness |
| `ring_mpsc/tests/mpsc_test.rs:782` | A consumer asserting `GATING == Acquire` from outside |
| `ring_spsc/tests/spsc_test.rs:869` | The same assertion, in a second consumer |
| `tests/manual/readme.md` M3 | The single-statement check, scoped to this crate |
