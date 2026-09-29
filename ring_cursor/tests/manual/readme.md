# ring_cursor — manual testing plan

`tests/cursor_test.rs` asserts the three clauses
`docs/feature/169_padded_cursor.md` is graded on, and they are unusually
well-suited to automation: `size_of`, `align_of`, and two addresses are exact
integers, so there is no judgement left in them.

What the tests cannot reach is everything *around* those numbers. A `64` that
is a magic literal rather than `ring_align::CACHE_LINE` passes every assertion
and silently forks the family's padding decision in two. A `CursorPair` that
gained a third field would still measure two lines and still pass. And the
gating readings fix `Acquire` internally — a decision a caller cannot see, and
therefore one that has to be argued in the source or it is just a default with
better manners.

Each check below is a source reading for that reason. Run from the workspace
root.

## M1 — the padding comes from `ring_align`, not from a literal 64

The number 64 appearing anywhere in this crate's own source is the failure: it
would mean the family has two independent statements of its cache-line size,
and a future port that raises `ring_align::CACHE_LINE` to 128 would move one
and not the other.

```bash
grep -nE "\b64\b" ring_cursor/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** no output. Every 64 in the file is in prose or a doc example
(where a literal is the assertion, not a definition). The type itself gets its
alignment from `ring_align::CacheAligned`'s own attribute and never restates
it.

## M2 — the cursor carries no state beyond its sequence

The first test asserts `size_of == 64`. A field added inside the existing
padding would not change that number — 8 bytes of sequence plus 8 bytes of
something else still rounds to one line — so the size assertion cannot see it.

```bash
grep -n -A 3 "pub struct PaddedCursor" ring_cursor/src/lib.rs
```

**Expected:** a single-field tuple struct wrapping `CacheAligned<AtomicSeq>`.
The padding is meant to be empty; a field placed in it is state that escaped
the crate that should own it, hidden in space that measures the same either
way.

## M3 — the fixed `Acquire` is named once and argued

```bash
grep -n -B 6 "^pub const GATING" ring_cursor/src/lib.rs
grep -n "Ordering::Acquire" ring_cursor/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
grep -n -A 10 "Which orderings this crate names and which it fixes" ring_cursor/src/lib.rs
```

**Expected:** exactly one line — `const GATING`'s own definition. The three
gating readings must refer to it rather than writing `Ordering::Acquire`
inline, so any second hit in code is a fork. The module documentation must
say *why* this crate fixes an ordering while `ring_atomic` refuses to — the two
look contradictory and a reader who meets only one of them will conclude the
other is a bug.

This is the check `ring_trace`'s M2 taught: five call sites with three policies
was invisible to every test, and the only instrument that found it was reading
the sites next to each other.

## M4 — the pair's readings all consult both cursors

A reading that consulted only the producer gives the right answer whenever the
consumer is at zero — which is exactly the state most tests start in.

```bash
sed -n '/  pub fn free_slots/,/^  }/p;/  pub fn pending/,/^  }/p;/  pub fn may_claim/,/^  }/p' \
  ring_cursor/src/lib.rs | grep -oE "self\.(producer|consumer)\.load" | wc -l
```

`grep -o | wc -l` rather than `grep -c`, because each body puts both loads on
one line and `-c` counts matching lines.

**Expected:** `6` — three readings, two loads each. `pending` is the one to
look at twice: it takes no capacity, so a reader may assume it takes fewer
cursors too.

## M5 — the readings delegate to `ring_seqno` rather than reimplementing it

The lap boundary is an off-by-one that `docs/feature/178_sequence_barrier_and_gating_set.md`
names explicitly. It is already decided, once, in `ring_seqno`. A second
statement of it here would be a second place to get it wrong.

```bash
grep -nE "ring_seqno::" ring_cursor/src/lib.rs
grep -nE "capacity\.get\(\)|saturating_sub|< self\.capacity" ring_cursor/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** three `ring_seqno::` calls, and no arithmetic of this crate's own
on the capacity. The second command finding anything means the boundary has
been restated locally.

