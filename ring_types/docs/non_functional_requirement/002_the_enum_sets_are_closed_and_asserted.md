# Non-Functional Requirement: The Enum Sets Are Closed and Asserted

### Scope

- **Purpose**: State the requirement that every enum this crate exports is a closed set whose closure is mechanically detectable, and measure how each of the three enums actually meets it — because they meet it by three different mechanisms of three different strengths.
- **Responsibility**: State the quality attribute, the requirement, its measurement method, and its acceptance threshold.
- **In Scope**: `WaitKind`, `OverflowPolicy`, `RingError`; the two `ALL` arrays; the wildcard-free `match` that does the real work; the gate that covers the enum no compiler check can reach.
- **Out of Scope**: The allocation requirement (→ [`non_functional_requirement/001`](001_errors_and_positions_do_not_allocate.md)); what the variants mean (→ [`type/`](../type/)).

### Quality Attribute

**Completeness of a sweep axis under future variant addition.**

Half this family's tests, and both of `ring_stats`'s and `ring_overflow`'s
central loops, are of the form *"for every policy / every wait kind, assert
X"*. An axis that silently loses a row does not fail — it passes, faster, with
one fewer case checked. That is the worst available failure mode: a green suite
that covers less than it did yesterday, with no diff in the test file to show
for it.

The attribute is therefore not correctness of the enums as declared. It is
**detectability of a future edit that leaves them under-swept.** The requirement
is about the next variant, not the current ones.

### Statement

**Every exported enum is a closed set, and adding a variant to it must fail
something loudly.**

```text
S1.  Each enum's full variant set is reachable as data, not only as syntax
S2.  Adding a variant without updating that data fails a build, a test,
     or a gate — never nothing
S3.  Corrupting the data without adding a variant (a dropped entry, a
     duplicated one) also fails
S4.  The failure names the enum
```

**S2 and S3 are separate requirements catching opposite mistakes**, and the
crate satisfies them with separate mechanisms. S2 is about the enum growing past
its roster; S3 is about the roster decaying while the enum stays still. A check
that catches one routinely misses the other.

The three enums are covered as follows:

| Enum | Variants | `ALL` array | Wildcard-free `match` | Roster `contains` |
|------|---------:|:-----------:|:---------------------:|:-----------------:|
| `WaitKind` | 4 | ✅ `[ Self; 4 ]` | ✅ in `tests/`, ✅ in `ring_wait`'s `src/` | ✅ |
| `OverflowPolicy` | 3 | ✅ `[ Self; 3 ]` | ✅ in `tests/`, ✅ in `ring_overflow`'s `src/` | ✅ |
| `RingError` | 9 | ❌ none | ✅ in `src/`, ❌ **impossible** in `tests/` | ✅ (hand-listed) |

**`RingError`'s split row is the whole subtlety of this requirement.** Declaring
a variant and *exercising* it are enforced by different mechanisms at different
latencies, and only the first is a compile error.

### Measurement Method

**Three mechanisms of three strengths. The one that looks like the enforcement
is not it.**

**1. The array length, which enforces nothing about the enum.** It is tempting
to read `pub const ALL : [ Self; 4 ]` as making a fifth variant a compile error.
It does not:

```sh
cd "$( mktemp -d )"
cat > all_probe.rs <<'EOF'
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub enum WaitKind { Spin, Yield, Park, None, Backoff }

impl WaitKind
{
  pub const ALL : [ Self; 4 ] = [ Self::Spin, Self::Yield, Self::Park, Self::None ];
}

fn main()
{
  println!( "compiled; ALL.len() = {} while the enum has 5 variants", WaitKind::ALL.len() );
}
EOF
rustc --edition 2021 all_probe.rs -o all_probe && ./all_probe
```

Live output:

```
compiled; ALL.len() = 4 while the enum has 5 variants
```

**A five-variant enum with a four-element `ALL` compiles cleanly.** The explicit
length constrains the array against its own initialiser, never against the enum.
It satisfies S3 and contributes nothing to S2.

**2. The wildcard-free `match`, which is the actual S2 enforcement.** In
`wait_kind_has_exactly_four_variants`:

```rust
// Exhaustive match: a new variant breaks compilation here rather than
// silently passing the length check via a replaced entry.
for kind in WaitKind::ALL
{
  match kind
  {
    WaitKind::Spin | WaitKind::Yield | WaitKind::Park | WaitKind::None => {}
  }
}
```

