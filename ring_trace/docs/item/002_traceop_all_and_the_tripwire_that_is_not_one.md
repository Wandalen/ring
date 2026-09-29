# Item: `TraceOp::ALL` and the Tripwire That Is Not One

### Scope

**Purpose:** Separate the crate's two claimed compile-time guards — one real, one
not — and record that the test named for keeping `TraceOp::ALL` in step with the
enum contains no assertion capable of failing.

**Responsibility:** The `ALL` constant and its declared purpose, `name()`'s
exhaustive-match claim, the suite's exhaustive-match claim, what each actually
forces, and a compiled demonstration of the case the second claim names.

**In Scope:** `ring_trace/src/lib.rs:72-81`, `:86-94`, `:96-117`;
`ring_trace/tests/trace_test.rs:200-227`.

**Out of Scope:** The lint reaching these declarations is
[`item/001`](001_six_impl_blocks_and_the_three_a_lint_cannot_reach.md). What the
five discriminants name is
[`type/001`](../type/001_five_discriminants_and_the_array_beside_them.md).

---

## Two Claims About the Same Enum

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the constant, and what its doc says it is for --'
command grep -m1 -A8 -F '  /// Every discriminant, for a test that must cover all of them.' ring_trace/src/lib.rs
echo '  -- the tripwire the source claims, which is real --'
command grep -m1 -A3 -F '  /// Written as an exhaustive `match` rather than a derive so that adding a' ring_trace/src/lib.rs
echo '  -- the tripwire the suite claims --'
command grep -m1 -A16 -F 'fn the_operation_kinds_are_exactly_the_five_declared()' ring_trace/tests/trace_test.rs
echo '  -- and what covered() can return --'
command grep -A 6 'fn covered( op : TraceOp ) -> bool' ring_trace/tests/trace_test.rs | command grep -c '=> true' | sed 's/^/    arms returning true: /'
command grep -A 6 'fn covered( op : TraceOp ) -> bool' ring_trace/tests/trace_test.rs | command grep -c '=> false' | sed 's/^/    arms returning false: /' || true
```

Live output:

```
  -- the constant, and what its doc says it is for --
  /// Every discriminant, for a test that must cover all of them.
  pub const ALL : [ Self; 5 ] =
  [
    Self::Claim,
    Self::Publish,
    Self::Consume,
    Self::Commit,
    Self::Drop,
  ];
  -- the tripwire the source claims, which is real --
  /// Written as an exhaustive `match` rather than a derive so that adding a
  /// discriminant fails to compile here, where a human then has to say what the
  /// new operation is called.
  ///
  -- the tripwire the suite claims --
