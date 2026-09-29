# Item: Four Impls, and What the Blanket One Does Not Claim

### Scope

**Purpose:** Read the crate's four impl blocks as declarations — what each one
claims, what the unbounded `Fill` impl leaves unclaimed, and which payloads a
downstream crate can and cannot teach to fill a slot.

**Responsibility:** The four impls at `:66`, `:80`, `:127` and `:137`, the
diagonal the blanket impl covers, the orphan rule's effect on the byte half, and
the call form every byte payload in the family is forced into.

**In Scope:** `ring_event/src/lib.rs:66`, `:80`, `:127`, `:137`,
`:138-142`; probes below.

**Out of Scope:** That the extension point exists at all is
[`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md).
What `Peek`'s two impls consult is
[`invariant/001`](../invariant/001_two_readings_of_one_emptiness.md).

---

## The Four Impls, and the Words Above Them

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every impl in the crate --'
command grep '^impl' ring_event/src/lib.rs
echo '  -- what the blanket one claims, in full --'
command grep -m1 -B1 -A7 -F 'impl< T > Fill< TypedSlot< T > > for T' ring_event/src/lib.rs
echo '  -- and what publish_into calls the operation --'
command grep -m1 -A4 -F '/// Publish `payload` into `slot` — the one write path both shapes take.' ring_event/src/lib.rs
echo '  -- every byte payload handed to publish_into anywhere --'
command grep -r 'publish_into( [^)]*b"' --include=*.rs | command grep -o 'publish_into( .*' | sort | uniq -c | sort -rn
```

Live output:

```
  -- every impl in the crate --
impl< T > Fill< TypedSlot< T > > for T
impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
impl< T > Peek for TypedSlot< T >
impl< const N : usize > Peek for BytesSlot< N >
  -- what the blanket one claims, in full --

impl< T > Fill< TypedSlot< T > > for T
{
  fn fill( self, slot : &mut TypedSlot< T > ) -> Result< (), RingError >
  {
    // Discarding the displaced value is `fill`'s documented contract — "write
    // `self` into `slot`, replacing whatever it held" — not an oversight. A
    // caller that needs the old record calls `TypedSlot::set` directly and
    // binds it; this trait exists to give both slot shapes one signature, and
  -- and what publish_into calls the operation --
/// Publish `payload` into `slot` — the one write path both shapes take.
///
/// Deliberately trivial. Its value is not what it does but that there is only
/// one of it: a ring's publish *path* passes through here — the step where a
/// claimed slot receives its payload — so no slot shape can acquire a publish
  -- every byte payload handed to publish_into anywhere --
      1 publish_into( &mut slot, &b"ab"[ .. ] ).unwrap();
      1 publish_into( &mut slot, &b"abcd"[ .. ] ).unwrap();
      1 publish_into( &mut slot, &b"abcde"[ .. ] );
      1 publish_into( &mut bytes, &b"x"[ .. ] ).unwrap();
      1 publish_into( &mut bytes, &b"payload"[ .. ] ).unwrap();
      1 publish_into( &mut bytes, &b"hi"[ .. ] ).unwrap();
```

## A Downstream Impl Beside the Blanket One

*The probe below is frozen at authoring time — the scratch crate that ran it
has since been swept — and no drift was found: the blanket impl and
`TypedSlot`'s signature (`ring_event/src/lib.rs:66`) are unchanged, and
the printed values are plain `Option< u64 >`/`Option< i32 >`, uninvolved
with `BytesSlot`'s since-rewritten `Debug` (see
[`invariant/002`](../invariant/002_a_refused_fill_changes_nothing.md)). The
recorded output should still reproduce; it just cannot be re-run to confirm.*

```rust
// -ev_probe/src/bin/cross_typed.rs
struct Millis( u64 );

impl Fill< TypedSlot< u64 > > for Millis
{
  fn fill( self, slot : &mut TypedSlot< u64 > ) -> Result< (), RingError >
  {
    slot.set( self.0 * 1000 );
    Ok( () )
  }
}

publish_into( &mut converted, Millis( 3 ) ).unwrap();   // TypedSlot< u64 >
publish_into( &mut direct, Millis( 3 ) ).unwrap();      // TypedSlot< Millis >
```

```
    converted        Some(3000)
    blanket still on Some(3)
```

## The Two Payloads a User Reaches For First

*Both diagnostics below are frozen at authoring time: the probe binaries
have since been swept, and rustc's exact error wording, span formatting and
E-code text are not a stability guarantee across compiler versions. The
orphan rule's `Fill`/`BytesSlot`/`Vec` triangle and the unsizing-coercion
gap between `&[ u8; 4 ]` and `&[ u8 ]` are properties of the language and
this crate's own impls (`ring_event/src/lib.rs:80`), not of any
particular compiler release, so the two refusals demonstrated (`E0117`
orphan, `E0277` missing unsize) should still hold; only the literal compiler
text is not re-verified.*

```rust
// -ev_probe/src/bin/foreign_payload.rs
impl< const N : usize > Fill< BytesSlot< N > > for Vec< u8 >
```

```
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
 --> src/bin/foreign_payload.rs:7:1
  |
7 | impl< const N : usize > Fill< BytesSlot< N > > for Vec< u8 >
  | ^^^^^^^^^^^^^^^^^^^^^^^^----------------------^^^^^---------
  |                         |                          |
  |                         |                          `Vec` is not defined in the current crate
  |                         `BytesSlot` is not defined in the current crate
  |
  = note: impl doesn't have any local type before any uncovered type parameters
```

```rust
// -ev_probe/src/bin/array_payload.rs
publish_into( &mut slot, b"abcd" ).unwrap();
```

