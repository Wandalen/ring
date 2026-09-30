# The `unsafe_code` Opt-Out and Its Obligations

### Scope

- **Purpose**: Justify this crate's `#![ allow( unsafe_code ) ]` — the opt-out from the workspace-wide `unsafe-code = "deny"` — by naming every `unsafe` site, the obligation each carries, and the condition that would retire it.
- **Responsibility**: The census of unsafe sites, the soundness argument, and the deletion condition.
- **In Scope**: The ten `unsafe` lines in `src/lib.rs`, and the allowlist entry permitting them.
- **Out of Scope**: The `unsafe impl Sync` specifically, which carries an obligation of a different kind (→ [`002`](002_an_unsafe_impl_sync_the_compiler_cannot_derive.md)); the allowlist's own third entry (→ [`../decisions/001`](../decisions/001_ring_core_sits_on_the_unsafe_allowlist_without_unsafe.md)).

### The Constraint Being Worked Around

The workspace sets `unsafe-code = "deny"`. A multi-producer ring cannot be
written under it: many producers hold `&Ring` and write into disjoint slots
concurrently, which is exactly the aliasing safe Rust exists to forbid and
exactly what the claim protocol makes sound. No safe API can express "each
producer has exclusive access to the one slot it claimed" — the claim is a
runtime fact about cursor values, and the borrow checker reasons about scopes.

`gate/declared/ring/unsafe_allowlist.txt` records the ruling permitting three
crates to opt out. This is the justification that entry requires.

### Every Unsafe Site

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep 'unsafe' src/lib.rs | grep -vE '^ *(//|///|//!)'
```

Live output:

```
#![ allow( unsafe_code ) ]
unsafe impl< S : Send > Sync for Ring< S > {}
  unsafe fn slot( &self, seq : Seq ) -> &S
    unsafe { &*self.slots.at( seq ).get() }
  unsafe fn slot_mut( &self, seq : Seq ) -> &mut S
    unsafe { &mut *self.slots.at( seq ).get() }
    unsafe { self.ring.slot_mut( self.seq ) }
    unsafe { self.ring.slot_mut( self.seq ) }
    Some( unsafe { self.ring.slot( self.start.advanced_by( offset as u64 ) ) } )
    Some( unsafe { self.ring.slot_mut( self.start.advanced_by( offset as u64 ) ) } )
```

**Ten lines, four kinds, one obligation each:**

| Kind | Sites | Obligation |
|------|-------|------------|
| The opt-out attribute | 1 | This file |
| `unsafe impl Sync` | 1 | The `Ring` is safely shared — argued at the impl and in [`002`](002_an_unsafe_impl_sync_the_compiler_cannot_derive.md) |
| `unsafe fn slot` / `slot_mut` | 2 declarations, 2 bodies | The caller has claimed or published the sequence being addressed |
| Call sites of those two | 4 | Each is inside a `Reserved` or a `Batch`, both of which exist only for a sequence the protocol has already gated |

**The four call sites are the ones that matter**, and all four are structural:
`Reserved` is constructed only by `claim`, and `Batch` only by `drain`, so no
path reaches `slot_mut` for a sequence that was not first claimed and no path
reaches `slot` for one not first published.

### Why It Cannot Live Lower

`ring_store` and `ring_slot` hold storage and no cursors. The invariant making
a shared slot write sound is stated entirely in terms of cursors — the producer
writes only what it has claimed and not published, the consumer reads only what
is published and not committed — so a crate holding only storage cannot
encapsulate it behind a safe API whatever shape that API takes. The closest
candidate considered, a `SharedBuffer::split()`, works through this and is
found unsound.

The `UnsafeCell` placement carries the same lesson at a smaller scale. It
wrapped the whole `Buffer` until Miri reported a retag conflict on
`Buffer< S >` itself: reaching a slot through `( *cell.get() ).at_mut( seq )`
materialises `&mut Buffer< S >`, an exclusive claim over the entire allocation,
so two producers writing two different slots aliased the whole buffer. The cell
is now per-slot, claiming exactly what the protocol guarantees to be exclusive.

### Deletion Condition

A safe abstraction that expresses "exclusive access to one element of a shared
array, gated by a runtime cursor comparison" — either from `std` or from a
crate the family is willing to depend on. None exists today.

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'crates with unsafe in code: '
for c in ring_*/; do
  cat ${c}src/*.rs 2>/dev/null | grep -vE '^\s*(//|///|//!)' | grep -q 'unsafe' && printf '%s ' "${c%/}"
done; echo
```

Live output:

```
crates with unsafe in code: ring_mpsc ring_spsc 
```

Two crates, both complete rings, both for the same reason. The day that count
reaches zero, this instance is obsolete.

### MP50 — Every Unsafe Call Site Is Structurally Gated

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep 'unsafe {' src/lib.rs | grep -vE '^[0-9]+: *(//|///|//!)'
```

Live output:

```
        unsafe { &*self.slots.at(seq).get() }
        unsafe { &mut *self.slots.at(seq).get() }
        unsafe { self.ring.slot_mut(self.seq) }
        unsafe { self.ring.slot_mut(self.seq) }
        Some(unsafe { self.ring.slot(self.start.advanced_by(offset as u64)) })
        Some(unsafe { self.ring.slot_mut(self.start.advanced_by(offset as u64)) })
```

Four blocks: two in `Reserved`'s accessors, two in `Batch`'s. `Reserved` is
returned only by `claim` and `Batch` only by `drain`, so reaching either means
the protocol already established the sequence's state.

**The obligation is discharged by construction rather than by an assertion**,
which is why there is no `debug_assert` anywhere in this crate — a check would
have nothing to check that the type system has not already arranged.

### MP51 — Ten Unsafe Lines, and the Same Ten in the Sibling

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/; do
  n=$( cat ${c}src/*.rs 2>/dev/null | grep -vE '^\s*(//|///|//!)' | grep -c 'unsafe' )
  [ "$n" -gt 0 ] && printf '%-14s %d\n' "${c%/}" "$n"
done
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
ring_mpsc      10
ring_spsc      10
```

Two crates, equal counts, same reason — both assemble a complete ring, both need
the same four kinds of site. The symmetry is evidence for the siting
argument above: the unsafe tracks "holds storage *and* the cursors bounding it", not
"holds storage".