The loop body is empty and the `match` asserts nothing at runtime. Its entire
purpose is to be a place a fifth variant cannot pass through. **This is the
mechanism, and none of it is in this crate's `src/`** — which is why reading
`policy.rs` alone leaves the impression that the length does the work.

It is not the only such `match`, and the others are production code rather than
tests: `ring_wait`'s `escalation_hint` and `pause` both match `WaitKind`
exhaustively, and `ring_overflow` matches `OverflowPolicy` exhaustively at two
sites. Measured by neutering the test-file `match` and adding a fifth
`WaitKind`: this crate's whole suite went green — 19/19 unit and 20/20
doctests, the length-assertion doctest included — while `ring_wait` threw two
`error[E0004]`s. So S2 is over-determined for both policy enums, and no single
site is load-bearing; the test-file `match` is simply the nearest one, and the
only one inside the declaring crate.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'match ' ring_types/tests/types_test.rs
```

Live output:

```
    match kind
    match policy
```

**Two matches in a 354-line suite, and they are both this construct.** Nothing
else in the file matches on anything.

**3. Gate G1's line coverage, which covers what mechanism 2 cannot reach for
`RingError`.** Two things must happen when a tenth variant is added, and they
are enforced separately:

| What | Mechanism | Latency |
|------|-----------|---------|
| A `Display` arm exists for it | `Display::fmt`'s wildcard-free `match` at `src/error.rs:166–181` | **Compile error** |
| A test actually exercises that arm | Gate G1's 100% line-coverage threshold | One gate run |

`Display::fmt` matches all nine variants with no wildcard, inside the defining
crate — where `#[ non_exhaustive ]` imposes nothing — so **the arm cannot be
forgotten.** What can be forgotten is the hand-maintained roster in
`every_error_displays_distinctly`, and there mechanism 2 is genuinely
unavailable: `types_test.rs` is a separate crate, so a `match` written there is
*required* to carry a wildcard arm and can never be made to fail on a new
variant. `bench_harness/gate/g1_coverage.sh` stands in — an unrostered
variant leaves its `Display` arm unexecuted, and the gate reports the file and
the fraction.

The suite records this working, on a real incident rather than a hypothetical:

> `PolicyUnsupported` was added to `ring_types` while implementing `ring_core`,
> this list was not updated, and G1 reported `ring_types/src/error.rs 16/17` on
> the next run. Detection took one gate run rather than a compiler error —
> slower, but not silent, which is the property that matters.

### Acceptance Threshold

| # | Threshold | Enum | Mechanism | State |
|---|-----------|------|-----------|-------|
| T1 | Adding a variant fails the build | `WaitKind` | Wildcard-free `match` | ✅ **Met** |
| T2 | Adding a variant fails the build | `OverflowPolicy` | Wildcard-free `match` | ✅ Met |
| T3a | Adding a variant fails the build | `RingError` | `Display::fmt`'s wildcard-free `match` in `src/` | ✅ Met |
| T3b | The new variant is *exercised*, not merely declared | `RingError` | G1 coverage — one gate run, not a compile error | ⚠️ Met, at a slower latency |
| T4 | A dropped `ALL` entry fails | `WaitKind` | `len() == 4` | ✅ Met |
| T5 | A **duplicated** `ALL` entry fails | `WaitKind` | Per-variant `contains` loop | ✅ Met |
| T6 | A duplicated `ALL` entry fails | `OverflowPolicy` | Per-variant `contains` loop | ✅ Met (was ❌; closed) |
| T7 | The failure names the enum | all three | Assertion messages; G1 names the file | ✅ Met |

**T6 was this document's one real gap, and it is now closed.** It is kept here
rather than deleted because how it was analysed is more instructive than the
three lines that fixed it: the analysis below was wrong in both directions, and
only measurement showed which way.

Before the fix, `OverflowPolicy`'s test carried the length check and the
exhaustive `match`, but not the per-variant `contains` loop its sibling has:

```rust
fn overflow_policy_has_no_overwrite_variant()
{
  assert_eq!( OverflowPolicy::ALL.len(), 3 );
  for policy in OverflowPolicy::ALL
  {
    match policy
    {
      OverflowPolicy::DropNewest | OverflowPolicy::DropOldest | OverflowPolicy::Fail => {}
    }
  }
}
```

Both checks pass for any three-element array: the length is 3, and the `match` is
exhaustive over the enum regardless of which values the array actually holds.
**`WaitKind`'s test would catch the same edit** — its `contains` loop asserts
each of the four named variants is present.

