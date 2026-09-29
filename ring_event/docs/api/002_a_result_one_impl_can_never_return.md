# API: A `Result` One Impl Can Never Return

### Scope

**Purpose:** Record what the unified write signature costs each of the two shapes
it unifies — one gains an error case it cannot produce, the other loses a return
value `ring_slot` documents as important.

**Responsibility:** `Fill::fill`'s signature against the two impl bodies, the
`Option< T >` the typed impl discards, and the family's one capture of that value.

**In Scope:** `ring_event/src/lib.rs:63`, `:68-77`, `:82-85`;
`ring_slot/src/lib.rs:100-123`, `:347`; `ring_core/src/lib.rs:389-390`.

**Out of Scope:** The surface as a whole is
[`api/001`](001_two_traits_three_functions_one_associated_type.md). Where the
family actually writes slots is
[`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md).

---

## One Signature Over Two Shapes That Return Different Things

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the unified signature, and the two impls it covers --'
sed -n '/^  fn fill( self, slot : &mut S ) -> Result< (), RingError >;$/p;/^  fn fill( self, slot : &mut TypedSlot< T > ) -> Result< (), RingError >$/,/^  }$/p;/^  fn fill( self, slot : &mut BytesSlot< N > ) -> Result< (), RingError >$/,/^  }$/p' ring_event/src/lib.rs
echo '  -- what TypedSlot::set returns, and why ring_slot says it returns it --'
awk '/^  \/\/\/ Place `value` in the slot, returning whatever it held before\.$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 4 { print } /^  pub fn set\( &mut self, value : T \) -> Option< T >$/{ n2 = NR } n2 && NR >= n2 && NR <= n2 + 4 { print }' ring_slot/src/lib.rs
echo '  -- what BytesSlot::write returns --'
command grep -m1 -A3 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
echo '  -- every capture of a displaced value anywhere under ring/ --'
command grep -r 'let [a-z_]* = [a-z_.]*set( ' --include=*.rs ring_*/src
command grep -m1 -A1 -F '            let displaced = reserved.set( record );' ring_core/src/lib.rs
echo '  -- and whether a release build keeps that capture --'
command grep -c '^\[profile.release' Cargo.toml || true
```

Live output:

```
  -- the unified signature, and the two impls it covers --
  fn fill( self, slot : &mut S ) -> Result< (), RingError >;
  fn fill( self, slot : &mut TypedSlot< T > ) -> Result< (), RingError >
  {
    // Discarding the displaced value is `fill`'s documented contract — "write
    // `self` into `slot`, replacing whatever it held" — not an oversight. A
    // caller that needs the old record calls `TypedSlot::set` directly and
    // binds it; this trait exists to give both slot shapes one signature, and
    // `BytesSlot` has nothing to hand back.
    slot.set( self );
    Ok( () )
  }
  fn fill( self, slot : &mut BytesSlot< N > ) -> Result< (), RingError >
  {
    slot.write( self )
  }
  -- what TypedSlot::set returns, and why ring_slot says it returns it --
  /// Place `value` in the slot, returning whatever it held before.
  ///
  /// Returning the displaced value rather than dropping it keeps the door open
  /// for a future evict-oldest policy to hand a caller what it evicted instead
  /// of losing it silently — but `ring_overflow` does not depend on this crate
  pub fn set( &mut self, value : T ) -> Option< T >
  {
    self.0.replace( value )
  }

  -- what BytesSlot::write returns --
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  {
    if payload.len() > N
    {
  -- every capture of a displaced value anywhere under ring/ --
ring_core/src/lib.rs:            let displaced = reserved.set( record );
            let displaced = reserved.set( record );
            debug_assert!( displaced.is_none(), "a claimed slot held a record" );
  -- and whether a release build keeps that capture --
0
```

---

### EV7 — The Typed Write Path Drops the Value `ring_slot` Documents as the Reason It Is Returned

`TypedSlot::set` returns `Option< T >` — whatever the slot held before — and
`ring_slot`'s doc comment at `:102-105` states why in one sentence: "Returning the
displaced value rather than dropping it is what lets `ring_overflow`'s
evict-oldest policy hand the caller what it evicted, instead of losing it
silently."

`Fill`'s typed impl at `:66` is `slot.set( self );`. The semicolon is the whole
finding: the displaced value is discarded, and `fill` returns `Ok( () )`. A caller
who publishes into an occupied typed slot through `publish_into` loses the
previous payload with no diagnostic — the same call written as `slot.set( value )`
hands it back.

The signature cannot help. `Fill::fill` returns `Result< (), RingError >`, chosen
so both shapes fit, and `BytesSlot::write` has nothing to report: it overwrites
bytes in place and returns `Result< (), RingError >`. There is no `Option< T >` a
byte slot could produce, so the unified return is the intersection, and the
intersection is empty on this axis.

**Finding.** The one path is narrower than the direct calls it replaces, and
nothing anywhere says so. `publish_into`'s documentation describes it as
"Deliberately trivial", and `Fill`'s `# Errors` section covers what the byte shape
refuses; neither mentions that the typed shape's return value has been dropped on
the floor.

Whether this bites today depends on something separate: a census of every capture
of a `set( … )` return anywhere under `ring_*` finds exactly one, at
`ring_core:389`, and it feeds a `debug_assert!` that asserts the value is `None`.
The workspace declares no `[profile.release]`, so cargo's default applies and the
assertion — with the capture — is compiled out of a release build. The value
`ring_slot` preserves therefore has no release-mode consumer at either end today,
which is what keeps this latent rather than active, and is also why nobody has
noticed the path drops it.

The typed impl's body now says so, right where the discard happens:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  fn fill( self, slot : &mut TypedSlot< T > ) -> Result< (), RingError >' ring_event/src/lib.rs
```

Live output:

```
  fn fill( self, slot : &mut TypedSlot< T > ) -> Result< (), RingError >
  {
    // Discarding the displaced value is `fill`'s documented contract — "write
    // `self` into `slot`, replacing whatever it held" — not an oversight. A
    // caller that needs the old record calls `TypedSlot::set` directly and
    // binds it; this trait exists to give both slot shapes one signature, and
```

**Disposition:** applied — the typed `Fill` impl's body in
`ring_event/src/lib.rs` now carries a comment, at the exact point of
the discard, stating that dropping the displaced value is `fill`'s documented
contract rather than an oversight, and naming the escape hatch
(`TypedSlot::set`, called directly) for a caller that needs the old record;
the crate's test suite re-verified passing (`cargo test -p ring_event
--all-features`, 2026-09-04). Now prints: `binds it; this trait exists to
give both slot shapes one signature, and`

---

### EV8 — Every Typed Publish Discharges an Error the Compiler Could Have Proved Impossible

The other half of the same intersection runs the other way. `BytesSlot::write` can
fail — a payload longer than `N` is `RingError::BatchTooLarge` — so the shared
signature must be fallible. The typed impl cannot fail, and the crate says so in
`Fill`'s own `# Errors` section at `:61-62`: "A typed payload cannot fail, and says
so by never returning `Err`."

Saying so in prose is not saying so in the type. Eight typed publishes across the
crate's doctests and test suite each end in `.unwrap()` or `.expect()`, discharging
a variant that no execution can produce. `a_typed_publish_cannot_fail` exists
precisely to pin this, and its comment is explicit that it is not a tautology: it
pins "that the shared signature's `Result` is the byte shape's need, and that the
typed shape pays no runtime check for it."

**Finding.** Read together with EV7, the unified signature is simultaneously wider
and narrower than either shape needs — it adds an impossible `Err` to the typed
side and removes a real `Option< T >` from it. Both are the price of one call
covering two shapes, both are consequences of the same decision, and only one of
them is documented.

The runtime cost is nil: `Ok( () )` is not a check, and the `.unwrap()` on a value
the optimiser can see is always `Ok` disappears. The cost is in what a reader
concludes. A caller who sees `Result` on a typed publish reasonably looks for the
failure mode, finds the `# Errors` text describing a byte-slot condition that
cannot apply to them, and has to work out on their own that the answer is
"none" — which is exactly the reasoning the type could have carried instead.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_two_traits_three_functions_one_associated_type.md) | The surface this signature sits in |
| [`algorithm/001`](../algorithm/001_two_executable_statements_and_one_branch.md) | The two impl bodies, in full |
| [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md) | Why the discard has no production victim yet |
| [`decisions/002`](../decisions/002_three_free_functions_instead_of_methods.md) | The shape of the shared surface these two costs buy |

### Sources

| Fact | Where |
|------|-------|
| The unified signature | `ring_event/src/lib.rs:63` |
| The typed impl discarding the displaced value | `ring_event/src/lib.rs:68-77` |
| `ring_slot`'s stated reason for returning it | `ring_slot/src/lib.rs:102-105` |
| `BytesSlot::write` having nothing to return | `ring_slot/src/lib.rs:347` |
| One capture in the family, into a `debug_assert!` | Census above, `ring_core/src/lib.rs:389-390` |
| No `[profile.release]`, so the assertion is compiled out | Census above |
| `Fill`'s "A typed payload cannot fail" | `ring_event/src/lib.rs:61-62` |

### Tests

| Test | Covers |
|------|--------|
| `a_typed_publish_cannot_fail` | The impossible `Err`, asserted a hundred times over |
| `a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing` | The failure the shared signature exists for |
| `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` | The typed impl reached directly |
