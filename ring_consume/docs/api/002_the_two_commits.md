# API: The Two Commits

### Scope

**Purpose:** Establish the contract shared by `commit` and `commit_available`,
where the two diverge, and what a caller must know to choose between them.

**Responsibility:** The two commit operations as a public contract — signatures,
return types, failure modes, and the guidance a caller has.

**In Scope:** `Consumer::commit` and `Consumer::commit_available`; their doc
comments and doctests; the choice between them.

**Out of Scope:** Their implementations' relationship — that is
[`pitfall/001`](../pitfall/001_commit_available_does_not_call_commit.md). The
guard's mechanics, which are
[`algorithm/002`](../algorithm/002_the_two_sided_guard.md).

---

## Two Signatures

```rust
pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
pub fn commit_available( &self ) -> Seq
```

One takes a target and may refuse. One takes nothing and cannot fail. Both
advance the same cursor with the same ordering, and both mean the same thing to
the producer: everything before this sequence has been read and its slots may be
reused.

| | `commit` | `commit_available` |
|--|---------|-------------------|
| Argument | a `Seq` the caller chose | none |
| Can fail | ✔ — `RingError::Empty`, two causes | ✘ |
| Return | the sequence committed | the sequence reached |
| Partial commit | ✔ | ✘ — always the whole run |
| Runs the guard | ✔ | ✘ — the value is in range by construction |

### CN20 — The Contract Says When Each Is Safe and Not When Each Is Right

Both doc comments are good on mechanics. `commit`'s `# Errors` names both
refusal conditions and their consequences
([`algorithm/002`](../algorithm/002_the_two_sided_guard.md) CN16);
`commit_available`'s doctest shows the cursor moving to the frontier and staying
there. Neither answers the question a caller actually arrives with.

That question is: **I have read some of what was available. Which do I call?**

The answer is `commit( last_read.next() )` — and getting there requires the
caller to know four things the documentation does not state together:

| A caller must know | Stated where |
|--------------------|--------------|
| `through` is exclusive — the first sequence *not* read | inferable from `# Errors`; never said directly |
| committing part of a run is legitimate | only from the guard accepting values between the ends |
| `commit_available` commits everything, read or not | said clearly |
| so `commit_available` after a partial read is corruption | **nowhere** |

That last row is the gap. `commit_available` is the shorter call, the one that
cannot fail, and the one whose name reads as the obvious default. It is correct
exactly when the caller has drained the entire run and wrong — silently, and in
the way the module documentation's whole opening argument is about — when the
caller read part of it.

A caller who reads eight of thirty available sequences and calls
`commit_available()` has told the producer that all thirty are free. Twenty-two
unread slots become writable. Nothing fails, nothing warns, and the loss
surfaces later as data that was never read, in a thread that did nothing wrong.

The module documentation makes the general argument — the window between
`available` and `commit` is a read of borrowed slots — but it makes it about the
*window*, and this mistake happens at the commit, in a caller that did respect
the window and simply used the wrong one of two commits.

One sentence on `commit_available` would close it: *call this only when the
whole available run has been read; after a partial read use
`commit( first_unread )`.*

**Cost:** reachable, and it is the crate's most likely real-world defect. The
failure is silent, the wrong call is the more attractive one, and the guidance
that would prevent it does not exist in either function's documentation.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '  /// Call this only when the whole available run has been read.' ring_consume/src/lib.rs
```

Live output:

```
  /// Call this only when the whole available run has been read. After a
  /// partial read, use [`commit`]`( first_unread )` instead — committing
  /// everything here tells the producer that slots which were never read are
