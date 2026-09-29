# invariant

Restrictions that must hold for this crate to be correct, and what enforces each.

### Overview Table

| ID | Name | Enforced by |
|----|------|-------------|
| 001 | [The Sequence Is Never Folded Here](001_the_sequence_is_never_folded_here.md) | A grep, one test, and a crate boundary |
| 002 | [Every Reading Is Total](002_every_reading_is_total.md) | Two saturating operations and a constructor upstream |

### The Two Are the Same Claim From Opposite Sides

001 says the crate never *loses* information: no modulo, no mask, no `SlotIndex`,
so two positions many laps apart stay distinguishable. 002 says it never
*refuses*: no `Result`, no panic, no partial function, so every pair of positions
has an answer.

Together they say a reading is always available and always meaningful. Neither
holds by construction — 001 is enforced by a crate split that a single line
could undo, and 002 leans on a non-zero guarantee established in another crate.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ22 | M1's grep | n/a — diagnostics | M1's grep is satisfied by the module doc alone; it would still pass if the folding invariant were violated in a function body |
| SQ23 | The one test that folds | n/a — observation | `positions_many_laps_apart_stay_comparable` performs the folding the crate exists to avoid in order to show what would be lost — the only place in the family a test computes the collision deliberately |
| SQ24 | Totality's source | n/a — observation | The division in `laps_between` is total only because `Capacity::new` rejects zero — a guarantee from another crate, with nothing local restating it |
| SQ25 | Return types | n/a — observation | No function returns `Result` and the crate's single `Option` encodes an absent input rather than a failure, so every reading is total in the strict sense and the type signatures say so without a word of prose |

### Regenerate

The one lint, and the unsafe count it does not govern:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '^#!\[' ring_seqno/src/lib.rs
command grep -c 'unsafe' ring_seqno/src/lib.rs || true
```

Live output:

```
#![ deny( missing_docs ) ]
0
```
