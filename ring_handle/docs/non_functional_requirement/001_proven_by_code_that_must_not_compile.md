# Non-Functional Requirement: Proven by Code That Must Not Compile

### Scope

- **Purpose**: State this crate's binary Reached condition as its own acceptance criterion, and account for what makes it structurally unlike every other criterion in the family.
- **Responsibility**: The attribute, the statement, the measurement, and the threshold — with the coverage gap named rather than smoothed over.
- **In Scope**: The two compile-fail cases; what they assert and what they leave uncovered.
- **Out of Scope**: The `Send` half of this crate's row (→ [Send Without Sync](002_send_without_sync.md)); `ring_poll`'s own bounded-time test.

### Quality Attribute

**Enforceability** — whether a stated restriction survives contact with future
editing. A secondary attribute is **diagnosability**: a violation should be
reported at the offending call site, not as a downstream symptom.

### Statement

Verbatim from
[the acceptance table](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md),
this crate's row:

> `Producer` exposes no drain method and `Consumer` no publish method —
> asserted by a `trybuild` compile-fail case for each; both are `Send`, and the
> pair can be moved to two threads without a shared mutable reference.

**Reached is binary**, per the table's own rule: where a criterion names a
number, the number is the test's assertion, not a target to approach. Here there
are no numbers at all — two compile-fail cases either exist and fail to compile,
or the feature is not Reached.

**This is the only row in the 22-row table whose central assertion is
negative.** Every other feature is Reached by demonstrating that something
*happens*: 100 000 items arrive, a claim of 64 slots issues one fence, a
registered ring is retrievable. This one is Reached by demonstrating that
something *cannot be written*.

| | Ordinary criterion | This criterion |
|---|---|---|
| Asserts | A behaviour occurs | A program is rejected |
| Fails when | The behaviour is absent or wrong | The program compiles |
| Adding a method to the crate | Cannot break it | **Breaks it** |
| Deleting the test | Suite goes green | Suite goes green |

**Row three is the property that makes this criterion worth its unusual
machinery**, and row four is the reason it must be listed in the acceptance
table rather than left as a local convention. A positive test suite gets
greener as the surface grows; this is the family's only test that gets redder.

### Measurement Method

1. **A `trybuild` compile-fail case per restriction**, each a standalone `.rs`
   file under `tests/ui/` that calls the forbidden method and is expected to be
   rejected. The acceptance table requires two — a drain call on `Producer`, a
   publish call on `Consumer`. **Four are written**, the extra pair asserting
   that neither handle can be cloned; see the threshold's P6 for why the
   criterion was widened rather than the gap merely recorded.

2. **The expected error is pinned, not merely "some error."** `trybuild`
   compares against a committed `.stderr` file. A case that fails for the wrong
   reason — a typo'd method name, a missing import, a moved value — is a case
   that passes while proving nothing, and this is the standard way compile-fail
   suites rot.

3. **The cases are compiled against this crate's real public surface**, not
   against a local stub. A case written against a test-only type would keep
   passing after the real `Producer` gained a drain method.

4. **The `.stderr` files are reviewed when they change.** `trybuild`'s
   `TRYBUILD=overwrite` regenerates them, which makes accepting a genuine
   regression a one-command operation. This is the criterion's own soft spot and
   the mitigation is procedural rather than technical: a changed `.stderr` is a
   change to what the crate guarantees.

5. **A compiler-version note.** Compile-fail output is not stable across
   toolchain releases, so a `.stderr` mismatch may mean the toolchain moved
   rather than the crate. The distinguishing check is whether the *program is
   still rejected* — which is the actual requirement — rather than whether the
   message matches byte for byte.

6. **The suite is shown to go red at least once.** A compile-fail suite whose
   cases have never failed proves only that four programs do not compile, which
   is also true of four programs containing a typo. The check is to add the
   forbidden method, run the suite, confirm the matching case fails, and remove
   it — recorded at `tests/manual/readme.md` H2 with the measured output.

   **This is the step that distinguishes measurement from decoration**, and it
   is the one a compile-fail suite most often skips: steps 1–5 all describe
   what the cases *are*, and none of them establishes that the mechanism
   responds to the thing it exists to detect.