**Which corruptions actually survived was narrower than the original analysis
concluded, and the original analysis got one of the four rows wrong.** A second
test in the same file — `overflow_policies_partition_by_reporting` — adds
`count( reports_failure ) == 1` and `count( drops_silently ) == 2`. Simulating
the four candidate arrays against those four assertions gave:

```text
[DropNewest,DropOldest,Fail]  len==3:true  assert_ne:true  rf==1:true  ds==2:true  → correct
[Fail,Fail,Fail]              len==3:true  assert_ne:true  rf==1:FALSE ds==2:FALSE → CAUGHT
[DropNewest,DropNewest,Fail]  len==3:true  assert_ne:true  rf==1:true  ds==2:true  → SURVIVES
[DropOldest,DropOldest,Fail]  len==3:true  assert_ne:true  rf==1:true  ds==2:true  → SURVIVES
```

**Row 3 is wrong, and the reason is that the simulation's assertion set is
incomplete.** It enumerates four unit-test assertions and no doctests, and
`OverflowPolicy::ALL`'s own doctest asserts `contains( &OverflowPolicy::DropOldest )`.
Actually running both arms rather than simulating them:

| `ALL` rewritten as | `ring_types` unit | `ring_types` doctest |
|---|---|---|
| `[DropNewest, DropNewest, Fail]` | 19/19 pass | **1 FAILED** |
| `[DropOldest, DropOldest, Fail]` | 19/19 pass | 20/20 pass |

So there was exactly **one** surviving corruption, not two: the arm that drops
`DropNewest`, the `#[ default ]` policy, which no doctest happens to name. A
simulation is only as complete as its assertion inventory, and this one silently
omitted a whole category of assertion that the real suite runs.

**What the count assertions do and don't buy is the transferable part.** They
reject `[ Fail, Fail, Fail ]` because that changes the partition sizes. They do
not reject either duplicate-a-drop-variant arm, because a duplicate that swaps
one drop variant for the other preserves both counts exactly. Counts constrain
the roster's *shape*; only membership names its *contents*. That is why the fix
is a `contains` loop and not a sharper count.

**The fix, applied:**

```rust
for expected in [ OverflowPolicy::DropNewest, OverflowPolicy::DropOldest, OverflowPolicy::Fail ]
{
  assert!( OverflowPolicy::ALL.contains( &expected ), "{expected:?} missing from ALL" );
}
```

Re-run against both arms afterwards, the surviving arm fails by name:

| `ALL` rewritten as | before the fix | after the fix |
|---|---|---|
| `[DropNewest, DropNewest, Fail]` | unit 19/19, doctest 1 failed | unit **FAIL** — `DropOldest missing from ALL` |
| `[DropOldest, DropOldest, Fail]` | unit 19/19, doctest 20/20 — **survived** | unit **FAIL** — `DropNewest missing from ALL` |
| unmodified | 19/19 + 20/20 | 19/19 + 20/20 |

The consequence is not confined to this crate. `ring_stats::dropped_total`
derives its total by summing over the array (`ring_stats/src/lib.rs:378`):

```rust
OverflowPolicy::ALL.iter().map( | p | self.dropped( *p ) ).sum()
```

A duplicated `ALL` makes that sum double-count one policy and omit another. The
original text concluded from this that `ring_stats`'s own suite would pass
throughout — it iterates the same array (13 references across its tests: 11 in
`stats_test.rs`, 2 in `tests/manual/readme.md`) and so "shares the corruption
rather than detecting it" — and generalised to **"a test that sweeps `ALL`
cannot validate `ALL`."**

**That generalisation is false, and both arms refute it.** Running the two
duplicate arms against the downstream crates, nine tests fail on each:

| Crate | Tests failing on both arms |
|-------|----------------------------|
| `ring_stats` (4) | `a_drop_is_counted_not_absorbed`, `a_drop_lands_under_its_own_policy_only`, `distinct_policy_counters_do_not_interfere_under_contention`, `the_drop_total_is_the_sum_over_every_policy` |
| `ring_overflow` (5) | `distinct_policies_give_distinct_resolutions`, `no_resolution_overwrites_unread_data_silently`, `counts_accumulate_and_stay_separated`, `exactly_one_counter_moves_per_call`, `a_resolution_is_a_plain_comparable_value` |

**The correct rule is a distinction between two kinds of sweep, and the failing
test names carry it.** A sweep whose assertions hold *of each element on its own*
inherits the corruption: with the roster holding one variant twice, each
iteration checks that variant against itself, agrees, and passes. A sweep whose
assertions require elements to *differ from one another* — `a_drop_lands_under
its own policy only`, `distinct_policies_give_distinct_resolutions`,
`counters_do_not_interfere` — cannot pass on a duplicate, because distinctness is
exactly what a duplicate destroys.

