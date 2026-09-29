# Algorithm: The Fixture

### Scope

- **Purpose**: Carry a line address in the awk spelling, beside two forms that are not one.
- **Responsibility**: Prove G21 reads the rule, not the syntax it was first written for.
- **In Scope**: Nothing real.
- **Out of Scope**: Everything real.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
awk 'NR >= 1 && NR <= 2' ring_core/src/lib.rs
awk '/pub fn capacity/{ m = NR } m && NR == m + 1' ring_core/src/lib.rs
printf 'a\nb\nc\n' | awk 'NR == 1'
```

Live output:

```
//! Composed ring over an SPSC, MPSC, or crossbeam backend behind one surface.
//!
  {
a
```
