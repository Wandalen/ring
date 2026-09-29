# Type: The Const Parameter as Capacity

### Scope

**Purpose:** Record that a `BytesSlot`'s width is part of its type rather than
part of its value, what that forecloses — runtime configuration, mixed widths in
one container — and that `N` carries no bound anywhere, so both degenerate ends
are instantiable.

**Responsibility:** `N` as a type-level commitment: what it fixes, what it costs,
and what the crate does not constrain about it.

**In Scope:** `ring_slot/src/lib.rs:161, 167, 269, 277` — the four sites
`N` appears; the family's instantiated widths;
`ring_types/src/capacity.rs:23, 60` for the contrast.

**Out of Scope:** What `N` costs in layout is
[`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md).
That `capacity()` reads `N` through a receiver it ignores is
[`item/002`](../item/002_the_six_of_a_bytes_slot.md) SL27. The `Copy` and
payload-channel questions are
[`type/001`](001_two_shapes_one_trait_no_copy.md).

---

## Where `N` Appears

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'const N : usize' ring_slot/src/lib.rs
printf '  where clauses in the crate : %d\n' "$( grep -cE '^[[:space:]]*where' ring_slot/src/lib.rs )"
```

Live output:

```
pub struct BytesSlot< const N : usize >
impl< const N : usize > core::fmt::Debug for BytesSlot< N >
impl< const N : usize > PartialEq for BytesSlot< N >
impl< const N : usize > Eq for BytesSlot< N > {}
impl< const N : usize > BytesSlot< N >
impl< const N : usize > Default for BytesSlot< N >
impl< const N : usize > Slot for BytesSlot< N >
  where clauses in the crate : 0
```

Four declarations, four unbounded `N`, zero `where` clauses in 288 lines. Every
`usize` is a legal width, and every impl applies to all of them.

---

### SL47 — Width Is Part of the Type, So It Cannot Be Configured and Cannot Be Mixed

`BytesSlot< 8 >` and `BytesSlot< 16 >` are unrelated types with different sizes:

```
--- widths are unrelated types ---
  BytesSlot< 8 >  is 16 bytes
  BytesSlot< 16 > is 24 bytes
  no array, Vec, or ring can hold both — they share no supertype but Slot
```

Two consequences follow, and the family exhibits both.

A ring's slot width is fixed at compile time. `Ring< BytesSlot< 8 > >` and
`Ring< BytesSlot< 16 > >` are separate types with separate monomorphisations; a
program that decides its payload width from a config file cannot express it
through this type at all without `dyn Slot`
([`pattern/001`](../pattern/001_one_trait_two_shapes.md) SL37 records that the
trait would permit it and nothing uses it).

That is the opposite of how the family handles a ring's *slot count*:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pub struct Capacity\|pub const fn get' ring_types/src/capacity.rs
grep 'capacity : Capacity' ring_store/src/lib.rs
```

Live output:

```
pub struct Capacity( usize );
  pub const fn get( self ) -> usize
  capacity : Capacity,
  pub fn new( capacity : Capacity ) -> Self
```

A ring takes its capacity as a constructor argument and holds it as a runtime
field. A slot takes its capacity as a type parameter and holds it nowhere at all.
Two capacities, two mechanisms, in adjacent crates.

**Finding.** The split is the right one — a ring's slot count must be chosen at
run time from configuration, and a slot's byte width must be known statically or
the array cannot be inline — and it is nowhere stated. A reader who has met
`Capacity` first and reaches for a runtime slot width finds no explanation of why
the same word takes two forms one tier apart, and the mistake surfaces as a
type error with no message pointing at the design reason behind it.

The second consequence is narrower and sharper: no container in the family can
hold two widths. A process handling both eight- and sixteen-byte wire frames
needs two rings, not one ring with two slot kinds. That is a real architectural
constraint imposed by a type parameter, and the module comment — which does
explain why a slot cannot grow — does not mention it.

---

### SL48 — `N` Is Unbounded, and Both Degenerate Ends Compile

With no `where` clause anywhere, `N` accepts every `usize`. The zero end is
coherent:

```
--- the zero end ---
  capacity            = 0
  write of nothing    = Ok(())
  write of one byte   = true
  size_of             = 8 bytes to carry 0
  is_empty            = true
```

A `BytesSlot< 0 >` accepts an empty write, refuses anything longer, reports
itself empty, and costs eight bytes to carry nothing
([`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md)
prices the length field that makes those eight bytes unavoidable). The crate's
suite covers it deliberately, in `a_zero_capacity_slot_accepts_only_nothing`.

The far end is reached once and only to read a number back:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'BytesSlot::< 4096 >' ring_slot/tests/slot_test.rs
```

Live output:

```
  assert_eq!( BytesSlot::< 4096 >::empty().capacity(), 4096 );
```

One line, asserting that `capacity()` returns the parameter. Nothing writes to a
wide slot, reads from one, puts one in a ring, or measures what one costs — and
the type goes very much further than 4096:

```
--- the far end, instantiable with no bound ---
  BytesSlot< 65536 >  = 65544 bytes, on the stack
