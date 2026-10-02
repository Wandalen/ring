# ring_wait manual testing plan

`tests/wait_test.rs` asserts the clauses
`docs/feature/173_wait_kind_and_strategies.md` is graded on: all four strategies
present, `None` non-blocking, every wait bounded. Those are countable.
`none_evaluates_the_predicate_exactly_once` counts to one, and
`escalation_terminates_from_every_starting_point` walks four chains to their
ends.

What the suite cannot see is the *cost* of a strategy, which is the only reason
there are four of them. `Spin`, `Yield`, and `Park` are behaviourally identical
under test: each returns true, each loops again, and a suite of three could be
one. The difference is entirely in what happens to the core between two
readings, and no assertion in a test file can observe that. So the checks below
read the source, and two of them read it for a thing a passing test would
actively hide.

Run from the workspace root. The code-line filter here is
`^[[:space:]]*//`, which drops `///`, `//!` and plain `//` alike. This crate's
`Park` arm carries a six-line ordinary comment mentioning `thread::park`, and a
filter that only dropped doc comments would read that prose as code.

## W1. Every wait is bounded, structurally

The module documentation argues it. An unbounded wait on a ring whose producer
has died is a hung thread with no diagnostic, and under `Park` it is a hung
thread that does not even burn CPU to show it. A test cannot assert the absence
of a `loop {}`. It can only time out.

```bash
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -nE "loop[[:space:]]*\{|while[[:space:]]+true"
```

**Expected:** no output. Every repetition in this crate is a `for` over a
counted range, so the budget is in the loop header where it cannot be
forgotten, rather than in a `break` somewhere in the body where it can.

## W2. The two waits fail with opposite errors

`for_space` and `for_data` are one line each over `wait_until`, and the one line
is the point. A producer handed `Empty` reads it as "nothing to do" and stops;
what it was told is "back-pressure, retry". The error is the whole
difference between the two functions.

```bash
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -nE "RingError::[A-Za-z]+"
```

**Expected:** exactly two hits: `RingError::Empty` as `wait_until`'s own
give-up value, and `RingError::Full` in `for_space`'s `map_err`. `for_data`
appears in neither. It wants `wait_until`'s error unchanged, and adding a
`map_err( |_| RingError::Empty )` there for symmetry would be a line that says
nothing and one more place to get it wrong.

## W3. `None` returns false, and that is the only thing making it non-blocking

`WaitKind::None` is the variant the tick path cannot do without, and it is
non-blocking because `pause` returns `false` for it and `wait_until` breaks on
that. There is no timeout, no budget of one, no special case in the loop. It
is one `false`.

```bash
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -nE "WaitKind::None"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -nE "if[[:space:]]+!pause|break"
```

**Expected:** two hits from the first: the `escalation_hint` arm returning
`None`, and the `pause` arm returning `false`. From the second, the `if !pause`
and its `break`, together on consecutive lines. If `wait_until` ever gains a
`if kind == WaitKind::None` special case, the non-blocking guarantee has moved
out of `pause` and into the loop, and `pause`'s own contract, documented as
"returns whether the caller should try again at all", has quietly become
advisory.

## W4. `Park` does not park, and says so

The `Park` arm sleeps. That is a deliberate, documented deviation from its own
name, because real parking needs the publisher to hold the waiter's handle and
unpark it, which is a registration relationship `ring_handle` owns. The risk is
that the sleep loses its explanation and looks like an oversight to the next
reader, who fixes it by reaching for `thread::park` and produces a thread nobody
will ever unpark.

```bash
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -nE "thread::(park|sleep|yield_now)|spin_loop"
grep -c "thread::park" ring_wait/src/lib.rs
```

**Expected:** three hits from the first: `spin_loop`, `yield_now`, `sleep`.
There is **no `thread::park` among them**. The second command must return
non-zero, because `thread::park` appears in the file exactly where it should, in
the comment explaining why it is not called.

## W5. The pause hint is a hint, not a busy loop

`Spin` emits `core::hint::spin_loop()` rather than spinning on an empty body.
Removing the hint changes no test outcome and costs measurable throughput on the
loop exit, so it is exactly the kind of line a cleanup pass deletes.

```bash
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -n -A 5 "WaitKind::Spin =>$"
```

The `$` anchor matters. Without it the pattern also matches
`escalation_hint`'s single-line `WaitKind::Spin => Some( WaitKind::Yield ),`,
and the check returns two blocks of which only one is the subject. `pause`'s arm
is the one whose `=>` ends its line, because its body is a block.

**Expected:** one block, whose body contains `core::hint::spin_loop()` inside a
bounded `for`. An empty loop body here is a `Spin` strategy that still passes
every assertion in the suite and starves its hyperthread sibling for the
duration of the wait.

## W6. Every declared dependency is used

```bash
comm -23 \
  <( grep -E "^ring_" ring_wait/Cargo.toml | cut -d' ' -f1 | sort ) \
  <( grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output. `ring_cursor` earns its place through `for_space` and
`for_data`; before those existed the dependency was declared and unused, which
`cargo udeps` would have caught at level 4 and this check catches now.

---

## Run Record

| Date | Checks | Result |
| ---- | ------ | ------ |
| 2026-08-28 | W1–W6 | 6/6 as expected |