Aggregate counts sit between the two and are weaker than they look: they catch a
corruption that changes the partition sizes and miss one that preserves them,
which is precisely how both arms slipped past
`overflow_policies_partition_by_reporting`. Membership is the only assertion here
that is decisive in every case, which is the general reason `contains` is the
right shape for this class of check and a count is not.

The same rule read the other way explains
[`bench_harness`](../../../bench_harness/tests/oracle_test.rs)'s twin gap, found
and closed alongside this one. Its `Accumulator::ALL` is swept by four loops, and
until a `contains` loop was added **both** decay arms scored 14/14 unit and 14/14
doctests — worse than here, since `Accumulator::ALL` has no reference outside its
own crate, so no downstream sweep existed to catch it either. Its central sweep
asserts `is_order_independent()` against what folding actually does: a fact each
accumulator satisfies about itself, so a duplicated roster is self-consistent and
passes. Distinctness was the missing ingredient, not effort.

**T3b's weaker strength is a fact about `#[ non_exhaustive ]`, not a defect to
fix here.** The attribute is correct for `RingError` — the family is still
adding backends and a tenth variant must not be a breaking change for external
consumers (→ [`api/001`](../api/001_the_vocabulary_surface.md) G3). Its cost is
exactly this: inside the crate, closure is a compile error (T3a); outside it, no
test can be made to fail on a new variant, and a coverage gate stands in.
Recording the trade is the useful thing; reversing it is not.

**The historical incident fits the split exactly.** `PolicyUnsupported`'s
`Display` arm *was* written when the variant was added — T3a's compile error
left no choice. What was skipped was the test roster in the other crate, and G1
reported `ring_types/src/error.rs 16/17` on the next run. The arm the compiler
forced into existence is the same line the gate found unexecuted.

**The fix for T6 was three lines** — the `contains` loop from `WaitKind`'s test,
transposed into `overflow_policy_has_no_overwrite_variant`, where the
measurement above is recorded beside it. The finding that mattered was not the
gap, which this document already named; it was that the document's own account
of the gap's scope was wrong in both directions — one corruption reported as
surviving that a doctest caught, and one bolded rule refuted by nine downstream
tests. **A documented gap is a hypothesis, not a measurement.** Both halves had
to be re-run before either the fix or the rule could be trusted.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | G3 and G6 — the guarantees this requirement measures |
| [../api/002_the_five_classifier_predicates.md](../api/002_the_five_classifier_predicates.md) | Q4, the same silent-default hazard on the predicates rather than the arrays |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_classifying_an_error_into_configuration_or_traffic.md](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) | The three-variant gap a new `RingError` variant silently joins |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | `#[ non_exhaustive ]` as a layout constraint, and what it forecloses |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_constant/002_wait_kind_all.md](../item/associated_constant/002_wait_kind_all.md) | The array T1, T4 and T5 hold for |
| [../item/associated_constant/003_overflow_policy_all.md](../item/associated_constant/003_overflow_policy_all.md) | The array T6 holds for, since the `contains` loop closed it |
| [../item/enum/001_wait_kind.md](../item/enum/001_wait_kind.md) | Four variants, fully covered |
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | Nine variants, covered only by a gate |
| [../item/enum/003_overflow_policy.md](../item/enum/003_overflow_policy.md) | Three variants, covered against growth and, since T6, against decay |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_errors_and_positions_do_not_allocate.md](001_errors_and_positions_do_not_allocate.md) | The crate's other requirement — enforced by a derive, and therefore not subject to any of this |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md](../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md) | What `OverflowPolicy::ALL`'s three entries actually reach at runtime |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_ring_error.md](../type/002_ring_error.md) | The enum with no `ALL` |

### Sources

