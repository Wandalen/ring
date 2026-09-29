# Algorithm: The Fixture

### Scope

- **Purpose**: Carry only content-anchored extractions, and one call that never numbers a line at all.
- **Responsibility**: Prove G21 reports zero problems when nothing in scope addresses source by line.
- **In Scope**: Nothing real.
- **Out of Scope**: Everything real.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/pub fn capacity( &self )/,/^  }$/p' ring_core/src/lib.rs
awk '/pub fn capacity/{ m = NR } m && NR == m + 1' ring_core/src/lib.rs
grep -c 'pub fn' ring_core/src/lib.rs
```

Live output:

```
  pub fn capacity( &self ) -> Capacity
  {
    match &self.storage
    {
      Storage::Spsc( ring ) => ring.capacity(),
      Storage::Mpsc( ring ) => ring.capacity(),
      // The validated `Capacity` is carried in the variant rather than rebuilt
      // from `ArrayQueue::capacity`. Rebuilding it re-ran this crate's only
      // fallible validation inside an infallible accessor, so a backend that
      // ever rounded its capacity would turn `ring.capacity()` into a panic.
      #[ cfg( feature = "crossbeam" ) ]
      Storage::Crossbeam( _, capacity ) => *capacity,
    }
  }
  {
14
```
