# lifecycle

Two lifecycles meet in this crate and neither is written down inside it. A cell is
born at zero, only ever climbs, and is dropped by the compiler with no destructor
and no rewind — an assumption every gate in the family is built on, stated in a
four-word `store` contract that implies the opposite. The crate itself was born in
a task file that says it may not be started yet, and is finished.

Both are one-way. Neither has a check. The five call sites that keep every cursor
moving forward do so because five expressions happen to be written with
`advanced_by` and `next()`, and the 33 task files that describe these crates do so
because nobody has recomputed one against its crate since it was written.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_born_at_zero_climbing_until_dropped.md) | Born at Zero, Climbing Until Dropped | Every store in the family, the absent destructor, and slots recycling where sequences do not |
| [002](002_from_task_file_to_crate.md) | From Task File to Crate, With the Gate Left Behind | The readiness gate 33 tasks never crossed, and the two that tried |

## The Cell's Life Is Three Events Long

Construct — `Seq::ZERO` by `Default`, or a given value by `new`. Climb — five
production call sites, every one writing a computed-forward expression. Drop —
eight bytes, no `Drop` impl anywhere near a cell.

What the family calls a reset is `ring_shutdown::reset`, which closes, discards
forward, and reopens. The slots come back; the sequence numbers never do. A ring of
four slots run three times through gives sequences 0–11 against slots 0–3, and
that asymmetry is the reason a wrapped slot's contents are unambiguous.

## The Crate's Life Skipped Its Own Gate

The task says the Scope is "Not yet worked out" and the crate may not be claimed
until it is. All 33 implementation tasks say a version of that; all 33 sit in
`unverified/`; 29 of the crates carry over two hundred lines of source. What a task
file is for — recording why an implementation took the shape it did — is, for this
family, the section that declined to be written.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the cell: birth, the writes, the absent end --'
command grep -m1  -A3 -F '  /// crate should be pinned to.' ring_atomic/src/lib.rs | tail -n 1
command grep -rnE '\.store\( ' --include=lib.rs ring_consume/src/ ring_spsc/src/ ring_mpsc/src/ \
  | command grep -vE ':[0-9]+: *//' | command grep -v 'stamp(' | sed 's|/src/lib.rs||'
command grep -rc 'impl Drop for AtomicSeq\|impl Drop for CountingSeq' ring_atomic/src/lib.rs || true
echo '  -- the crate: where its task sits, and what its Scope says --'
find ring_atomic/task -name '*.md' | sort
command grep -c 'Not yet worked out' ring_atomic/task/unverified/102_implement_ring_atomic.md || true
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT29 | `ring_atomic` | n/a — unenforced | Five cursor writes across 33 crates and every one computes a forward value (`advanced_by`, `next()`, a clamp) — so the monotonicity the primitive refuses to enforce is supplied entirely by the shape of five expressions, and nothing checks that a sixth would match |
| AT30 | `ring_atomic` | n/a — doc gap | A cell has no destructor, no rewind, and no reuse: `ring_shutdown::reset` discards forward and leaves both cursors climbing, so slots recycle and sequences never do — the asymmetry the whole design rests on, stated nowhere in the crate that owns it, and unreadable through `ring_core`'s fourteen public methods |
| AT31 | `ring_atomic` | n/a — drift | The implementation task sits in `unverified/` with an unwritten Scope declaring the crate may not be claimed yet, against 376 lines and 17 tests — and all 33 tasks are in the same state, so every decision the family made survives only wherever its author happened to put it |
| AT32 | `ring_tls` | **wrong doc** | Of three task-file shapes, the two that worked out a Scope are the two most specifically wrong — `ring_tls`'s names `BumpLog` nineteen times as its first deliverable, for a crate that exports `TlsBuffer< T >` and contains no `BumpLog` at all |
