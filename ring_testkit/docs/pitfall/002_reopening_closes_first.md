# Pitfall: Reopening Closes First

### Scope

- **Purpose**: Record that there is no way to reopen a `Shutdown` without closing it, and what that costs a concurrent caller.
- **Responsibility**: The mechanism, why it is deliberate in `ring_shutdown`, and what the fixture does about it.
- **In Scope**: `Step::Reopen`.
- **Out of Scope**: Whether `ring_shutdown` should offer an unconditional `open` — that is its decision, not this crate's. Raised in [`../decisions/readme.md`](../decisions/readme.md) Pending 2.

### The mechanism

`Stopped::reopen` takes `self` **by value**, and `Shutdown::close` is the only
thing that produces a `Stopped`. `ring_shutdown` states the reason directly:

> Taking `self` by value is the point: a drain is only sound while the ring is
> closed, so the proof that it is closed must not survive reopening.

That is a good rule. Its consequence is that reopening is only reachable
*through* closing:

```rust
Step::Reopen => shutdown.close().reopen(),
```

On an already-closed ring this is exactly right — `close` is idempotent, and
the token it hands back is spent immediately. On an **already-open** ring it
closes and reopens, which is a no-op in every observable the fixture records,
and a real window in a concurrent program.

### The window

| Setting | What `Reopen` does | Observable |
|---|---|---|
| Ring already closed | Reopens it | The intended effect |
| Ring already open, single thread | Closes, then reopens, with nothing in between | Nothing — `closed_at_end` is `false` either way |
| Ring already open, concurrent producers | Closes, then reopens | Another thread's `Guarded::try_push` landing inside the window is refused `Refusal::Closed`, on a ring nobody asked to close |

**The third row is not reachable from this crate's fixture**, which is
single-threaded by construction, and that is precisely why it is written down.
A pitfall that the test suite can demonstrate does not need a document; this one
is invisible to every test here and would be discovered by whoever first wraps
a `Script` around real threads.

### Consequences

| # | Consequence | Where it lands |
|---|---|---|
| Q1 | `Step::Reopen` is documented as closing first, on the variant itself | A caller reading the enum sees it without reaching for this file |
| Q2 | The no-op case is pinned by a test rather than assumed | `reopening_an_open_ring_is_observably_a_no_op` compares a script with a leading `Reopen` against the same script without one |
| Q3 | The fixture does not work around it | Skipping the close when `is_closed()` is already false would be a check-then-act race in exactly the concurrent setting where the window matters — a workaround that fails only where it is needed |

**Q3 is the decision worth stating twice.** `if !shutdown.is_closed() { .. }`
would make the single-threaded no-op cheaper and the concurrent hazard *worse*,
because the gap between the check and the close is another window. The honest
move is to leave the mechanism alone and document it.

### Evidence

