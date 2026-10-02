# ring_trace manual testing plan

`tests/trace_test.rs` asserts the two numbers
`docs/feature/185_ring_stats.md` names for this crate: one entry per operation
when enabled, zero when not. A test can count entries. It cannot check what
matters about the disabled path, that it **returns before touching the lock**.
A disabled trace that took and released the mutex on every call would record
zero entries and pass every assertion, while putting a contended lock on the
exact path this family exists to measure.

That is a source property, so this plan reads the source.

Run from the workspace root.

## M1. The disabled path returns before the lock

```bash
sed -n '/  pub fn record(/,/^  }/p' ring_trace/src/lib.rs
```

**Expected:** `if !self.enabled { return; }` as the *first* statement, before
any mutex access. A trace switched off must cost a predictable-branch read of a
`bool` and nothing more.

## M2. Every lock access goes through one place

Four read methods and one write method all touch the same mutex. If each
handles a poisoned lock its own way, they will disagree about what a poisoned
trace means, and the disagreement will surface as a diagnostic that lies.

```bash
grep -nE "\.lock\(\)|entries_guard" ring_trace/src/lib.rs
```

**Expected:** exactly one `.lock()` call in the whole file, inside
`entries_guard`, and every other site calling `entries_guard()`.

## M3. Poisoning is recovered, and the choice is argued

```bash
grep -n -B 4 -A 4 "unwrap_or_else" ring_trace/src/lib.rs
grep -n -A 10 "Every access recovers from poisoning" ring_trace/src/lib.rs
```

**Expected:** `PoisonError::into_inner` recovery, and a written argument for why
recovery is right *here*. The argument is that a `Vec<TraceEntry>` has no
invariant a panic could half-establish, so there is nothing for poisoning to
protect.

Note the reachability limit, which is the reason this is a manual check and not
a test. No public method can poison the lock, because every guard is held across
code that cannot unwind. The recovery is a defensive property, verified by
reading, and a test asserting it would have to reach into the private mutex to
create the condition.

## M4. The enabled flag cannot be flipped after construction

A trace switched on mid-run produces a log with a silent hole at the front,
which reads exactly like a run where nothing happened early.

```bash
grep -nE "enabled" ring_trace/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** `enabled` is set only in `enabled()` and `disabled()`, read in
`record` and `is_enabled`, and never assigned anywhere else. No `set_enabled`,
no `&mut self` toggle, no `AtomicBool`.

## M5. The default is off

```bash
grep -n -A 6 "impl Default for Trace" ring_trace/src/lib.rs
```

**Expected:** `Self::disabled()`, with a doc line saying why. A ring nobody
asked to trace must not be tracing.

## M6. A batch is one entry, and the reason is written down

```bash
grep -n -B 8 "pub struct TraceEntry" ring_trace/src/lib.rs
```

**Expected:** `count` is documented as the thing that keeps a batch claim one
entry rather than 64, because expanding it would contradict feature 177's own
claim that it *was* one operation.

## M7. The doc examples are the API's first reader

```bash
cargo test -p ring_trace --doc
```

**Expected:** every example passes; the `Trace` example shows the enabled and
disabled cases side by side, since the pair is the feature.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | `if !self.enabled { return; }` is the first statement; the lock is reached only after it. |
| 2026-08-28 | M2 | ✅ *(after fix)* | First run found **five** independent lock sites with three different poisoning policies: `record` used `.expect()` (panics), `len`/`count_of` used `map_or( 0, .. )` (silently reports zero), `entries` used `unwrap_or_default()`, `clear` used `if let Ok`. Consolidated into a single private `entries_guard`; now one `.lock()` at line 244 and five call sites. |
| 2026-08-28 | M3 | ✅ *(after fix)* | `unwrap_or_else( std::sync::PoisonError::into_inner )`, with the argument in the module's "Why a `Mutex` is the right cost here" section naming both rejected alternatives. |
| 2026-08-28 | M4 | ✅ | `enabled` assigned only in the two constructors, read in `record` and `is_enabled`. No setter, no atomic. |
| 2026-08-28 | M5 | ✅ | `Self::disabled()`, documented as "the state a ring that was never asked to trace must be in". |
| 2026-08-28 | M6 | ✅ | `count`'s field doc and the struct doc both state it; the struct doc names feature 177 explicitly. |
| 2026-08-28 | M7 | ✅ | 9 doc tests pass. |

M2 is the check that earned this plan, and the defect it found is worse than it
looks in the table. A poisoned trace would have **panicked the producer thread**
on the write path, a diagnostic killing the thread it was added to observe. At
the same time it would have reported `0` entries to whoever read it, making
"nothing happened" and "the log broke" indistinguishable. Every test in `trace_test.rs`
passed throughout, because no test can poison the lock. The only instrument that
finds this is a person reading five call sites next to each other.
