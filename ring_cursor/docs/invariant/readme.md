# invariant

The two standing restrictions on this crate — one the test suite enforces, one
only a human reading the source can.

### Overview Table

| ID | Name | Restriction | Enforced by |
|----|------|-------------|-------------|
| 001 | [One Cursor, One Line](001_one_cursor_one_line.md) | A cursor occupies exactly one cache line, and two in a struct occupy two | `tests/cursor_test.rs` — five tests, three clauses |
| 002 | [The Number 64 Never Appears Here](002_the_number_64_never_appears_here.md) | The alignment is inherited from `ring_align`, never restated | **Nothing automatic.** `tests/manual/readme.md` M1, read by hand |

**The gap between them is the crate's real risk.** 001 fails loudly and
immediately. 002 cannot fail at all: the obvious reimplementation —
`#[ repr( align( 64 ) ) ]` written directly on `PaddedCursor` — satisfies every
clause of 001, passes the whole suite, and gives the family a second independent
cache-line constant. The manual plan's own closing note names M1 as the check
worth keeping for exactly that reason.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the alignment, inherited rather than restated --'
command grep 'CacheAligned< AtomicSeq >' ring_cursor/src/lib.rs
echo '  -- non-doc lines here carrying the literal --'
command grep -cE '^[^/]*\b64\b' ring_cursor/src/lib.rs
echo '  -- control: the same count where it is genuinely written --'
command grep -cE '^[^/]*\b64\b' ring_align/src/lib.rs
```

Live output:

```
  -- the alignment, inherited rather than restated --
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
  -- non-doc lines here carrying the literal --
0
  -- control: the same count where it is genuinely written --
2
```

**Zero here, and a non-zero next door.** A count that must be zero says nothing
on its own — a renamed pattern prints zero just as convincingly — so the control
runs the identical expression against the file that does declare the number.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU21 | The three clauses | n/a — observation | Two tests out of twenty-two carry all three clauses. The whole-file run is quoted rather than a filtered one so that ratio is visible: the invariant this crate exists for is one-eleventh of what its suite spends its time on |
| CU22 | Clause 2 | **latent hazard** | `size_of == CACHE_LINE` is satisfied just as well by a type carrying `#[ repr( align( 64 ) ) ]` directly as by one inheriting it. Only clause 3, which reads two real addresses, distinguishes an inherited alignment from a restated one — and it is the clause a change advertised as "one fewer dependency" is least likely to keep |
| CU23 | The restriction's own check | n/a — diagnostics | The grep filters out lines beginning `///` or `//!`, so it cannot see the three `64` literals at `src/lib.rs:140`, `:141` and `:184`. Those are doctests — compiled and executed by `cargo test --doc` — so the check is blind to the one place in this file where a literal both runs and would keep running after a fork |
| CU24 | U1 | n/a — unenforced | The only instrument that sees this invariant directly is `tests/manual/readme.md` M1, a human running a grep. The recipe published here is the same grep, and it is wired to nothing that fails — `recipes.py` checks that it still prints what it says, not that what it prints is empty |
