# decisions

Choices this crate made that had a live alternative, with what would reopen each.

### Overview Table

| ID | Name | Chose | Alternative still live? |
|----|------|-------|-------------------------|
| 001 | [Gating Is Fixed, Not a Parameter](001_gating_is_fixed_not_a_parameter.md) | One named `Acquire`, not an argument | The *placement* is — the family already states the same decision four times |
| 002 | [The Capacity Is Held by the Pair](002_the_capacity_is_held_by_the_pair.md) | A third field on `CursorPair` | Yes, for the multi-cursor case, which took the other answer |

**Both decisions are about where a fact lives rather than what it is.** Nobody
disputes that a gating read is `Acquire` or that a reading needs a capacity; the
choices were whether to name them once and where. 001 is the one that did not
hold — its measurement is the crate's most consequential finding, and it is
recorded rather than fixed.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the ordering this crate fixes --'
command grep '^pub const GATING' ring_cursor/src/lib.rs
echo '  -- the readings that use it, none of which take it as a parameter --'
command grep -E '^  pub fn (free_slots|pending|may_claim)' ring_cursor/src/lib.rs
echo '  -- control: the tier below, which refuses to choose --'
command grep -E '^  fn load\( &self, order' ring_atomic/src/lib.rs
```

Live output:

```
  -- the ordering this crate fixes --
pub const GATING : Ordering = Ordering::Acquire;
  -- the readings that use it, none of which take it as a parameter --
  pub fn free_slots( &self ) -> usize
  pub fn pending( &self ) -> u64
  pub fn may_claim( &self ) -> bool
  -- control: the tier below, which refuses to choose --
  fn load( &self, order : Ordering ) -> Seq;
  fn load( &self, order : Ordering ) -> Seq
  fn load( &self, order : Ordering ) -> Seq
```

**The control is what makes the first two arms a decision rather than an
oversight.** `ring_atomic` takes `order` on every load; this crate's three
readings take none. Both crates are consistent with themselves and they disagree
with each other on purpose, which is the whole of [`001`](001_gating_is_fixed_not_a_parameter.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU13 | `GATING`'s doc comment | **wrong doc** | "Named rather than written inline at three call sites" undercounts twice over: four functions read the constant — `slowest`, `free_slots`, `pending`, `may_claim` — and because three of them read both cursors, seven individual loads name it. The figure was right when one reading existed |
| CU14 | `GATING`'s doc comment | **wrong doc** | The argument for `pub` named `ring_gating` and `ring_barrier` as the crates that read a cursor set to decide safety. Neither imports `GATING`. The five that do are `ring_claim`, `ring_consume`, `ring_mpsc`, `ring_publish` and `ring_spsc`, and none of them appeared in the justification for the item they depend on. **Disposition: applied** — doc comment now names the five crates that actually import it |
| CU15 | `Capacity` in the pair | n/a — observation | The decision is argued from correctness alone — "a caller supplying it per call could supply a different one each time" — and its cost is a whole cache line, 64 bytes to carry 8. Both halves are true; only the first is in the doc comment, so a reader of the source sees the reason and not the price |
| CU16 | The 192-byte size | n/a — diagnostics | No assertion states the size. `tests/cursor_test.rs:419-420` asserts a bound and the fourteen doctests assert behaviour, so the number this decision turns on is measured by reading a table in this document rather than by anything that runs |
