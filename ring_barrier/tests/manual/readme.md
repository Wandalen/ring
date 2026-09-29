# ring_barrier — manual testing plan

`tests/barrier_test.rs` covers this crate's half of
`docs/feature/178_sequence_barrier_and_gating_set.md` — the minimum across sets
of 1, 2 and 3 cursors — and covers it exhaustively: the minimum is placed at
every index of every set size from one to eight, so a fold that reads an end
rather than folding fails 30 of those 36 cases.

The risk this crate carries is not in the fold. It is that `ring_barrier` and
`ring_gating` look like the same crate. Both hold a set of cursors, both take a
minimum, and the difference — capacity is in one answer and not the other — is
invisible in every test where the frontier happens to be under one lap. The
first three checks below all read for that one confusion, from three directions.

Run from the workspace root. The code-line filter is `^[[:space:]]*//`, which
drops `///`, `//!` and plain `//` alike: this crate's prose argues at length
about capacity and about `ring_gating`, and a filter that kept ordinary comments
would report every one of those sentences as a code hit.

## B1 — capacity does not appear in the barrier's arithmetic

The module documentation's central claim. A barrier answer is bounded by what
dependencies have finished, not by how many slots exist; a capacity reaching the
computation means the two crates have merged.

```bash
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -nE "capacity|Capacity"
```

**Expected:** no output. `Capacity` is not imported and does not appear at all,
not even in a doc example — which is a weaker result than it was before the
signature changed, and worth saying so. While `Barrier::over` took a
`&GatingSet`, every doc example here had to build one, so a capacity was in the
file and this check was reading real restraint. Now the constructor takes a bare
`&[ PaddedCursor ]` and there is nothing to build; the check passes because
capacity has no way in, not because it was kept out.

So the load-bearing assertion moved to the test file.
`available_ignores_capacity_entirely` reads one set of cursors from both sides
at once — `GatingSet::headroom` clamped to 4, `Barrier::available` reporting
1,000 over the same cursors — which is the claim this grep used to make and can
no longer make on its own.

## B2 — the dependencies are a borrowed slice, never an owned aggregate

Two mistakes, one check. Owning a second `Vec<PaddedCursor>` would compile, pass
the whole sequential suite, and be wrong the first time a dependency advanced —
the barrier would read a snapshot taken at construction while the producer gated
on the live one. Borrowing a `&GatingSet` instead of a slice is the subtler one:
it also compiles and also passes, and it makes the four-operation handshake
unwireable, because a `GatingSet` owns its cursors and a `Publisher`'s cursor
can therefore never become a dependency of anything.

```bash
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -nE "Vec<|Vec ?<|GatingSet|PaddedCursor"
```

**Expected:** no `Vec` and no `GatingSet` of any kind — five hits, all
`PaddedCursor`: the `use`, the field `dependencies : &'a [ PaddedCursor ]`,
`over`'s parameter, `dependencies()`'s return, and `cursor()`'s
`Option< &'a PaddedCursor >`. The `'a` on the last two is what ties a handed-out
cursor back to the borrowed slice, so a caller cannot outlive what it reads.

That second mistake is not hypothetical — it is what this crate did until
`ring_publish/tests/handshake_test.rs` could not be written, which is the
staged-validation loop working as intended: build the machinery, let it tell you
the design is wrong.

## B3 — the two empty-set answers are both deliberate

`available` returns 0 for an empty barrier; `ring_gating::headroom` returns a
full capacity for an empty set. That is the family answering the same-shaped
question two opposite ways on purpose, and the shape of mistake here is
"fixing" one to match the other.

```bash
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -nE "map_or|ok_or|unwrap_or"
```

**Expected:** exactly two hits — `map_or( 0, … )` in `available` and
`ok_or( RingError::Empty )` in `wait_for`. No `unwrap_or`, and in particular no
`unwrap_or( Seq::ZERO )`: `frontier` returns `Option` so that "no dependencies"
stays distinguishable from "dependencies, all at zero", and an `unwrap_or`
anywhere in this file collapses that distinction back.

`an_empty_barrier_and_an_empty_gating_set_answer_oppositely` asserts the pair
together, so the two crates cannot be quietly aligned without a test failing.

## B4 — every declared dependency is actually used

Three real dependencies and one dev-dependency, and each needs checking against
its own consumer. A single command over the whole manifest — which is what this
check was before `ring_gating` moved to `[dev-dependencies]` — reports
`ring_gating` as unused, because the library indeed does not use it and must
not.

```bash
# The library's own dependencies, against the library.
comm -23 \
  <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_barrier/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )

# The dev-dependency, against the tests.
comm -23 \
  <( awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f' ring_barrier/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -hvE "^[[:space:]]*//" ring_barrier/tests/*.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output from either. `ring_types`, `ring_cursor` and `ring_wait`
are named in the library; `ring_gating` is named only in the tests.

`ring_wait` is the one easy to lose — it is used only by `wait_for`, the method
most likely to be seen as redundant with the caller's own retry loop.
`ring_gating` is the one easy to *re-promote*: three tests assert the
relationship between this crate's answers and that crate's over one set of
cursors, and the shortest way to make them compile is to move the dependency
back up, which would quietly restore the coupling B2 exists to prevent.

## B5 — `wait_for` returns the frontier, not the request

A consumer that waited for one item and found six should drain six. Returning
`from + count` instead type-checks, satisfies any test that asks for exactly
what is available, and throws away every batch that waiting discovered — which
shows up as a throughput result, not a failure.

```bash
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -n -A 5 "pub fn wait_for"
```

`-A 5` rather than `-A 3`: the signature spans two lines, so a three-line window
stops at the wait call and never reaches the return — a check that cannot see
the line its expected result is about.

**Expected:** the body waits and then returns `self.frontier()`, with no
arithmetic on `from` or `count` anywhere in it. `wait_for_returns_the_frontier_and_not_the_requested_count`
asserts the outcome; this reads the structure, because the arithmetic version
passes that test whenever the frontier happens to equal the request.

## B6 — the concurrent tests actually have a concurrent writer

Two tests in the suite spawn a thread. Both would still pass with the spawn
removed — `a_barrier_never_reports_a_frontier_a_dependency_has_not_reached`
asserts a cursor that never moves, and it would pass trivially against a set
nobody is touching. The spawn is what makes it a race test rather than a
tautology.

```bash
grep -c "thread::scope" ring_barrier/tests/barrier_test.rs
grep -n "scope.spawn" ring_barrier/tests/barrier_test.rs
```

**Expected:** `2` from the first, and two `scope.spawn` calls from the second.
If either disappears, the crate keeps a green suite and loses its only coverage
of the property it exists to provide.

---

## Run Record

| Date | Checks | Result |
| ---- | ------ | ------ |
| 2026-08-28 | B1–B6 | 6/6 as expected |
| 2026-08-28 | B1–B6 | 6/6 after B1, B2 and B4 were rewritten for the slice-based `Barrier` and `ring_gating`'s move to `[dev-dependencies]` — B4 had genuinely started failing |
