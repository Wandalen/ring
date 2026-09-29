# Item: Two Ordering Constants Where the Sibling Has Five

### Scope

- **Purpose**: Record the two published `Ordering` constants and why the vocabulary is smaller here than in the multi-producer sibling.
- **Responsibility**: `OWN` and `HANDOFF` — their values, their doc tests, and the three names that have no counterpart.
- **In Scope**: The constants as items, and the accesses that use them.
- **Out of Scope**: The happens-before argument they serve (→ [`../invariant/002`](../invariant/002_no_lock_in_the_path.md)); the stamp protocol that needs the other three (→ `ring_mpsc`'s `item/002`).

### The Two

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
grep -E '^pub const [A-Z]' src/lib.rs
printf 'GATING is imported, not declared:  '; grep -c 'use ring_cursor::.*GATING' src/lib.rs
```

Live output:

```
pub const OWN : Ordering = Ordering::Relaxed;
pub const HANDOFF : Ordering = Ordering::Release;
GATING is imported, not declared:  1
```

| Constant | Value | Names |
|----------|-------|-------|
| `OWN` | `Ordering::Relaxed` | An end reading its own cursor, which no other thread writes |
| `HANDOFF` | `Ordering::Release` | An end publishing its cursor to the other end |

A third ordering is in use and is **not** declared here: `GATING`, imported from
`ring_cursor`. Every cross-end load in this file carries it. So the crate's
ordering vocabulary is two published names plus one borrowed one, and only the
two published names are pinned by a test in this crate.

### What the Sibling Has and This Crate Does Not

`ring_mpsc` publishes five: `PUBLISH`, `OBSERVE`, `COMMIT`, `OWN`, `UNSTAMPED`.
Three have no counterpart here, and the reason is the same in each case — they
order accesses to a per-slot stamp array this crate does not have
(→ [`../data_structure/002`](../data_structure/002_the_absent_stamp_array.md)).
`OWN` is the one name both crates publish, with the same value and the same
meaning.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | Declares `OWN` and `HANDOFF`; imports `GATING` |
| `tests/spsc_test.rs` | `the_two_orderings_are_the_ones_the_design_names` pins both values |
| `../../../ring_cursor/src/lib.rs` | Declares `GATING`, the third ordering this file uses |

### SP29 — Both Constants Are Pinned by One Test, Not by Doc Tests

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'ring_mpsc doc-test asserts:  '; grep -c 'assert_eq!( ring_mpsc::' ring_mpsc/src/lib.rs
printf 'ring_spsc doc-test asserts:  '; grep -c 'assert_eq!( ring_spsc::' ring_spsc/src/lib.rs
printf 'ring_spsc test:              '; grep -oE 'fn the_two_orderings[a-z_]*' ring_spsc/tests/spsc_test.rs
```

Live output:

```
ring_mpsc doc-test asserts:  5
ring_spsc doc-test asserts:  2
ring_spsc test:              fn the_two_orderings_are_the_ones_the_design_names
```

Two mechanisms for the same guarantee. The doc-test form documents at the
declaration and runs under `cargo test --doc`; the integration-test form runs
under the ordinary suite and keeps the two values side by side where the pairing
is legible.

Neither is wrong. Recorded because the family has two conventions for pinning the
same kind of constant, and nothing chooses between them.

### SP30 — The Vocabulary Is Two Published Names and One Borrowed One

A reader looking at `pub const OWN` and `pub const HANDOFF` sees the crate's
ordering discipline in two lines and will conclude those are the two. The file's
actual cross-end loads carry a third name imported from `ring_cursor`.

**The fix is not obviously to republish `GATING` here** — that would give the
family two names for one constant. It is recorded as an inconsistency because the
published pair reads as exhaustive and is not.
