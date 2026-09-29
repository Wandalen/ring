# pitfall

Both pitfalls were the same two derives seen from two sides. `BytesSlot` used to
derive `PartialEq` and `Debug` over its whole `[ u8; N ]`, while every accessor
the type offers — `read`, `len`, `is_empty`, `capacity` — is bounded by
`self.len`. So the crate carried two inconsistent notions of what a slot *is*,
and nothing in the source named the split. Both traits are now hand-written over
`read()` instead, so the split is gone rather than merely documented.

The first instance finds the one test positioned to catch it asserting the
opposite: `slots_compare_by_payload_not_by_tail` is named for, documented as, and
understood to pin a property `BytesSlot` does not have, and passes because its
two fixtures are constructed identically. The second follows the same derives to
`clear`, whose trait contract promises the slot drops what it held — true of
`TypedSlot`, half-true of `BytesSlot`, where a cleared slot still prints `secret`
under `Debug` and compares unequal to a fresh one.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Test That Names a Property the Type Lacks](001_the_test_that_names_a_property_the_type_lacks.md) | SL41, SL42 — an assertion that passes for the wrong reason, and two relations the crate never reconciles |
| 002 | [Clear Forgets, It Does Not Erase](002_clear_forgets_it_does_not_erase.md) | SL43, SL44 — one contract with two deliveries, and residue that outlives its publish |

### Two Relations, One Type

| Observation | Reads | Sees the tail? |
|-------------|-------|:--------------:|
| `read()` | `&self.bytes[ ..self.len ]` | ✘ |
| `len()` | `self.len` | ✘ |
| `is_empty()` | `self.len == 0` | ✘ |
| `capacity()` | `N` | ✘ |
| `PartialEq` (derived) | every field, all `N` bytes | **✔** |
| `Debug` (derived) | every field, all `N` bytes | **✔** |

Four accordant accessors, two dissenting derives, and no line of source
acknowledging that the two disagree. A caller comparing slots — in an assertion,
a dedup, a change-detection check — gets the array relation while reading the
source for the accessor relation.

### The Test That Cannot Fail

```rust
let mut written_once = BytesSlot::< 8 >::empty();
written_once.write( b"ab" ).unwrap();

let mut overwritten = BytesSlot::< 8 >::empty();   // never overwritten
overwritten.write( b"ab" ).unwrap();

assert_eq!( written_once, overwritten );
```

The variable named `overwritten` is written exactly once, from the same payload,
so both backing arrays are `[ 97, 98, 0, 0, 0, 0, 0, 0 ]` and the assertion holds
for a reason unrelated to the property claimed. Build the fixture the name
describes — write `"XXXXXXXX"`, then `"ab"` — and the two slots are
indistinguishable through every public accessor and compare **unequal**.

### One Contract, Two Deliveries

| | `TypedSlot< T >` | `BytesSlot< N >` |
|---|---|---|
| Contract, as now written | *"Not a promise to overwrite"* — names both | *same paragraph* |
| `clear` body | `self.0 = None` | `self.len = 0` |
| Payload after | Dropped; heap freed | Resident, unreachable through the trait |

Both are correct implementations — zeroing `N` bytes per clear would put a memset
on the drain path of a ring whose whole premise is allocate-once reuse. The
defect recorded here was that the contract promised the stronger thing for both
and neither the trait's doc nor `BytesSlot`'s said which of the two you get.
That is closed: `Slot::clear` now states outright that it is **not a promise to
overwrite**, names which shape drops and which only moves a length, and says how
long the residue lasts. The residue also stopped being observable — `Debug` and
`PartialEq` are hand-written over `read()`, so neither shows a byte past `len`.

Two generic call sites clear slots — `ring_store::Buffer::clear` sweeps the
whole buffer, `ring_event::recycle` clears one — and neither can distinguish the
implementations. That has not changed. What changed is that the contract they
are written against now describes the split instead of denying it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the test whose fixture cannot distinguish the two relations
command grep -m1 -A13 -F '/// Two slots holding the same payload compare equal even when they arrived at' ring_slot/tests/slot_test.rs

# the derive that compares every byte, including the unreachable tail
command grep -m1 -A7 -F '/// assert!( !slot.is_empty() );' ring_slot/src/lib.rs | tail -n 6

# and read's length bound, which the four accessors share
grep -n -A3 'pub fn read' ring_slot/src/lib.rs

# one contract, two implementations
awk '/^  \/\/\/ Return the slot to its empty state\.$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 7 { print } /^    self\.0\.is_none\(\)$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 6 { print } /^    Self::is_empty\( self \)$/{ n3 = NR } n3 && NR >= n3 + 3 && NR <= n3 + 6 { print }' ring_slot/src/lib.rs

# every place the family clears a slot, both generic over the trait
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'Slot::clear\|slot\.clear()' ring_*/src/*.rs \
  | grep -v '^ring_slot/' | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sort
```

The counterexample fixtures, the `secret` residue, and the `Debug` output all
come from a release probe; the figures are quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL41 | `ring_slot` | **wrong doc** | `slots_compare_by_payload_not_by_tail` asserts a property `BytesSlot` does not have; it passes because both fixtures are written once from empty, so no tail can differ |
| SL42 | `ring_slot` | **latent hazard** | Four accessors define the slot as its first `len` bytes and two derives define it as all `N`; nothing in the source names the split, and the one test positioned to catch it denies it |
| SL43 | `ring_slot` | n/a — doc gap | `Slot::clear` promises the payload is dropped; `BytesSlot` resets a length and leaves the bytes printable by `Debug` and significant to `PartialEq`, unremarked on either type |
| SL44 | `ring_slot` | **latent hazard** | A slot holds the longest payload ever written to its position, not the current one — unreachable through `read()`, reachable through `Debug`, and printed by any diagnostic dump or assertion failure |
