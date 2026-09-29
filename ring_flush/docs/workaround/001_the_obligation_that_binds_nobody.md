# Workaround: The Obligation That Binds Nobody

### Scope

- **Purpose**: Work out W1 — Rust's lack of linear types — as it actually lands in this crate: what compensation was reachable, what was used, and where the compensation has a hole the crate has not noticed.
- **Responsibility**: The drop-side signals available to the crate, which declarations carry them, and the one that does not.
- **In Scope**: `#[must_use]` placement across `src/lib.rs`; the absence of `Drop` impls; the test that pins the loss.
- **Out of Scope**: Why no flush happens at `Drop` (→ [`pattern/002`](../pattern/002_driven_not_self_firing.md)); the cost of the loss as a lifecycle stage (→ [`lifecycle/001`](../lifecycle/001_the_consolidation_cycle.md)'s U5); `drain_final`'s signature, which is a decision rather than a constraint (→ [`decisions/002`](../decisions/002_the_final_drains_signature.md)).

### What the Language Actually Withholds

W1 names one missing feature and it is worth being exact about which. Rust has:

| Available | What it binds | Reaches W1? |
|-----------|---------------|-------------|
| `#[must_use]` on a type | Any expression of that type discarded as a statement | Partly — only if the obligation is *carried by a return value* |
| `#[must_use]` on a function | That function's own discarded result | Partly, same limit |
| `Drop` | Every value, at scope exit, unconditionally | No — it runs, but it cannot fail, cannot report, and cannot be told to run *now* |
| `#[must_use]` on a *binding* | — | **Does not exist.** This is the gap |

The obligation this crate needs to state is "call `drain_final` on this binding
before it goes out of scope." Every mechanism above binds an *expression* or
runs at a *point*; none binds a live variable. That is the whole of W1, and it
is genuinely external — no arrangement of this crate's code closes it.

**What is reachable is making the obligation loud**, and the crate does that
three ways: the name `drain_final` reads as terminal, the returned
[`FlushOutcome`](../type/002_flush_outcome.md) is `#[must_use]` with a written
reason, and the loss has a test asserting it happens.

### Sources

| File | Relationship |
|------|-----------------|
| [`src/lib.rs`](../../src/lib.rs) | Every `#[must_use]` site, and the absence of a `Drop` impl |

### Lifecycles

| File | Relationship |
|------|-----------------|
| [`../lifecycle/001_the_consolidation_cycle.md`](../lifecycle/001_the_consolidation_cycle.md) | U5 — the loss, as a stage of the cycle |
| [`../lifecycle/002_from_configuration_to_the_final_drain.md`](../lifecycle/002_from_configuration_to_the_final_drain.md) | Teardown as a named phase, W1's main compensation |

### Patterns

| File | Relationship |
|------|-----------------|
| [`../pattern/002_driven_not_self_firing.md`](../pattern/002_driven_not_self_firing.md) | Why a flushing `Drop` was rejected on its own merits, independently of W1 |

### Types

| File | Relationship |
|------|-----------------|
| [`../type/002_flush_outcome.md`](../type/002_flush_outcome.md) | The one type carrying `#[must_use]`, and the message it carries |

### Workarounds

| File | Relationship |
|------|-----------------|
| [`002_the_compilation_boundary_that_was_never_built.md`](002_the_compilation_boundary_that_was_never_built.md) | W2, the other constraint, and the one whose compensation is stale |

### Tests

| Test | Relationship |
|------|--------------|
| `dropping_a_driver_with_records_staged_publishes_nothing` | The loss, asserted as behaviour rather than described as a hazard |
| `a_refused_final_drain_keeps_the_records` | What the caller must do about the outcome the type forces them to read |

### FL49 — The Type Carries the Obligation for Three Methods and the Fourth Returns `Self` Unguarded

Where the crate's ten `#[must_use]` sites actually sit:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every must_use, and the declaration it guards --'
awk '/must_use/{ m = NR } m && NR > m && NR <= m + 3 && /^(pub |  pub )/ { printf "    %3d  %s\n", m, $0; m = 0 }' \
  ring_flush/src/lib.rs
echo '  -- the four verbs that return something and are not in that list --'
command grep -E '^  pub fn (with_log|drive|drive_at_barrier|drain_final)' ring_flush/src/lib.rs
echo '  -- Drop impls in the whole family --'
printf '    %s\n' "$( command grep -rc 'impl Drop' ring_*/src/*.rs 2>/dev/null | command grep -cv ':0' )"
```

Live output:

```
  -- every must_use, and the declaration it guards --
    126  pub enum FlushOutcome
    230    pub const fn count( &self ) -> usize
    291    pub const fn new() -> Self
    302    pub fn entries( &self ) -> &[ FlushEntry ]
    309    pub fn len( &self ) -> usize
    318    pub fn is_empty( &self ) -> bool
    446    pub const fn policy( &self ) -> FlushPolicy
    458    pub fn staged( &self ) -> usize
    479    pub fn buffer_capacity( &self ) -> usize
    486    pub fn log( &self ) -> Option< &FlushLog >
  -- the four verbs that return something and are not in that list --
  pub fn with_log( mut self ) -> Self
  pub fn drive( &mut self ) -> FlushOutcome
  pub fn drive_at_barrier( &mut self ) -> FlushOutcome
  pub fn drain_final( &mut self ) -> FlushOutcome
  -- Drop impls in the whole family --
    0
```

Ten `#[must_use]` sites and nine of them are accessors — `len`, `staged`,
`policy`, `log` and the rest, where discarding the result is merely pointless.
The tenth is on `FlushOutcome` the *type*, and it is the load-bearing one: it
carries a written message, and it means `drive`, `drive_at_barrier` and
`drain_final` all inherit the warning without any of the three needing the
attribute. That is the right place to put it — the obligation belongs to the
outcome, not to the three routes that produce one.

**`with_log` is the one verb that returns something and inherits nothing.** It
takes `self` by value and returns `Self`, so `flusher.with_log();` written as a
statement compiles, moves the flusher into the call, drops the value that comes
back, and leaves the caller with no flusher and no warning. If records were
staged, that is W1's exact failure mode — a binding dropped with an obligation
outstanding — reachable in one line, inside the crate whose whole subject is
that failure mode, through the one method the language *could* have guarded.

**Zero `Drop` impls exist across all thirty-three crates**, so `#[must_use]` is
not merely the family's preferred drop-side signal; it is the only one present.
Nothing in the family observes a value going out of scope.

Recorded rather than fixed because the fix is one attribute and the question is
whether the builder is the only such site — a family-grain sweep for
`-> Self` without `#[must_use]` would answer that, and this crate can only see
its own.

### FL50 — The Unpreventable Loss Is Pinned by a Test, Which Makes It Specified Rather Than Merely Possible

W1's row says the loss is the one class of data loss this crate cannot detect,
report, or prevent. The suite detects it exactly:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the test that exercises the drop path --'
command grep 'fn dropping_a_driver_with_records_staged_publishes_nothing' ring_flush/tests/flush_test.rs
echo '  -- what it asserts, scoped to its own body rather than a line window --'
awk '/^fn dropping_a_driver_with_records_staged_publishes_nothing/{ i = 1; next }
     i && /^\}/ { i = 0 }
     i && /assert/ { printf "    %d: %s\n", NR, $0 }' ring_flush/tests/flush_test.rs