```

**Disposition:** applied — `commit_available`'s doc comment now carries
exactly the one sentence this finding names: call it only after the whole
available run has been read, and use `commit( first_unread )` after a partial
read. The crate's 22 tests (1 `allocation_test.rs` + 21 `consume_test.rs`)
plus 17 doctests re-verified passing (`cargo test --all-features`,
2026-09-04). Now prints:
`Call this only when the whole available run has been read.`

---

### CN21 — The Fallible One Cannot Fail Where It Is Meant to Be Used

`commit` returns `Result`, and in the usage the crate is designed around it
never returns `Err`:

```rust
let run = consumer.available();
// ... read run ...
consumer.commit( run.end() ).unwrap();   // cannot fail
```

`run.end()` came from `available()`, and `commit`'s guard accepts everything in
`[ run.start(), run.end() ]`. Between the two calls the run can only *grow* —
the producer publishes, the frontier advances, `start` is the consumer's own
cursor which only it moves. So the value is still in range, and the `Result` is
an obligation the caller discharges with `unwrap` or `let _ =` at every
correctly-written call site.

The `Result` is not useless — it catches the two genuine caller errors
(committing backwards, committing past the frontier), and those are real
mistakes worth catching. But its cost falls entirely on correct code, and its
benefit falls entirely on incorrect code, which is the opposite of where a
caller would expect an API's ergonomics to land.

Two consequences worth recording:

**`unwrap()` in the happy path is idiomatic here and looks like a smell.** A
reviewer seeing `consumer.commit( run.end() ).unwrap()` reasonably asks what
happens when it fails. The answer — it cannot, given that `run` came from
`available()` — is not written down anywhere, so the reviewer either accepts an
unexplained `unwrap` or adds error handling for an unreachable branch.

**The crate's own doctest models the failing case, not the working one.**
`commit`'s doctest asserts `Err`, `Ok`, `Err` — two refusals and one success,
demonstrating the guard. It never shows the ordinary sequence of
`available()` → read → `commit( run.end() )`, which is the only pattern most
callers will write. The one place a caller would look for the idiom shows them
the diagnostics instead.

`commit_available` has the opposite shape: it cannot fail, its doctest shows the
ordinary use, and it is the one that is dangerous after a partial read (CN20).
Between them the two functions manage to put the safety in the one with the
awkward ergonomics and the ergonomics in the one with the hazard.

**Cost:** reachable as an ergonomics and documentation cost. Neither function
is wrong; the pair pushes callers toward the more dangerous one and gives the
safer one no worked example of correct use.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A2 'ordinary call site never takes the error arm' ring_consume/src/lib.rs
grep 'consumer.commit( run.end() )' ring_consume/src/lib.rs
```

Live output:

```
  /// The ordinary call site never takes the error arm shown above: a value
  /// read from [`available`] is always still in range when it reaches
  /// `commit`, because between the two calls the run can only grow.
  /// assert_eq!( consumer.commit( run.end() ), Ok( Seq( 4 ) ) );
```

`cargo test --release -p ring_consume` (isolated `CARGO_TARGET_DIR`) confirms the
addition compiles and passes as a second, distinct doctest on `commit` —
`test ring_consume/src/lib.rs - Consumer<'a>::commit (line 388) ... ok`
alongside the original at line 366 — and the crate's other 37 unit/doctest
cases are unaffected.

**Disposition:** applied — `commit`'s doc comment now carries a second example
showing the ordinary `available()` → read → `commit( run.end() )` sequence
succeeding, immediately after the one that only shows refusals. Now prints: `assert_eq!( consumer.commit( run.end() ), Ok( Seq( 4 ) ) );`

---

## What Is Correctly Absent

| Not present | Correctly so |
|-------------|--------------|
| a `commit_unchecked` | the check is two comparisons against values already loaded; skipping it saves nothing |
| a batched multi-commit | the cursor is a single position; committing twice is committing the later value |
| a `try_commit` returning `bool` | `Result` already carries the same information and composes with `?` |
| distinct errors for the two refusals | `RingError` is `ring_types`' surface — see [`algorithm/002`](../algorithm/002_the_two_sided_guard.md) CN17 |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| api | [001](001_sixteen_public_items.md) | the surface these two sit in |
| algorithm | [002](../algorithm/002_the_two_sided_guard.md) | the guard `commit` runs and `commit_available` does not |
| pitfall | [001](../pitfall/001_commit_available_does_not_call_commit.md) | the duplicated store behind the two signatures |
| decisions | [001](../decisions/001_two_calls_not_one.md) | the window CN20's mistake happens at the end of |
| lifecycle | [001](../lifecycle/001_a_sequence_from_published_to_committed.md) | what committing does to a sequence's state |

### Sources

| What | Where |
|------|-------|
| `commit` | `ring_consume/src/lib.rs:425-435` |
| `commit_available` | `ring_consume/src/lib.rs:468-479` |
| `commit`'s doctest | `ring_consume/src/lib.rs:386-401` |
| `commit_available`'s doctest | `ring_consume/src/lib.rs:446-460` |

### Tests

| Claim | Verified by |
|-------|-------------|
| Partial commit is accepted | `consume_test.rs`, the inclusive-range test |
| `commit_available` takes everything | `consume_test.rs:282-288` |
| A second `commit_available` is a no-op | `consume_test.rs:293` |
| `commit( run.end() )` cannot fail | the guard accepts `[ start, end ]`; `end` is its own upper bound |
