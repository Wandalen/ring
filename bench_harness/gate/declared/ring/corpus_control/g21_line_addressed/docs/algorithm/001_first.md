# Algorithm: The Fixture

### Scope

- **Purpose**: Carry one line-addressed extraction, and one pager that is not one.
- **Responsibility**: Prove G21 fires on the first and stays silent on the second.
- **In Scope**: Nothing real.
- **Out of Scope**: Everything real.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '1,2p' ring_core/src/lib.rs
printf 'a\nb\nc\n' | sed -n '1,2p'
```

Live output:

```
//! Composed ring over an SPSC, MPSC, or crossbeam backend behind one surface.
//!
a
b
```