### Acceptance Threshold

| # | Criterion | Met when |
|---|-----------|----------|
| P1 | A drain call on `Producer` does not compile | `tests/ui/producer_drains.rs` is rejected, with pinned stderr |
| P2 | A publish call on `Consumer` does not compile | `tests/ui/consumer_publishes.rs` is rejected, with pinned stderr |
| P3 | Both cases fail for the intended reason | The pinned stderr names the missing method, not an unrelated error |
| P4 | The cases compile against the crate's real public surface | No test-only shim types are involved |
| P5 | The crate's tests cite this crate's declared feature identifier textually (per `bench_harness/gate/declared/ring/features.txt`) | Gate `g3_features.sh` records the crate→feature edge |
| P6 | Neither handle can be cloned | `tests/ui/producer_clones.rs` and `tests/ui/consumer_clones.rs` are rejected — **beyond the acceptance table's two cases** |
| P7 | The suite responds to a real violation | The forbidden method added → the matching case fails; removed → the suite is green again (`tests/manual/readme.md` H2) |

**What this threshold does not cover, stated plainly:** it says nothing about a
handle exposing its backend. That is V4 of
[Capability Follows the Handle](../invariant/001_capability_follows_the_handle.md),
it compiles, and it **passes P1 through P7 unchanged.**

**The `!Clone` half of that gap is closed, and how it closed is the point.** As
specified, the criterion left both V3 (a derived `Clone`) and V4 open, and this
instance's original text proposed a third and fourth case "possible and not
currently specified", deferring to the family-grain question of amending a
shared acceptance table. The cases were written instead. Amending
`bench_harness`'s table is indeed a family-grain decision; **writing a stricter
local test is not, because a criterion the crate exceeds is not a criterion the
crate has changed.** P1–P5 remain exactly what the table asks for and are
independently checkable; P6 is additional and is labelled as such.

**V4 stays open for a reason that is not deferral.** A compile-fail case names
one thing that must not exist. V4 is the absence of *any* public route to the
backend — `inner()`, a `Deref`, a public field, a `From` impl — and there is no
way to write "no method returns `&Backend`" as a program that must not compile.
Closing it needs a different mechanism (a surface snapshot, or a lint), which is
a larger piece of machinery than this crate should build alone.

**The threshold also does not assert the `Send` half** of this crate's row.
That is a distinct property with a distinct measurement and it is
[Send Without Sync](002_send_without_sync.md)'s. This crate's row is Reached only
when both this criterion and that one are met — the row bundles them, and
splitting them across two instances is a documentation choice, not a weakening.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The surface P1 asserts a hole in |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The surface P2 asserts a hole in |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | The invariant this criterion measures — and its V3/V4, which the criterion does not reach |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_send_without_sync.md](002_send_without_sync.md) | The other half of this crate's row, measured separately |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_enforce_by_withholding.md](../pattern/001_enforce_by_withholding.md) | Why a practice that enforces by absence needs a negative test to stay enforced |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | The edits P1–P5 catch, and the two they do not |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's row — the verbatim statement and the binary-Reached rule |

### Tests

| File | Relationship |
|------|--------------|
| `tests/ui/producer_drains.rs` | P1 — expected to be rejected, with pinned `.stderr` |
| `tests/ui/consumer_publishes.rs` | P2 — expected to be rejected, with pinned `.stderr` |
| `tests/ui/producer_clones.rs`, `tests/ui/consumer_clones.rs` | P6 — the two cases beyond the acceptance table's criterion |
| `tests/ui_test.rs` | Drives the `trybuild` runner. Kept separate from `handle_test.rs` so a toolchain upgrade that shifts diagnostic wording fails one binary rather than the crate's whole test surface |
| `tests/handle_test.rs` | Carries P5's feature citation, and the runtime half of this crate's criterion |

### HD33 — The Threshold Grades Four Cases and the Runner Drives Seven

