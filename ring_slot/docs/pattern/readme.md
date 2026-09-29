# pattern

Two conventions leave this crate and are read by the four above it. The first is
structural: one trait, two implementors, static dispatch everywhere. The second
is a signature habit — `set` hands back what it displaced — that four crates
interpret four different ways.

Both instances find the same shape of defect at the edges of an otherwise clean
pattern. The trait is object-safe by construction and nothing in the family uses
it that way, so a supported-or-accidental question was never decided; the trait
also carries lifecycle and no payload, which is correct and leaves half the
abstraction in a crate three tiers up with no pointer to it from here. And the
displaced value's real justification — that it is the only evidence a caller can
obtain that a slot it believed empty was not — is nowhere written, while the
sentence that *is* written points at a policy crate with no dependency on
`ring_slot` at all; the one reader who uses the value uses it for exactly the
unwritten reason, behind a `debug_assert!` release builds delete.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [One Trait, Two Shapes](001_one_trait_two_shapes.md) | SL37, SL38 — object safety nobody claims, and the payload half living one crate away |
| 002 | [The Displaced Value Returned](002_the_displaced_value_returned.md) | SL39, SL40 — a real reason to keep the value that nothing states, and four call sites handling one return four ways |

### The Whole Abstraction

```rust
pub trait Slot
{
  /// Whether this slot currently holds nothing.
  fn is_empty( &self ) -> bool;

  /// Return the slot to its empty state.
  // …followed by a longer paragraph naming which shape drops its payload and
  // which only moves a length. Elided: the surface, not the contract, is what
  // this section is about.
  fn clear( &mut self );
}
```

No supertraits, no associated types, no generic methods, no default bodies — so
`Slot` is object-safe by construction, and a `&dyn Slot` holding both shapes in
one slice compiles and costs a doubled pointer (16 bytes against a thin
reference's 8) plus an indirect call. Nothing in the family pays it: every
consumer is `Buffer< S >` or `Ring< S >`, monomorphised, with `is_empty` and
`clear` inlined to a field comparison and a store.

That the mixed-shape ring is expressible today, with no change to this crate, is
neither endorsed nor forbidden anywhere — the minimalism that makes `Slot` cheap
to implement also made the question unnecessary to answer.

### Three Traits, Two Crates

| Trait | Carries | Where |
|-------|---------|-------|
| `Slot` | Lifecycle — is it empty, empty it | `ring_slot/src/lib.rs:38` |
| `Fill< S >` | The write direction | `ring_event/src/lib.rs:46` |
| `Peek` | The read direction | `ring_event/src/lib.rs:95` |

`Fill< S >` is parameterised by the *slot*, so payload types implement it — which
is why it cannot live here: `ring_slot` would have to know every payload type in
the family, exactly the coupling the generic design avoids. The split is right
and undocumented; the crate's module comment names the two shapes and the cost
asymmetry between them and never mentions that payload access is generic
somewhere else.

### One Return, Four Handlings

| Site | Form | Reads the value? |
|------|------|:----------------:|
| `ring_core/src/lib.rs:389` | `let displaced = reserved.set( record );` | ✔ — then `debug_assert!` |
| `ring_event/src/lib.rs:75` | `slot.set( self );` | ✘ |
| `ring_mpsc/src/lib.rs:914` | `reserved.set( value );` | ✘ |
| `ring_spsc/src/lib.rs:719` | `drop( reservation.set( record ) );` | ✘ — named, then discarded |

All four are placing a value into a slot the caller has just claimed. Nothing
distinguishes the contexts; the variation is stylistic. The one binding checks
`displaced.is_none()` — "a claimed slot held a record", a protocol violation
meaning a live record was silently destroyed — through `debug_assert!`, which is
compiled out of exactly the release builds where the check would matter.

The documented reason for the return names `ring_overflow`'s evict-oldest policy.
`ring_overflow` has no dependency on `ring_slot`, no mention of it, and a surface
consisting of one enum and one pure function from a policy and a stats snapshot
to a `Resolution` — it decides *what should happen* on a full ring and never
holds a slot. The real reason is unstated: the displaced value is the only
evidence a caller can obtain that a slot it believed empty was not.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the trait, entire
command grep -m1 -A7 -F 'pub trait Slot' ring_slot/src/lib.rs

# nothing in the family dispatches dynamically
grep -rn 'dyn Slot\|Box< dyn\|&dyn' ring_*/src/*.rs 2>/dev/null | grep -vE ':[[:space:]]*//' \
  || echo '  none — the trait is never used as an object'

# three traits describing one slot, across two crates
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn '^pub trait' ring_slot/src/lib.rs ring_event/src/lib.rs \
  | sort

# set's stated justification
sed -n '/^  \/\/\/ Place `value` in the slot, returning whatever it held before\.$/,/^  \/\/\/ ```$/p;/^  pub fn set( &mut self, value : T ) -> Option< T >$/,/^    self\.0\.replace( value )$/p' ring_slot/src/lib.rs

# the crate that justification names, and what it actually offers
grep -n 'ring_slot' ring_overflow/Cargo.toml ring_overflow/src/lib.rs 2>/dev/null \
  || echo '  no dependency, no mention'
grep -n '^pub fn \|^pub enum ' ring_overflow/src/lib.rs

# every set in shipped library code
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn '\.set( ' ring_*/src/*.rs | grep -vE ':[[:space:]]*//' | sort

# the one protocol check, and the macro that removes it
command grep -m1 -B1 -A5 -F '          Ok( mut reserved ) =>' ring_core/src/lib.rs
```

Object-safety and fat-pointer figures come from a release probe; they are quoted
in [`pattern/001`](001_one_trait_two_shapes.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL37 | `ring_slot` | n/a — observation | `Slot` is object-safe by construction and the family dispatches statically everywhere; a mixed-shape ring is expressible today and neither endorsed nor forbidden |
| SL38 | `ring_slot` | n/a — doc gap | The trait carries lifecycle and no payload, so the abstraction spans two crates — and nothing here points at `ring_event::Fill`/`Peek`, which hold the other half |
| SL39 | `ring_slot` | **wrong doc** | The reason to keep the displaced value is real and unstated — it is the only evidence a caller can obtain that a slot it believed empty was not; the sentence that is there names a Tier 1 policy crate structurally beside this one, which holds no slot and evicts nothing |
| SL40 | `ring_core` | **latent hazard** | The family's only claimed-slot protocol check sits behind `debug_assert!`, so the one place a silently-destroyed record could be caught is compiled out of release builds |
