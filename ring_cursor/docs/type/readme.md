# type

The crate's exports read as contracts — what a consumer may rely on, and what
it may not — rather than as declarations.

### Overview Table

| ID | Name | Kind | Promises |
|----|------|------|----------|
| 001 | [`GATING`](001_gating.md) | `pub const Ordering` | The value is `Acquire`, and it is the ordering a gating read uses |
| 002 | [`PaddedCursor`](002_padded_cursor.md) | `pub struct` | `size_of == align_of == CACHE_LINE`; shared by reference, never copied |

`CursorPair` is deliberately absent. It is a *composition* of 002 rather than a
contract of its own — everything a caller may rely on about it follows from the
two cursors it holds, and is documented as a layout in
[`data_structure/002`](../data_structure/002_the_cursor_pair.md) rather than as a
promise here.

Both entries share a property worth stating once: **their contracts are asserted
from outside the crate.** `ring_mpsc` and `ring_spsc` each assert `GATING`'s
value in their own suites, and `ring_cursor`'s size claim is re-asserted against
`CACHE_LINE` rather than against `64`. A contract only one party checks is a
comment.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- everything this crate exports --'
command grep -E '^pub (const|struct|use|fn)' ring_cursor/src/lib.rs
```

Live output:

```
  -- everything this crate exports --
pub use ring_atomic::SeqCell;
pub const GATING : Ordering = Ordering::Acquire;
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
pub struct CursorPair
```

**Five lines are the whole export list, and only two of them declare a type this
crate owns.** One is a re-export of another crate's trait, one is a `core` enum's
value, and one is a function — so a definition named `type/` covers two structs
and documents a constant whose type came from `core`.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU45 | `GATING` | n/a — observation | The crate's one exported constant is a `core::sync::atomic::Ordering`, a type this crate does not own. It can carry no `must_use`, no newtype's protection and no invariant of its own — the decision it encodes is enforced entirely by callers choosing to name it |
| CU46 | `GATING` | **latent hazard** | Being a plain `Ordering` means any caller may substitute a different one at any load, and four of the five crates that import it also import `PaddedCursor`, whose `SeqCell` impl accepts whatever it is passed. The constant makes the right ordering easy to name; nothing makes a wrong one hard to write |
| CU47 | The export list | n/a — observation | Five lines: one `use`, one `const`, one `fn`, two `struct`. Only the two structs are types this crate defines, so a definition named `type/` documents two structures and one constant whose type came from `core` |
| CU48 | `src/lib.rs:4` | **wrong doc** | Line 4 cited `docs/workstream/008_ring_write_path.md`, which did not exist — the path was a directory. All 33 crates in the family carried the identical citation on the identical line, so the workstream pointer was broken family-wide rather than mistyped here. **Disposition: applied** — all 33 crates now cite `docs/workstream/008_ring_write_path/readme.md`, which exists |
