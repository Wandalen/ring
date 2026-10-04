# ring_cursor manual testing plan

`tests/cursor_test.rs` asserts the cursor's layout through `size_of`,
`align_of` and two addresses. These are exact integers, so there is no
judgement left in them, and they suit automation unusually well.

What the tests cannot reach is everything *around* those numbers. A `64` that
is a magic literal rather than `ring_align::CACHE_LINE` passes every assertion
and silently forks the family's padding decision in two. A `CursorPair` that
gained a field in its padding would still measure the same and still pass. And the
gating readings fix `Acquire` internally. That is a decision a caller cannot see,
and therefore one that has to be argued in the source or it is just a default
with better manners.

Each check below is a source reading for that reason. A check whose subject is
code must exclude `///` and `//!` lines before counting anything, because a
grep over a Rust file otherwise reads documentation as code. Run from the
workspace root.

## M1. The padding comes from `ring_align`, not from a literal line size

A literal line size (64 or 128) appearing anywhere in this crate's own source
is the failure. It would mean the family has two independent statements of its
cache-line size, and a port that raises `ring_align::CACHE_LINE`, as the move
from 64 to 128 did, would change one and not the other.

This is the check the crate is most likely to fail later. Writing
`#[ repr( align( 128 ) ) ]` directly on `PaddedCursor` is the obvious
implementation. It is shorter, has one fewer dependency, and passes every
layout test. The test suite would notice nothing.

```bash
grep -nE "\b(64|128)\b" ring_cursor/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** no output. Every 64 or 128 in the file is in prose. The doc
examples compare against `ring_align::CACHE_LINE` too. The type itself gets its
alignment from `ring_align::CacheAligned`'s own attribute and never restates
it.

## M2. The cursor carries no state beyond its sequence

The first test asserts `size_of == 128`. A field added inside the existing
padding would not change that number, because 8 bytes of sequence plus 8 bytes
of something else still rounds to one line. The size assertion cannot see it.

```bash
grep -n -A 3 "pub struct PaddedCursor" ring_cursor/src/lib.rs
```

**Expected:** a single-field tuple struct wrapping `CacheAligned<AtomicSeq>`.
The padding is meant to be empty; a field placed in it is state that escaped
the crate that should own it, hidden in space that measures the same either
way.

## M3. The fixed `Acquire` is named once and argued

```bash
grep -n -B 6 "^pub const GATING" ring_cursor/src/lib.rs
grep -n "Ordering::Acquire" ring_cursor/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
grep -n -A 10 "Which orderings this crate names and which it fixes" ring_cursor/src/lib.rs
```

**Expected:** exactly one line, `const GATING`'s own definition. The three
gating readings must refer to it rather than writing `Ordering::Acquire`
inline, so any second hit in code is a fork. The module documentation must
say *why* this crate fixes an ordering while `ring_atomic` refuses to. The two
look contradictory, and a reader who meets only one of them will conclude the
other is a bug.

## M4. The pair's readings all consult both cursors

A reading that consulted only the producer gives the right answer whenever the
consumer is at zero, which is the state most tests start in.

```bash
sed -n '/  pub fn free_slots/,/^  }/p;/  pub fn pending/,/^  }/p;/  pub fn may_claim/,/^  }/p' \
  ring_cursor/src/lib.rs | grep -oE "self\.(producer|consumer)\.load" | wc -l
```

`grep -o | wc -l` rather than `grep -c`, because each body puts both loads on
one line and `-c` counts matching lines.

**Expected:** `6`, from three readings with two loads each. `pending` is the one
to look at twice. It takes no capacity, so a reader may assume it takes fewer
cursors too.

## M5. The readings delegate to `ring_seqno` rather than reimplementing it

The lap boundary is an off-by-one. It is already decided, once, in
`ring_seqno`. A second statement of it here would be a second place to get it
wrong.

```bash
grep -nE "ring_seqno::" ring_cursor/src/lib.rs
grep -nE "capacity\.get\(\)|saturating_sub|< self\.capacity" ring_cursor/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** one `ring_seqno::` call in each of the three readings, and no
arithmetic of this crate's own on the capacity. The second command finding
anything means the boundary has been restated locally.

## M6. Every declared dependency is used

An unused dependency is more than untidy. It is a claim about the crate's
position in the family's dependency forest that the source does not support.

```bash
comm -23 \
  <( grep -E "^ring_" ring_cursor/Cargo.toml | cut -d' ' -f1 | sort ) \
  <( grep -vE "^[[:space:]]*(///|//!)" ring_cursor/src/lib.rs \
       | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output. Every declared dependency appears in code.

Note the `grep -v` on doc lines. It does real work here, beyond being
defensive. This crate's module documentation names `ring_batch` (to
contrast its ordering decision) and its doc examples name `ring_cursor` itself.
A scan that reads prose reports crates as used that are not declared, which
inverts the check. It would answer "declared but unused" with a list of
crates that are *used but not declared*, and both readings look like a
violation.

## M7. The doc examples are the API's first reader

```bash
cargo test -p ring_cursor --doc
```

**Expected:** every example passes, and `PaddedCursor`'s own example shows the
size and alignment together. Those two numbers are the feature, and an example
that showed only the load would be showing `ring_atomic`'s feature instead.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | No output. The only `64`s in the file are in prose and in `PaddedCursor`'s doc example, where the literal is the assertion. |
| 2026-08-28 | M2 | ✅ | `pub struct PaddedCursor( CacheAligned< AtomicSeq > );` has one field, leaving no room for hidden state to be honest about. |
| 2026-08-28 | M3 | ✅ *(check corrected)* | First run counted 4 `Ordering::Acquire` against an expected 1. Three were in doc examples, where naming the ordering is the demonstration. `grep -c` over the whole file cannot tell code from prose; the command now drops doc lines and reports the single `const GATING` line. |
| 2026-08-28 | M4 | ✅ *(check corrected)* | First run reported 3 against an expected 6. `grep -c` counts matching *lines*, and each of the three bodies puts both loads on one line. Switched to `grep -o \| wc -l`; reads 6. |
| 2026-08-28 | M5 | ✅ | Three `ring_seqno::` calls (`free_slots`, `pending`, `may_claim`) at lines 267, 286, 316; second command silent, so no local restatement of the lap boundary. |
| 2026-08-28 | M6 | ✅ *(check corrected)* | First run listed six crates "used" against four declared: `ring_batch` from the module doc's ordering contrast, `ring_cursor` from its own doc examples. Rewritten as a `comm -23` of declared-minus-used over code lines only; empty output. |
| 2026-08-28 | M7 | ✅ | 12 doc tests pass. |
| 2026-08-29 | M7 | ✅ | Re-run: 14 pass, 0 fail, in 0.56s. Two more than the 08-28 run, because the crate gained doc examples in between; both new ones pass. |