echo '  -- and how the workaround row describes the same thing --'
command grep -o 'cannot detect, report, or prevent' ring_flush/docs/workaround/readme.md \
  | sed 's/^/    /'
```

Live output:

```
  -- the test that exercises the drop path --
fn dropping_a_driver_with_records_staged_publishes_nothing()
  -- what it asserts, scoped to its own body rather than a line window --
    443:     assert_eq!( flusher.staged(), 5 );
    446:   assert_eq!( consumer.len(), 0, "a drop published — the publication point is not the caller's" );
    448:   assert_eq!( consumer.try_recv_batch( &mut landed ), 0 );
  -- and how the workaround row describes the same thing --
    cannot detect, report, or prevent
```

The row and the test are both correct and they are about different subjects. The
row is about *production*: a consumer who forgets gets no error, no log line and
no returned value, because there is no route by which the crate could produce
one. The test is about *specification*: in a controlled scope, with the drop
deliberately provoked, the absence of publication is asserted.

**The consequence is the one worth recording.** A hazard nobody has written down
can be removed by whoever decides it was a bad idea. A hazard with a passing test
asserting it occurs is a *contract*: a future `Drop` impl that flushed the staged
records — the obvious well-meaning fix, and the one W1's phrasing invites —
would now turn this test red, and the person who wrote it would have to decide
whether the redness is a regression or a correction.

That is the right outcome and it was not the stated intent. The test's own doc
comment argues the design (a flushing `Drop` would publish at a point nobody
chose, and would need a producer that might refuse with no caller left to hear).
So the reasoning is recorded where it will actually be read — at the point of
failure — which is better than the workaround table, and it means **W1's row
understates its own compensation.** The row lists documentation, a name and a
visible outcome. It does not list the test, and the test is the only one of the
four that will interrupt somebody.