| # | Claim | Test |
|---|---|---|
| R1 | `Reopen` after `Close` admits publications again | `reopening_admits_publications_again` |
| R2 | `Reopen` on an open ring changes no observable | `reopening_an_open_ring_is_observably_a_no_op` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the Reopen arm, verbatim:        %s\n' "$( awk '/pub fn run\( &self/{f=1} f && /Step::Reopen/{ sub( /^ */, "" ); print; exit }' ring_testkit/src/lib.rs )"
printf 'loom model closures in the crate: %s\n' "$( command grep -c 'loom::model( ||' ring_testkit/tests/exhaustive_test.rs || true )"
printf 'Shutdown in the loom file:       %s\n' "$( command grep -c 'Shutdown' ring_testkit/tests/exhaustive_test.rs || true )"
printf 'close or Refusal there:          %s\n' "$( command grep -cE 'close|Refusal' ring_testkit/tests/exhaustive_test.rs || true )"
printf 'loom threads spawned there:      %s\n' "$( command grep -c 'loom::thread::spawn' ring_testkit/tests/exhaustive_test.rs || true )"
printf 'real threads in the ordinary suite: %s\n' "$( command grep -c 'std::thread::spawn' ring_testkit/tests/testkit_test.rs || true )"
printf 'ring_shutdown doc definitions:   %s\n' "$( ls -d ring_shutdown/docs/*/ | command grep -cv '/definition/$' )"
printf 'of them, a decision log:         %s\n' "$( ls -d ring_shutdown/docs/decisions 2>/dev/null | wc -l )"
printf 'decisions it has filed:          %s\n' "$( ls ring_shutdown/docs/decisions/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'what those ask, in order:        %s\n' "$( head -qn1 ring_shutdown/docs/decisions/[0-9][0-9][0-9]_*.md | sed 's/^# Decision: //' | tr '\n' '/' | tr '\140' '~' )"
printf 'its docs asking for an open:     %s\n' "$( command grep -rl 'unconditional open' ring_shutdown/docs/ 2>/dev/null | wc -l )"
printf 'its docs naming Step::Reopen:    %s\n' "$( command grep -rl 'Step::Reopen' ring_shutdown/docs/ 2>/dev/null | wc -l )"
printf 'what its docs defend instead:    %s\n' "$( command grep -rhoE 'cannot be obtained without closing first' ring_shutdown/docs/ | head -1 )"
printf 'files in ring_shutdown naming us: %s\n' "$( command grep -rl 'ring_testkit' ring_shutdown/ 2>/dev/null | wc -l )"
printf 'of those, docs rather than code: %s\n' "$( command grep -rl 'ring_testkit' ring_shutdown/docs/ 2>/dev/null | wc -l )"
```

Live output:

```
the Reopen arm, verbatim:        Step::Reopen => shutdown.close().reopen(),
loom model closures in the crate: 3
Shutdown in the loom file:       0
close or Refusal there:          0
loom threads spawned there:      6
real threads in the ordinary suite: 2
ring_shutdown doc definitions:   13
of them, a decision log:         1
decisions it has filed:          2
what those ask, in order:        Should ~Guarded::into_inner~ Exist/Should a ~Stopped~ Token Be Unique/
its docs asking for an open:     0
its docs naming Step::Reopen:    0
what its docs defend instead:    cannot be obtained without closing first
files in ring_shutdown naming us: 12
of those, docs rather than code: 11
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | Where `Step::Reopen` sits among the steps |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_states_a_script_moves_through.md](../lifecycle/001_the_states_a_script_moves_through.md) | The open/closed transitions, with this one marked as passing through both |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_shutdown/src/lib.rs`](../../../ring_shutdown/src/lib.rs) | `Stopped::reopen` and its by-value `self` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | R1, R2 |

### TK43 — the crate owns a model checker and never points it at the one hazard that needs one

The window table's third row is the whole reason this file exists, and it carries
no evidence row because it is *"not reachable from this crate's fixture, which is
single-threaded by construction"*. That sentence is true of `Script::run`. It is
not true of the crate.

`tests/exhaustive_test.rs` runs **three** `loom::model` closures and spawns
**six** loom threads inside them — the only file in all 33 `ring_*` crates that
does. `tests/testkit_test.rs` spawns **two** real threads. So the crate has both
a model checker and ordinary concurrency, and eight thread spawns between them.

The count of `Shutdown` in the loom file is **zero**. So is the count of `close`
and `Refusal`. The three models exercise a producer and a consumer against a
leaked ring's ends; not one of them constructs the type whose only documented
hazard is concurrent.

This is not a demand that the model be extended — a loom model of a `Reopen`
window would need a `Shutdown`, two producers and an interleaving budget, and it
would be a real piece of work. It is a statement of where the crate's own
resources sit relative to its own recorded risk. The document says the hazard is
unreachable; the tool that would reach it is in the same `tests/` directory, and
neither the pitfall nor the manual plan mentions the possibility.

### TK44 — the question is parked in the consumer's decision log, where its owner will never read it

The Out of Scope line defers *"whether `ring_shutdown` should offer an
unconditional open"* to this crate's [`../decisions/readme.md`](../decisions/readme.md)
Pending 2 — and correctly declines to answer it, because it is `ring_shutdown`'s
call and not this crate's.

`ring_shutdown` has thirteen doc definitions, one of which is a decision log of
its own, and that log has since filed **two** decisions. Neither is this one.
They ask *"Should `Guarded::into_inner` Exist"* and *"Should a `Stopped` Token Be
Unique"* — the second lands next door to the question parked here, since both
turn on `reopen` consuming its receiver, and still asks something else: whether
the token can be *duplicated*, not whether the ring can be opened without one.
Zero of `ring_shutdown`'s docs contain the phrase *unconditional open*, and zero
name `Step::Reopen`. What its docs carry instead is the defense — that a
`Stopped` *"cannot be obtained without closing first"* — stated as settled, with
no record that a consumer ever found the consequence expensive.

**The traffic is not the problem, which is what makes this worth keeping.** When
this file was first written, exactly one file in all of `ring_shutdown` named
`ring_testkit`, and it was a comment in a test listing dependents — an easy
story to tell about two crates that do not talk. That story is now false: **ten**
files there name this crate, **nine** of them documents. `ring_shutdown` has read
this crate closely enough to cite its `Stopped` binding as the only one outside
itself, its guard as the only one held outside itself, and its manifest entry as
the edge that arrived before the family expected it. It went through this crate
line by line and came back with none of the question.

So the channel exists and carries detail in both directions, and the pending
question still did not travel. That rules out the cheap explanation and leaves
the structural one: **a question filed in a consumer's decision log is addressed
to nobody.** Nothing reads it, nothing forwards it, and citation traffic — of
which there is now a great deal — moves facts about code, not open questions
about design.

This is a placement problem, not a content problem. The analysis in this file is
where it belongs — it is the consumer's experience of the owner's design, and
the consumer is the only one who can report it. What is missing is the second
half: nothing carries the question upstream, so `ring_shutdown`'s decision log
reasons about the by-value `self` entirely from the inside — what the token
proves, whether two can exist — and never from the one report that says what it
costs to call.