| File | Relationship |
|------|--------------|
| [`src/policy.rs`](../../src/policy.rs) | Lines 58 and 131 — the two `ALL` declarations, the doctests that pin their lengths, and the doc comments that now say what those lengths do *not* pin |
| [`src/error.rs`](../../src/error.rs) | Line 43's `#[ non_exhaustive ]`, the attribute that makes T3 the best available |
| [`ring_stats/src/lib.rs`](../../../ring_stats/src/lib.rs) | Line 378 — the family's one production sweep over an `ALL` array, and what T6 would corrupt |
| [`bench_harness/gate/g1_coverage.sh`](../../../bench_harness/gate/g1_coverage.sh) | T3's mechanism; the 100% threshold and the non-vacuity check beside it |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ T1, T2, T4, T5 asserted by `wait_kind_has_exactly_four_variants` and `overflow_policy_has_no_overwrite_variant`; the roster in `every_error_displays_distinctly` is what G1 measures for T3. ✅ **T6 too, since the `contains` loop landed in `overflow_policy_has_no_overwrite_variant`** — the omission had been invisible because the test that would carry it looked complete beside its sibling: one had three checks, the other two, and nothing in either file said the third was intended |
| [`ring_stats/tests/stats_test.rs`](../../../ring_stats/tests/stats_test.rs) | ✅ Eleven references to `OverflowPolicy::ALL`, four of which do detect T6 — `a_drop_is_counted_not_absorbed`, `distinct_policy_counters_do_not_interfere_under_contention`, `a_drop_lands_under_its_own_policy_only`, `the_drop_total_is_the_sum_over_every_policy`, each on both decay arms. Sweeping the array inherits its corruption only where the assertions hold of each element on its own; these require the elements to differ, which is what a duplicate destroys |

### TY45 — The Closed-Set Tests Catch an Added Variant, and Caught a Duplicated Entry Only by Accident

The exhaustive match is doing the real work, and it is doing it for the enum,
not for the array:

```rust
// tests/types_test.rs — the guard that works, for growth
match policy
{
  OverflowPolicy::DropNewest | OverflowPolicy::DropOldest | OverflowPolicy::Fail => {}
}
```

Add a variant and this stops compiling — that half was never in doubt. The
original reading of the other half was:

> Change `ALL` to `[ DropNewest, DropNewest, Fail ]` and the length assertion
> still reads 3, the match still covers every variant it sees, and the
> `reports_failure` count is still 1. Nothing fails.

**That was wrong, and running it is what showed so.** Something did fail: this
crate's own doctest on `OverflowPolicy::ALL` asserts
`contains( &DropOldest )`, so the arm quoted above was already caught — by one
line, in a doctest, which the analysis had not enumerated because it simulated
against the unit assertions only. The arm that genuinely survived was the
*other* one, `[ DropOldest, DropOldest, Fail ]`, which dropped the
`#[ default ]` policy from the roster and still scored 19/19 unit tests and
20/20 doctests. Naming the wrong arm is worse than naming none: it points the
fix at a corruption already covered and leaves the uncovered one unnamed.

Both arms are now refused by name, by the `contains` loop in
`overflow_policy_has_no_overwrite_variant`. The three lines transposed from
`WaitKind`'s sibling test were the fix, which is why
[`../decisions/readme.md`](../decisions/readme.md) files this under *Not
decisions* rather than as a trade-off — but the fix was the cheap half. The
expensive half was learning that a gap recorded in a design document is a
hypothesis about a suite, and that the suite answers differently than the
document predicts in both directions at once: one corruption reported as
surviving that was already caught, and one bolded rule about sweeps
(§ *A test that sweeps `ALL` cannot validate `ALL`*, above) refuted by nine
tests in two sibling crates that do exactly that.

The test now reads:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^fn overflow_policy_has_no_overwrite_variant/,/^}$/p' ring_types/tests/types_test.rs
```

Live output:

```
fn overflow_policy_has_no_overwrite_variant() {
    assert_eq!(OverflowPolicy::ALL.len(), 3);
    for expected in [OverflowPolicy::DropNewest, OverflowPolicy::DropOldest, OverflowPolicy::Fail] {
        assert!(OverflowPolicy::ALL.contains(&expected), "{expected:?} missing from ALL");
    }
    for policy in OverflowPolicy::ALL {
        match policy {
            // Each arm is a policy that either drops a *nameable* item or refuses.
            // No arm overwrites an unread one; a variant that did would have to be
            // added here, which is where a reviewer would see it.
            OverflowPolicy::DropNewest | OverflowPolicy::DropOldest | OverflowPolicy::Fail => {},
        }
    }
}
```

**Disposition:** applied — `overflow_policy_has_no_overwrite_variant` now runs
a `contains` loop across `OverflowPolicy::ALL`, refusing both
`[ DropNewest, DropNewest, Fail ]` and `[ DropOldest, DropOldest, Fail ]` by
name, so a duplicated `ALL` entry can no longer pass the suite undetected. Now prints: `assert_eq!(OverflowPolicy::ALL.len(), 3);`
