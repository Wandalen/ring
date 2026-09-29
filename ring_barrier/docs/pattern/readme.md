# pattern

Two patterns, both family-wide rather than local: one about who owns the
cursors a type reasons about, one about the ladder every gating type climbs and
where the top rung stops being shared.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Borrowed View and the Owned Set](001_the_borrowed_view_and_the_owned_set.md) | The ownership rule — the type that creates the cursors owns them — and the failing test that produced it |
| 002 | [The Quantity, the Predicate, and the Wait](002_the_quantity_the_predicate_and_the_wait.md) | The three-rung API ladder, the four third rungs the family shipped, and the caller census over all of them |

### `ring_gating` and `ring_barrier`, Side by Side

| | `ring_gating` — producer | `ring_barrier` — consumer |
|--|--------------------------|---------------------------|
| Ownership | owns a `Vec< PaddedCursor >` | borrows a `&'a [ PaddedCursor ]` |
| Quantity rung | `headroom( producer ) -> usize` | `available( from ) -> u64` |
| Predicate rung | `admits( producer, count ) -> bool` | `admits( from, count ) -> bool` |
| Third rung | a **reason** — `check` → `Result< () >` | a **wait** — `wait_for` → `Result< Seq >` |
| Fails with | `RingError::Full` | `RingError::Empty` |
| Capacity in the arithmetic | ✔ | ✘ — [`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md) |

The rows agree until the last three, and the last three all follow from one
thing: a producer out of room needs to be told why, a consumer out of data needs
to be told when.

### Counted

Scoped to the three gating types — `CursorPair`, `GatingSet`, `Barrier`.
`ring_seqno`'s same-named free functions (`may_claim:68`, `free_slots:90`,
`pending:107`) are the arithmetic layer underneath all three, not a fourth
instance of the ladder, and are excluded:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rn 'pub fn frontier' ring_*/src/*.rs | wc -l   # 1 — this crate's
grep -rn 'pub fn available' ring_*/src/*.rs | wc -l  # 5, incl. available_up_to
```

| Rung | Methods on the three types | Reached from any `src/` |
|------|---------------------------|-------------------------|
| Quantity | `free_slots`, `pending`, `headroom`, `available` | ✔ — all but `free_slots` |
| Predicate | `may_claim`, `admits` ×2 | ✔ — `may_claim` ×2, `Barrier::admits` ×1 |
| **Third** | `for_space`, `for_data`, `check`, `wait_for` | **✘ — none, in any of the four** |

Per-method caller counts, with the command that regenerates them, are in
[002](002_the_quantity_the_predicate_and_the_wait.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR11 | family | n/a — inconsistency | `available` names 4 methods family-wide with 3 return types (`u64`, `Available`, `usize` ×2); `admits` names 3; `frontier` names exactly 1 |
| BR20 | family | n/a — coverage | No third rung in the family has a caller in any `src/`: `ring_wait::for_space`, `ring_wait::for_data`, `GatingSet::check` and `Barrier::wait_for` have 24 callers between them, every one a test of the rung itself |
| BR44 | family | n/a — observation | `ring_gating` and `ring_barrier` share no trait, no return type and no signature either could pass to the other — `available` returns `u64`, `headroom` returns `usize` — and the only common code is the fold that lives in neither of them |
| BR45 | family | n/a — inconsistency | Quantity, predicate, wait is written four times across the family with the steps named differently each time; `admits` is the one name two crates share and its two signatures take different parameters for different questions |