```

A sixty-four-kilobyte slot is a legal type. Constructed by value it is a
stack-allocated object of that size, and a ring of a thousand of them is
sixty-five megabytes — allocated once and never freed while the ring lives
([`lifecycle/002`](../lifecycle/002_a_slot_across_a_rings_laps.md)).

What the crates that consume `BytesSlot` actually use is four widths, all small:

```sh
cd "$(git rev-parse --show-toplevel)"
# the (::)? is load-bearing: the turbofish form is more than half of all sites
echo '--- the consuming crates ---'
grep -rhoE 'BytesSlot(::)?< *[0-9]+ *>' \
  $( ls ring_*/src/*.rs ring_*/tests/*.rs | grep -v '^ring_slot/' ) \
  | tr -d ' ' | sed 's|::||' | sort -t'<' -k2 -n | uniq -c
echo '--- and what ring_slot own source and suite add ---'
grep -rhoE 'BytesSlot(::)?< *[0-9]+ *>' ring_slot/src/lib.rs ring_slot/tests/*.rs \
  | tr -d ' ' | sed 's|::||' | sort -t'<' -k2 -n | uniq -c
```

Live output:

```
--- the consuming crates ---
      7 BytesSlot<4>
     12 BytesSlot<8>
      4 BytesSlot<16>
      1 BytesSlot<32>
--- and what ring_slot own source and suite add ---
      1 BytesSlot<0>
      1 BytesSlot<1>
      1 BytesSlot<2>
      6 BytesSlot<4>
     18 BytesSlot<8>
      7 BytesSlot<16>
      1 BytesSlot<32>
      1 BytesSlot<64>
      1 BytesSlot<4096>
```

Four through thirty-two bytes, twenty-four instantiations, nothing larger —
the crates that build rings out of them. The crate's own suite reaches both ends
the consumers never do: `BytesSlot< 0 >`, `< 1 >`, and `< 2 >` at the bottom, and
one `BytesSlot< 4096 >` in a `capacity()` assertion at the top. So the boundary
band is exercised exactly where it costs nothing and nowhere a ring would pay
for it.

**Finding.** The type admits a range many orders of magnitude wider than the band
any ring uses, with no bound, no assertion, and no note about where the sensible
band ends — and the one wide instantiation that exists asserts a number rather
than exercising a payload, so the far end is named without being tested. That is
not something to fix by adding a constraint — any
ceiling would be arbitrary, and `N` genuinely should stay open for a caller with
a wider frame format. What is missing is the sentence saying where the design
stops working: past a few hundred bytes, `write`'s copy dominates
([`non_functional_requirement/001`](../non_functional_requirement/001_what_a_slot_costs.md)
records it as the crate's one payload-scaled operation), the all-or-nothing write
forces a staging buffer
([`item/002`](../item/002_the_six_of_a_bytes_slot.md) SL28), and cloning moves
the whole array
([`non_functional_requirement/002`](../non_functional_requirement/002_four_traits_and_what_they_compare.md)
SL35 measures 4104 bytes for a one-byte payload).

Each of those is documented as a property in its own place. None of them is
attached to `N`, which is the parameter a caller actually chooses — so the
guidance exists in the corpus and not at the decision point.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_two_shapes_one_trait_no_copy.md) | The other type-level commitments, and the `Copy` neither shape has |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md) | What each `N` costs in bytes at rest |
| [`item/002`](../item/002_the_six_of_a_bytes_slot.md) | `capacity()`, the method that reports `N` |
| [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) | The length field that makes `BytesSlot< 0 >` cost eight bytes |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_a_slot_costs.md) | Where the two `N`-scaled operations are |

### Sources

| Fact | Where |
|------|-------|
| The four `N` sites, unbounded | `ring_slot/src/lib.rs:161, 167, 269, 277` |
| Zero `where` clauses | `ring_slot/src/lib.rs` |
| Ring capacity as a runtime value | `ring_types/src/capacity.rs:23, 60`, `ring_store/src/lib.rs:62, 88` |
| Twenty-four consumer instantiations at four widths | `ring_*/src/*.rs`, `ring_*/tests/*.rs`, excluding `ring_slot` |
| The four widths only the crate's own suite reaches | `ring_slot/tests/slot_test.rs:120, 122, 201, 260` |
| Both degenerate ends, measured | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_zero_capacity_slot_accepts_only_nothing` | `N == 0`, the lower degenerate end |
| `capacity_is_the_const_parameter` | Three widths including 4096, the widest anywhere — and the only touch the far end gets |
| `a_bytes_slot_round_trips_its_payload` | A width in the band the family actually uses |
| `ring_store` — `the_same_buffer_type_serves_both_slot_shapes` | One buffer type over two shapes, but never two widths |
| *(to create)* | Two rings of different widths side by side, pinning that they cannot share a container |