## M6 — every declared dependency is actually used

Four dependencies are declared in `Cargo.toml`. An unused one is not merely
untidy: it is a claim about the crate's position in the family's dependency
forest that the source does not support, and `cargo +nightly udeps` only runs
at verification level 4.

```bash
comm -23 \
  <( grep -E "^ring_" ring_cursor/Cargo.toml | cut -d' ' -f1 | sort ) \
  <( grep -vE "^[[:space:]]*(///|//!)" ring_cursor/src/lib.rs \
       | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output — every declared dependency appears in code.

Note the `grep -v` on doc lines, and note that it is doing real work rather
than being defensive: this crate's module documentation names `ring_batch` (to
contrast its ordering decision) and its doc examples name `ring_cursor` itself.
A scan that reads prose reports six crates used where four are declared, which
inverts the check — it would answer "declared but unused" with a list of
crates that are *used but not declared*, and both readings look like a
violation.

## M7 — the doc examples are the API's first reader

```bash
cargo test -p ring_cursor --doc
```

**Expected:** every example passes, and `PaddedCursor`'s own example shows the
size and alignment together — those two numbers are the feature, and an example
that showed only the load would be showing `ring_atomic`'s feature instead.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | No output. The only `64`s in the file are in prose and in `PaddedCursor`'s doc example, where the literal is the assertion. |
| 2026-08-28 | M2 | ✅ | `pub struct PaddedCursor( CacheAligned< AtomicSeq > );` — one field, no room for hidden state to be honest about. |
| 2026-08-28 | M3 | ✅ *(check corrected)* | First run counted 4 `Ordering::Acquire` against an expected 1. Three were in doc examples, where naming the ordering is the demonstration. `grep -c` over the whole file cannot tell code from prose; the command now drops doc lines and reports the single `const GATING` line. |
| 2026-08-28 | M4 | ✅ *(check corrected)* | First run reported 3 against an expected 6. `grep -c` counts matching *lines*, and each of the three bodies puts both loads on one line. Switched to `grep -o \| wc -l`; reads 6. |
| 2026-08-28 | M5 | ✅ | Three `ring_seqno::` calls (`free_slots`, `pending`, `may_claim`) at lines 267, 286, 316; second command silent, so no local restatement of the lap boundary. |
| 2026-08-28 | M6 | ✅ *(check corrected)* | First run listed six crates "used" against four declared — `ring_batch` from the module doc's ordering contrast, `ring_cursor` from its own doc examples. Rewritten as a `comm -23` of declared-minus-used over code lines only; empty output. |
| 2026-08-28 | M7 | ✅ | 12 doc tests pass. |
| 2026-08-29 | M7 | ✅ | Re-run: 14 pass, 0 fail, in 0.56s. Two more than the 08-28 run — the crate gained doc examples in between, and both new ones pass. |

**Three of seven checks were wrong on first run, all in the same way**, and the
pattern is now consistent enough across this workstream to name: a grep over a
Rust file reads documentation as if it were code. Every one of the three had a
plausible expected value, produced a number, and the number meant something
other than what it was being read as — 4 orderings that were 1, 3 loads that
were 6, 6 dependencies that were 4. None of them would have failed loudly; each
would have been recorded as a passing check of something that was never
checked.

The rule that falls out: **a check whose subject is code must exclude `///` and
`//!` lines before counting anything.** ring_event's M2 and ring_atomic's M1
were the same defect in earlier crates.

M1 is the check worth keeping for the crate rather than for the method. It is
the one this crate would most likely fail later rather than now: writing
`#[ repr( align( 64 ) ) ]` directly on `PaddedCursor` is the obvious
implementation — shorter, one fewer dependency, and it passes all three clauses
of the reached-test. It is also how the family acquires a second, independent
cache-line constant, which is the exact failure `ring_align::CACHE_LINE`'s own
documentation warns about for a 128-byte target. The test suite would notice
nothing.