Measurement 1 says "**Four are written**" and P1–P7 name four files. The
trybuild driver names more:

```sh
cd "$(git rev-parse --show-toplevel)"
f=ring_handle/docs/non_functional_requirement/001_proven_by_code_that_must_not_compile.md
echo '  -- cases the P1..P7 threshold rows actually name --'
command grep -E '^\| P[0-9] \|' "$f" | command grep -oE '[a-z_]+\.rs' | sort -u | sed 's|^|    |'
echo '  -- cases the trybuild driver runs --'
command grep -oE 'tests/ui/[a-z_]+\.rs' ring_handle/tests/ui_test.rs \
  | sed 's|tests/ui/|    |' | sort
```

Live output:

```
  -- cases the P1..P7 threshold rows actually name --
    consumer_clones.rs
    consumer_publishes.rs
    producer_clones.rs
    producer_drains.rs
  -- cases the trybuild driver runs --
    consumer_clones.rs
    consumer_publishes.rs
    producer_clones.rs
    producer_drains.rs
    producer_shared_across_threads.rs
    producer_try_clones.rs
    ring_used_after_split.rs
```

Four graded, seven driven. The three ungraded ones are not spares — each is
cited elsewhere in this corpus as the sole detector for a property:
`producer_shared_across_threads` is [`002`](002_send_without_sync.md)'s evidence
for `!Sync`, `ring_used_after_split` is
[`algorithm/001`](../algorithm/001_splitting_a_ring_into_two_ends.md)'s only
proof that the split is a move, and `producer_try_clones` is
[`pattern/001`](../pattern/001_enforce_by_withholding.md)'s part-2 detector.

**So three compile-fail cases carry real weight and no acceptance criterion
grades them.** That is the opposite failure from the one this instance is
careful about: it worries that a criterion might be *softened* to let something
pass, and the actual gap is a criterion that has not been *widened* to cover
what the suite already does.

The instance's own reasoning covers the fix — "writing a stricter local test is
not [a family-grain decision], because a criterion the crate exceeds is not a
criterion the crate has changed." That argument produced P6 for the two clone
cases. It applies unchanged to the other three and was not run a second time.

### HD34 — P7's Evidence Is From a Five-Case Suite and Two Cases Have Never Been Shown to Fail

Measurement 6 argues that a compile-fail suite is decoration until an injected
violation drives it red, and P7 cites `tests/manual/readme.md` H2 for that.
Compare what H2 recorded against what runs now:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what H2 was written against, and what it injected --'
sed -n '/^## H2 —/,/^## H3 —/p' ring_handle/tests/manual/readme.md \
  | command grep -E 'Five programs|other four|^\*\*Result|pub fn try_recv' \
  | cut -c1-96 | sed 's|^|    |'
echo '  -- and how many cases the driver runs today --'
printf '  cases driven: %s\n' \
  "$( command grep -c 'compile_fail' ring_handle/tests/ui_test.rs )"
```

Live output:

```
  -- what H2 was written against, and what it injected --
    Five programs that do not compile is also an accurate description of five
      pub fn try_recv( &mut self ) -> Option< T >
    it was expected not to. The other four still pass, since none of them mentions
    **Result (2026-08-28): holds.**
  -- and how many cases the driver runs today --
  cases driven: 7
```

H2 counts "five programs" and predicts "the other four still pass." Seven run
today. The injected violation was a single forbidden `try_recv` on `Producer`,
which drives exactly one case — `producer_drains.rs`.

**One case of seven has been shown to respond, and measurement 6's argument
applies verbatim to the other six.** `consumer_publishes`, `producer_clones` and
`consumer_clones` were in the suite when H2 ran and were not injected against;
`producer_try_clones` and `producer_shared_across_threads` were added after H2's
recorded date and have never been driven red at all.

This does not make P7 false — it makes P7 narrower than it reads. The honest
statement is "the mechanism responds, demonstrated once, on the drain case."
Whether the other six would respond is a different claim, and the reason it
matters is exactly the one measurement 6 gives: a case that fails for the wrong
reason and a case that cannot fail look identical from a green suite.