fn the_operation_kinds_are_exactly_the_five_declared()
{
  // ALL must stay in step with the enum, but nothing here forces that: the
  // exhaustive match below fails to compile when a discriminant is *added* to
  // the enum — a guard `name()` in src/lib.rs already provides, one compile
  // error earlier — and says nothing about whether the new variant was also
  // added to ALL. Keeping ALL in step with the enum is a manual step; this
  // test's exhaustiveness is a (harmless, redundant) copy of name()'s guard.
  fn covered( op : TraceOp ) -> bool
  {
    match op
    {
      TraceOp::Claim | TraceOp::Publish | TraceOp::Consume | TraceOp::Commit | TraceOp::Drop => true,
    }
  }

  assert_eq!( TraceOp::ALL.len(), 5 );
  -- and what covered() can return --
    arms returning true: 1
    arms returning false: 0
```

## The Case the Second Claim Names, Compiled

*The scratch binary behind this probe is gone — swept, like every
`-tr_probe/` build, per this project's convention for temporary files — so
the compile-and-run below can't be repeated. What it exercises is
language-level (an exhaustive match compiles regardless of what `ALL` lists),
and the shape it mirrors is unchanged: `TraceOp` still declares exactly five
discriminants and `ALL` still lists five. TR27 and TR28 below draw their
conclusions from this recording; read it as preserved evidence, not something
the gate can rerun.*

```rust
// -tr_probe/src/bin/variant_drift.rs
// TraceOp's exact shape, with a sixth discriminant added to the enum and to
// both exhaustive matches — everywhere the compiler demanded — but not to ALL.
enum Op { Claim, Publish, Consume, Commit, Drop, Evict }

pub const ALL : [ Self; 5 ] = [ Self::Claim, Self::Publish, Self::Consume, Self::Commit, Self::Drop ];

// the suite's own two assertions, verbatim in shape
assert_eq!( Op::ALL.len(), 5 );
for op in Op::ALL { assert!( covered( op ) ); }
```

```
  it compiled, and both suite assertions passed
  discriminants the enum declares : 6
  discriminants ALL lists         : 5
  the one the log can never see   : evict
```

---

### TR27 — The Source's Tripwire Is Real; the Suite's Names a Guard That Does Not Exist

`name()`'s doc claims a specific compile-time property: written as an exhaustive
`match` rather than a derive "so that adding a discriminant fails to compile
here, where a human then has to say what the new operation is called". That is
exactly true. A sixth `TraceOp` variant breaks the build at that match, and the
person who added it must name it before the crate compiles again.

The suite claimed a different property and got it wrong. The comment heading
`the_operation_kinds_are_exactly_the_five_declared` said the exhaustive match in
`covered` was written "so adding a discriminant without adding it to ALL fails
to compile here" — text this finding's own fix has since replaced, at
`trace_test.rs:203-208`. The match forces an edit when a discriminant is
*added* — which the crate's own `name()` already forced, one compile error
earlier. It has nothing to say about `ALL`. Add the variant, satisfy
both matches because the compiler insists, leave `ALL` at five: the probe does
exactly that and reports that it compiled and that both of the suite's assertions
passed, with the enum declaring six and `ALL` listing five.

The gap is not academic. `ALL` is what
`an_enabled_trace_records_one_of_every_operation_kind` and
`a_disabled_trace_records_zero_of_every_operation_kind` iterate, so a discriminant
missing from `ALL` is a discriminant those two tests silently stop covering — and
`count_of` for that kind is never exercised again.

**Finding.** A comment describing a guarantee the code does not provide, in the
one place a maintainer would look to confirm it does. Recorded as **misleading
doc** rather than a test defect because the test is harmless; what fails is the
reassurance. The honest version is two clauses: `name()`'s match forces the edit
when a discriminant appears, and nothing forces the corresponding edit to `ALL`,
which is a manual step. There is no stable check that would close it — no
`variant_count` outside nightly, no derive in this crate's dependency set — so
naming the manual step is the whole available remedy, and it is worth more than a
comment claiming the step is automatic.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A1 -F 'nothing here forces that' ring_trace/tests/trace_test.rs
```

Live output:

```
  // ALL must stay in step with the enum, but nothing here forces that: the
  // exhaustive match below fails to compile when a discriminant is *added* to
```

**Disposition:** applied — the test's comment now states the true guard chain:
`name()`'s match, not this one, is what forces an edit when a discriminant is
added, and keeping `ALL` in step remains a manual step this test's
exhaustiveness only redundantly copies. `cargo test --release -p ring_trace
--test trace_test` confirms all 19 tests still pass. Now prints: `nothing here forces that`

---

### TR28 — Neither Assertion in That Test Can Fail

`covered` has one match arm and it returns `true`; there is no arm returning
`false`. So `assert!( covered( op ) )` holds for every value of every type that
can reach it, and the loop around it cannot fail. `assert_eq!( TraceOp::ALL.len(),
5 )` compares the length of a `[ Self; 5 ]` against the literal `5` — the type
already fixed the answer, and the assertion is `5 == 5` written out.

So the test's entire content is compile-time: `covered`'s exhaustiveness. And
that exhaustiveness duplicates `name()`'s, which lives in the crate and therefore
fires first — a new discriminant breaks `src/lib.rs` before the test is ever
built. The test contributes nothing the source does not already have, at either
compile time or run time.

Meanwhile its name is `the_operation_kinds_are_exactly_the_five_declared`, which
promises precisely the property TR27 shows nothing checks: that `ALL` and the
enum agree.

**Finding.** Recorded as an observation rather than a defect — a tautological
assertion costs nothing and the compile-time intent behind it is legitimate, even
if redundant here. What is worth writing down is the shape: a test whose name
states the strongest property in the file, whose body contains two assertions
that cannot fail, and whose one real instrument is a copy of a guard the source
already carries. If the manual `ALL` step is going to stay manual, this test is
the natural place to say so in a comment, since it is where a maintainer looking
for that guarantee arrives.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_six_impl_blocks_and_the_three_a_lint_cannot_reach.md) | The lint covering these declarations |
| [`type/001`](../type/001_five_discriminants_and_the_array_beside_them.md) | The vocabulary the constant enumerates |
| [`pattern/002`](../pattern/002_exhaustive_match_as_a_tripwire.md) | The idiom, across the family |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | Why a sixth discriminant would not break anything downstream |

### Sources

| Fact | Where |
|------|-------|
| The `ALL` constant and its doc | `ring_trace/src/lib.rs:86-94` |
| `name()`'s tripwire claim | `ring_trace/src/lib.rs:98-100` |
| The suite's tripwire claim, as replaced | `ring_trace/tests/trace_test.rs:203-208` |
| `covered` with one arm, returning `true` | Census above |
| Six declared, five listed, compiled and passing | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `the_operation_kinds_are_exactly_the_five_declared` | The claim examined here |
| `an_enabled_trace_records_one_of_every_operation_kind` | Iterates `ALL`, so inherits its drift |
| `a_disabled_trace_records_zero_of_every_operation_kind` | The same, on the disabled path |
| `every_operation_kind_has_its_own_name` | The match that is the real tripwire |
