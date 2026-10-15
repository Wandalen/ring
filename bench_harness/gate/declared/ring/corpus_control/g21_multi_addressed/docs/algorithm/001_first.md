# Algorithm: The Fixture

### Scope

- **Purpose**: Carry a line address in a sed program of several commands, beside the two forms the conversion away from it produces.
- **Responsibility**: Prove G21 parses the program rather than matching one command's shape — the blind spot that let 148 of these live across 15 crates while `g21_line_addressed` held the control green.
- **In Scope**: Nothing real.
- **Out of Scope**: Everything real.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '1,2p;4,5p' ring_core/src/lib.rs
awk '/pub fn capacity/{ n1 = NR } n1 && NR == n1 + 1 { print }' ring_core/src/lib.rs
sed -n '1,/^\/\/!$/p' ring_core/src/lib.rs
```

Live output:

```
//! Composed ring over an SPSC, MPSC, or crossbeam backend behind one surface.
//!
//! family specified in `docs/some/family_path.md`. This is the
//! **composition point**: this family's own ruling says that feature 187's crossbeam
  {
//! Composed ring over an SPSC, MPSC, or crossbeam backend behind one surface.
//!
```
