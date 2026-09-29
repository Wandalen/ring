# Invariant Doc Definition

### Scope

- **Purpose**: State the two standing restrictions this crate imposes — one on any struct holding two padded fields, one on the other 32 crates of the family.
- **Responsibility**: For each, state the restriction, name what enforces it, and record what a violation costs.
- **In Scope**: The pair-separation property; single ownership of the cache-line number across the family.
- **Out of Scope**: Correctness of the number itself, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md); ordering or visibility of the values inside padded cursors, which is [`ring_atomic`](../../../ring_atomic/readme.md)'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Two Wrapped Fields Never Share a Line](001_two_wrapped_fields_never_share_a_line.md) | The property the crate exists to deliver, stated over a pair rather than over the wrapper — and the caller obligations that remain | 🔄 |
| 002 | [One Constant for the Whole Family](002_one_constant_for_the_whole_family.md) | A restriction on 32 other crates that this crate cannot enforce, and which one of them currently violates | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the restriction: one declaration of the line size, family-wide --'
command grep -r 'pub const CACHE_LINE' ring_*/src/*.rs
echo '  -- every bare 64 standing in for it, across the family --'
command grep -r '>= 64\|% 64\|== 64' ring_*/src/*.rs
```

Live output:

```
  -- the restriction: one declaration of the line size, family-wide --
ring_align/src/lib.rs:pub const CACHE_LINE : usize = 64;
  -- every bare 64 standing in for it, across the family --
ring_cursor/src/lib.rs:  /// assert_eq!( cursor.addr() % 64, 0, "a 64-aligned value starts on a line boundary" );
ring_mpsc/src/lib.rs:    claim.abs_diff( consume ) >= 64
```

The first grep is the invariant and the second is its violations, in one block
deliberately: a run that printed one line and then nothing would mean the
restriction holds, and a run that printed neither would mean the patterns had
rotted. Both halves must print for either to be read.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL17 | The family | **latent hazard** | The invariant is "one declaration for 33 crates" and the family currently holds three numbers: the declaration at `ring_align/src/lib.rs:36`, plus literals at `ring_cursor/src/lib.rs:184` and `ring_mpsc/src/lib.rs:865`. `ring_cursor` proves the constant was reachable at both — `tests/cursor_test.rs:42` imports it, while the doctest seventeen lines into the same crate's `src/` writes `% 64` |
| AL18 | `ring_mpsc/src/lib.rs:865` | **latent hazard** | The costlier of the two: it is in running code rather than a doctest, and it is the *subtraction* form, so on a 128-byte-line port it keeps passing, keeps compiling, and quietly asserts a separation half the size it claims |
| AL19 | This crate's enforcement | n/a — unenforced | `ring_align` can declare the constant and cannot restrict the other 32 crates from spelling it out — no gate in `bench_harness/gate/` greps for a bare 64 in the family, so the invariant is a statement about intent that the two violations above already contradict |
| AL20 | Two wrapped fields | n/a — observation | The first invariant is stated over a *pair* and not over the wrapper, which is the only formulation that is true: a single `CacheAligned` has no line to be separate from, and the property comes into existence when a second one is placed beside it |