```
error[E0277]: the trait bound `&[u8; 4]: Fill<BytesSlot<8>>` is not satisfied
   --> src/bin/array_payload.rs:9:28
    |
  9 |   publish_into( &mut slot, b"abcd" ).unwrap();
    |   ------------             ^^^^^^^ the trait `Fill<BytesSlot<8>>` is not implemented for `&[u8; 4]`
    |   |
    |   required by a bound introduced by this call
    |
help: the trait `Fill<BytesSlot<8>>` is not implemented for `&[u8; 4]`
      but it is implemented for `&[u8]`
help: convert the array to a `&[u8]` slice instead
    |
  9 |   publish_into( &mut slot, &b"abcd"[..] ).unwrap();
    |                            +       ++++
```

---

### EV27 — The Blanket Impl Covers Only the Diagonal, Which Makes `Fill` a Conversion Point Nobody Documents

`impl< T > Fill< TypedSlot< T > > for T` reads, at a glance, like it consumes the
whole typed half of the design space: every type fills a typed slot, so what is
left for anyone else to implement?

It does not say that. It says every type `T` fills a `TypedSlot< T >` — the
diagonal, and only the diagonal. A `TypedSlot< U >` for `U ≠ T` is untouched by
it, and the coherence checker agrees: the probe declares
`impl Fill< TypedSlot< u64 > > for Millis` in a downstream crate, it compiles
alongside the blanket impl, and both are reachable in the same program.
`publish_into( &mut converted, Millis( 3 ) )` lands `3000` in a `TypedSlot< u64 >`
while `publish_into( &mut direct, Millis( 3 ) )` still lands `Millis( 3 )` in a
`TypedSlot< Millis >`, selected by the slot type at the call site.

That second impl multiplies by a thousand. It is a unit conversion performed
during a publish.

**Finding.** `publish_into` is documented as "deliberately trivial", and it is —
the function forwards and nothing else. But the operation it names is not
trivial, because `Fill` is the payload's own code and the payload chooses what
landing means. A downstream crate can normalise, scale, truncate, timestamp or
validate inside `fill`, and the ring's publish path will run it. That is a real
capability of the design and arguably its best one; the crate never mentions it,
and the sentence a reader is most likely to carry away says the opposite of it.
Two clauses on the blanket impl — that it covers `T` into `TypedSlot< T >` only,
and that a payload filling a differently-typed slot is both permitted and where
conversion belongs — would state what the type system already enforces.

---

### EV28 — The Extension Point Is Open on the Typed Side and Closed on the Byte Side, and Nothing Says So

`decisions/001` establishes that a downstream crate can add payload kinds and
even a third slot shape. That is true for types it owns. It is not true for the
byte half, in the specific case a user meets first.

`Fill` is foreign to a downstream crate, `BytesSlot` is foreign, and `Vec< u8 >`
is foreign. The probe's `impl Fill< BytesSlot< N > > for Vec< u8 >` is `E0117`
with the orphan rule named in the diagnostic, and the same refusal applies to
`String`, `&str`, `Box< [ u8 ] >` and `Cow< '_, [ u8 ] >` for the same reason.
The one shipped byte impl is `for &[ u8 ]`, so every byte payload in the world
has to become a `&[ u8 ]` before it can be published, and a caller holding a
`Vec< u8 >` writes `&vec[ .. ]` rather than teaching `Vec` anything.

The literal case is sharper still. `b"abcd"` is a `&[ u8; 4 ]`, not a `&[ u8 ]`,
and no unsizing coercion happens while resolving `P : Fill< S >`, so
`publish_into( &mut slot, b"abcd" )` is `E0277`. All four byte publishes in the
workspace are written `&b"…"[ .. ]` — the census finds no other form, because
there is no other form that compiles.

**Finding.** The typed half admits any local payload against any slot type
(EV27); the byte half admits nothing new at all from outside this crate, and
requires a slicing expression at every call site inside it. Neither limit is a
defect — both follow from coherence rules the language is right to have, and
rustc reports each of them with a diagnostic that names the fix — but the
asymmetry is real, it is invisible from the trait declaration, and the crate's
own account of `Fill` as the open extension point does not qualify itself. One
sentence on the byte impl, saying that a foreign payload type must reach it as a
`&[ u8 ]` because the orphan rule forbids the alternative, converts a wall a
reader hits into a wall a reader was warned about.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_five_public_items_and_their_traffic.md) | The declarations these impls satisfy |
| [`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | The extension point this bounds |
| [`api/002`](../api/002_a_result_one_impl_can_never_return.md) | The `Ok( () )` the blanket impl always returns |
| [`algorithm/002`](../algorithm/002_the_dispatch_happens_before_the_program_runs.md) | The resolution these errors come out of |

### Sources

| Fact | Where |
|------|-------|
| The four impls | `ring_event/src/lib.rs:66`, `:80`, `:127`, `:137` |
| The blanket impl's body | `ring_event/src/lib.rs:66-78` |
| "Deliberately trivial" | `ring_event/src/lib.rs:149-152` |
| A downstream cross-typed impl coexisting with the blanket | Probe above |
| The orphan rule refusing a foreign byte payload | Probe above |
| A byte-string literal not satisfying the byte impl | Probe above |
| All four byte publishes using the slicing form | Census above |

### Tests

| Test | Covers |
|------|--------|
| `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` | The extension point, at a local payload |
| `peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type` | The two `Peek` impls, from the reader's side |
| `the_same_generic_body_round_trips_a_bytes_slot` | The byte impl, reached the only way it can be |
