# Algorithm Doc Definition

### Scope

- **Purpose**: Record the two computations this crate performs — deciding whether two addresses fall on different cache lines, and producing a whole-line size for an arbitrary payload.
- **Responsibility**: For each, state the inputs, the steps, why the obvious alternative is wrong, and what the result is used for.
- **In Scope**: Line-index derivation by integer division; the size rounding `#[ repr( align( 64 ) ) ]` performs.
- **Out of Scope**: What a consumer does with a separated pair, which is [`ring_cursor`](../../../ring_cursor/readme.md)'s; the cost of a cache-line invalidation, which is hardware.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Deciding Line Membership by Division](001_deciding_line_membership_by_division.md) | Why the predicate divides rather than subtracts — distance is not the question, and the difference is visible at 2 bytes apart | 🔄 |
| 002 | [Rounding a Payload Up to Whole Lines](002_rounding_a_payload_up_to_whole_lines.md) | How a safe attribute produces the size guarantee, and why the guarantee is about *pairs* rather than about any single value | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two computations, as declared --'
command grep -E '^pub const fn on_distinct_lines|^  a / CACHE_LINE|^#\[ repr\( align' ring_align/src/lib.rs
echo '  -- the precondition the division rests on, asserted nowhere in this crate --'
command grep -c 'is_power_of_two' ring_align/src/lib.rs
echo '  -- control: the family does assert it, one tier down --'
command grep 'is_power_of_two' ring_types/src/capacity.rs
echo '  -- the subtraction form this definition calls wrong, in live code --'
command grep -r 'abs_diff( consume )' ring_mpsc/src/lib.rs
```

Live output:

```
  -- the two computations, as declared --
#[ repr( align( 64 ) ) ]
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
  a / CACHE_LINE != b / CACHE_LINE
  -- the precondition the division rests on, asserted nowhere in this crate --
1
  -- control: the family does assert it, one tier down --
    if !slots.is_power_of_two()
  -- the subtraction form this definition calls wrong, in live code --
    claim.abs_diff( consume ) >= 64
```

**The count is no longer zero.** A compile-time `assert!` now states
`CACHE_LINE.is_power_of_two()` directly in this crate, matching the same
predicate the control line beneath it greps for in `ring_types` — AL1 (below)
is resolved by this assertion. The pairing still matters for the reason it
always did: a grep whose target gets renamed prints nothing, which reads
identically to "still unasserted" unless the control line is checked too.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL1 | `on_distinct_lines` | **latent hazard** | Integer division equals the line index only because `CACHE_LINE` is a power of two and lines are naturally aligned; this crate now asserts it at compile time, the same technique `ring_types::Capacity::new`'s `is_power_of_two` check already applied to `slots` — the constant a port is expected to change is no longer the one left unguarded |
| AL2 | `on_distinct_lines` | n/a — observation | The subtraction form disagrees with the division form on two of the four tabulated rows — `(63, 64)` and `(100, 130)` — and both errors run in the conservative direction, reporting "not separated" for addresses that are genuinely on different lines, which is the harder error to notice because it fails toward padding something that did not need it |
| AL3 | `tests/align_test.rs` | n/a — observation | `two_wrapped_fields_land_on_different_lines` asserts both forms, and the redundancy is load-bearing in one direction only: the subtraction assertion is the *stronger* claim there, since two fields one byte apart across a boundary satisfy the exported predicate and would mean the padding had failed |
| AL4 | `ring_mpsc/src/lib.rs`'s `claim.abs_diff( consume ) >= 64` | **latent hazard** | `claim.abs_diff( consume ) >= 64` is the exact form this definition documents as wrong, written with a literal instead of `CACHE_LINE`, in the family's highest-traffic crate — so the one live use of the rejected arithmetic is also the one site a port cannot detect |
