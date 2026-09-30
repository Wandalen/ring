# invariant

Two properties hold over the same thirteen-line loop, and neither is checked by
the same kind of instrument. The first is structural — no unbounded repetition
exists in the crate — and is enforced by a grep, because no test can assert the
absence of a `loop {}` without becoming a stopwatch. The second is behavioural —
`WaitKind::None` looks exactly once — and is enforced by a count, because the
stopwatch that also covers it would pass a `None` that spun 1024 times.

Both instances are also honest about where the guarantee stops: at the closure
boundary, in both cases, and completely.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Every Repetition Is a Counted `for`](001_every_repetition_is_a_counted_for.md) | W1, the two loops and their bounds, why the budget sits in the `for` header, and the three escapes the invariant does not close |
| 002 | [`None` Looks Exactly Once](002_none_looks_exactly_once.md) | W3, the single `false` that implements it, the stopwatch-versus-count comparison, and the three tests that bracket the property |

### The Two Properties and Their Instruments

```sh
cd "$(git rev-parse --show-toplevel)"

# W1 — structural. Expected: no output.
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "loop[[:space:]]*\{|while[[:space:]]+true"

# W3 — behavioural. Expected: two `WaitKind::None` arms, and no `if kind ==`.
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs | grep -E "WaitKind::None"
```

Live output:

```
        WaitKind::Park | WaitKind::None => None,
        WaitKind::None => false,
```

| | Structural (001) | Behavioural (002) |
|--|------------------|-------------------|
| Property | every repetition is counted | `None` evaluates `ready` exactly once |
| Enforced by | W1, a grep | `tests/wait_test.rs:128-144`, a count |
| Also asserted by | nothing — a test would be a timeout | `:112-126`, a 1-second stopwatch |
| Why the second instrument is weaker | — | a `None` that spins the full 1024 costs 1.0–2.4 µs |
| Loops covered | 2 (`:183`, `:121`) | 1 (`:183`) |
| Total repetition bound | `spins × 8` | 1 |
| Escapes not covered | 3 | 1 |

### Regenerate the Loop Census

```sh
cd "$(git rev-parse --show-toplevel)"

# every repetition construct in the crate, comments stripped
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "\bfor\b|\bwhile\b|\bloop\b"

# every bare `loop` in the family — note the brace is on the next line, so a
# `loop\s*{` pattern finds none of them
for d in ring_*/; do
  n=$( cat "$d"src/*.rs 2>/dev/null | grep -vE "^[[:space:]]*//" \
       | grep -cE "^[[:space:]]*loop[[:space:]]*\{?[[:space:]]*$" )
  [ "$n" -gt 0 ] && printf '%-16s %s\n' "$( basename "$d" )" "$n"
done

# the family's one deliberate *waiting* exception, and its argument
sed -n '/^\/\/! ## Why a plain spin, and not a `WaitKind`$/,/^\/\/! itself\. A `WaitKind` here would offer a `Park` that can only ever hurt, and$/p;/^  pub fn publish( &self, start : Seq, len : usize ) -> Seq$/,/^  }$/p' ring_publish/src/lib.rs
```

Live output:

```
      for _ in 0..=( attempt % 8 )
  for attempt in 0..spins.max( 1 )
ring_bench       5
ring_publish     1
//! ## Why a plain spin, and not a `WaitKind`
//!
//! [`Publisher::publish`] loops on a `spin_loop` hint with no wait strategy and
//! no budget, which everywhere else in this family would be a bug. Here it is
//! the correct shape, and the difference is what is being waited *for*.
//!
//! Waiting for space is unbounded: it depends on a consumer that may be slow,
//! stalled, or gone, so it needs a strategy and a give-up. Waiting for your
//! predecessor to publish is bounded by that producer finishing a slot write it
//! has already started and cannot abandon — it is not blocked on anything
//! itself. A `WaitKind` here would offer a `Park` that can only ever hurt, and
  pub fn publish( &self, start : Seq, len : usize ) -> Seq
  {
    loop
    {
      if let Ok( end ) = self.try_publish( start, len )
      {
        return end;
      }
      core::hint::spin_loop();
    }
  }
```

| | Count |
|--|------:|
| `for` loops in this crate | 2 |
| `while` loops in this crate | 0 |
| `loop` blocks in this crate | 0 |
| Loops whose bound is a caller's argument | 1 |
| Loops whose bound is a constant | 1 |
| Crates in the family with a bare `loop` in `src/` | 2 |
| Bare `loop` blocks family-wide | 6 — `ring_bench` 5, `ring_publish` 1 |
| Of those, loops that are **waits** | 1 (`ring_publish:202`) |

`ring_bench`'s five are drain loops, each exiting on `try_recv_batch` returning
`0` (`ring_bench/src/lib.rs:1092,1144,1176,1226,1262`). They terminate because
a ring is finite, not because a budget says so, and nothing is waiting on
anything — which is why the family's only bare `loop` that this invariant has an
opinion about is `ring_publish`'s.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT33 | `ring_wait` | n/a — doc gap | `wait_until` documents "# Panics — Never", justified by a bounded loop, on the one function whose body is a caller-supplied closure; a panicking predicate unwinds straight through and nothing tests it in this crate or the two that call it |
| WT34 | `ring_wait` | n/a — coverage | The crate's only budget sweep runs `1..8`, stopping one iteration short of the `attempt % 8` period, so the sawtooth WT5 records sits just outside the reach of the only test whose loop would have run into it |
| WT35 | `ring_wait` | n/a — coverage | The test file is `#![ cfg( not( loom ) ) ]` at file scope with no `#[ cfg( loom ) ]` counterpart, so the crate whose entire subject is what a thread does while waiting contributes no interleaving model to a family where 27 crates' tests are loom-aware |
| WT36 | family | n/a — duplication | The non-blocking guarantee is a `match` arm returning `false`, while `WaitKind::is_non_blocking` says the same thing in `ring_types`; the only occurrence of the predicate in this crate's source is a rustdoc link, and one test in the wrong crate is all that connects the two definitions |
| WT53 | `ring_wait` | n/a — unenforced | The guard asserting no unbounded loop matched nothing in 33 crates because its pattern required a brace on the keyword's own line, against a codebase that puts it on the next; the corrected pattern finds six, one of them the `ring_publish::publish` loop the same instance's next paragraph cites by line number |
