# Pattern: Delegate the Fold, Own the Storage

### Scope

**Purpose:** Record that the family's sequence-to-index fold has one
implementation reached two ways — transitively through `Buffer::at`, which is how
both rings get it, and directly through `ring_index::of`, which one crate does —
and that the direct route takes its capacity as a free parameter with nothing
binding it to a buffer.

**Responsibility:** The delegation pattern as a shape: who owns the fold, who
owns the storage, and what the two routes to a slot address cost.

**In Scope:** `ring_store/src/lib.rs:220-257`;
`ring_batch/src/lib.rs:358-362`; every `ring_index` dependent.

**Out of Scope:** The mask arithmetic is
[`algorithm/001`](../algorithm/001_one_mask_no_modulo.md). What happens to an
out-of-range index is
[`decisions/001`](../decisions/001_panic_rather_than_option.md).

---

### BF34 — The Fold Reaches Both Rings Without Either Depending On It

`ring_index` has two dependents outside itself:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -l 'ring_index' ring_*/Cargo.toml | sed 's|ring/||;s|/Cargo.toml||' | sort
```

Live output:

```
ring_batch
ring_store
ring_index
```

Neither `ring_mpsc` nor `ring_spsc` is among them, and they are the crates that
address slots by sequence for a living. Every call site of the fold, family-wide:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn '[^_a-zA-Z]of( ' ring_*/src/*.rs | grep -vE ':[[:space:]]*(///|//!|//)' \
  | grep -vE '::of\(|fn of\(' | sed 's|ring/||' | sort | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_batch/src/lib.rs:  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
ring_store/src/lib.rs:    self.get( of( seq, self.capacity ) )
ring_store/src/lib.rs:    let index = of( seq, self.capacity );
ring_index/src/lib.rs:  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()
ring_index/src/lib.rs:  of( a, capacity ) == of( b, capacity )
```

Three call sites outside `ring_index`, two of them here — and here is where the
rings reach it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A13 -F '  /// assert_eq!( buffer.get( SlotIndex( 2 ) ).get(), Some( &1 ) );' ring_store/src/lib.rs | tail -n 12
```

Live output:

```
  #[ must_use ]
  pub fn at( &self, seq : Seq ) -> &S
  {
    self.get( of( seq, self.capacity ) )
  }

  /// Mutably borrow the slot a sequence addresses.
  #[ must_use ]
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
  {
    let index = of( seq, self.capacity );
    self.get_mut( index )
```

**Finding.** The pattern works exactly as intended. One crate owns the fold, one
crate owns the storage, and the two are composed in a pair of three-line methods
that every ring calls without knowing either tier exists. `ring_mpsc` and
`ring_spsc` carry no `ring_index` dependency, cannot get the mask wrong, and
cannot drift from `ring_batch`'s version of it, because there is no version of it
to drift from — there is one function.

The composition point is what makes it work, and it is worth naming: `at` binds
the fold to *this buffer's* capacity, `self.capacity`, which is the field set in
`new` alongside the slice length. A caller cannot supply the wrong capacity to
`at` because a caller does not supply one at all. That is the whole safety
argument for the sequence-addressed route, and it holds because of one word in
the body.

---

### BF35 — The Direct Route Takes Capacity as a Free Parameter, and Nothing Binds It

The one caller of `of` outside this crate does not have a buffer:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs
```

Live output:

```
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
{
  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
}
```

`capacity` arrives as an argument. `drain_order` emits `SlotIndex` values folded
against whatever it was handed, hands them to a caller, and that caller indexes a
buffer `ring_batch` never saw.

**Finding.** There are two routes to a slot address in this family and they have
different safety properties. `Buffer::at` reads the capacity out of the buffer
being indexed, so a mismatch is unrepresentable. `ring_batch::drain_order` takes
the capacity as a parameter, so a mismatch is one wrong argument away and the
resulting `SlotIndex` is a perfectly ordinary value that will index whatever it
is given.

This is the concrete mechanism behind the provenance argument in `get`'s panic
doc ([`decisions/001`](../decisions/001_panic_rather_than_option.md) BF7). That
doc reasons that an out-of-range index means two rings' capacities were mixed,
and treats that as a caller bug worth panicking on. `drain_order` is the function
that would produce one: it is the only place in the family where a `SlotIndex` is
manufactured against a capacity the storage did not supply, and its signature is
where a wrong capacity enters.

Nothing is broken — `drain_order` has one purpose and its caller has the right
capacity in hand — but the pattern is worth stating in both directions. The
delegation is safe *because* `Buffer::at` closes over its own field, and the one
place that opens the parameter back up is also the one place the panic doc's
scenario becomes reachable. Neither crate's docs connect the two.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/002`](002_every_write_is_a_borrow.md) | The other shape this crate commits to |
| [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) | What the delegated function actually computes |
| [`decisions/001`](../decisions/001_panic_rather_than_option.md) | The panic whose scenario `drain_order` makes reachable |
| [`api/002`](../api/002_six_ways_to_reach_a_slot.md) | Both routes as API surface |
| [`item/002`](../item/002_the_slotindex_the_buffer_trusts.md) | The `Seq` that lives for one line inside `at` |

### Sources

| Fact | Where |
|------|-------|
| `ring_index`'s two external dependents | `ring_batch/Cargo.toml`, `ring_store/Cargo.toml` |
| Every call site of the fold | Census above |
| The composition point | `ring_store/src/lib.rs:248, 255` |
| The free capacity parameter | `ring_batch/src/lib.rs:358-362` |
| The provenance argument | `ring_store/src/lib.rs:195-202` |

### Tests

| Test | Covers |
|------|--------|
| `a_sequence_addresses_the_slot_ring_index_says_it_does` | That `at` agrees with `of` at forty sequences |
| `a_full_lap_overwrites_and_a_partial_one_does_not` | The fold's wrap behaviour through storage |
| `ring_batch` — `drain_order` assertions | The direct route, at a capacity the test supplies |
| *(to create)* | An assertion that `drain_order`'s indices and `Buffer::at`'s agree for one capacity |
