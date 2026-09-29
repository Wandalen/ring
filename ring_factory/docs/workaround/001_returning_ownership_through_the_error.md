# Workaround: Returning Ownership Through the Error

### Scope

**Purpose:** Record the language constraint that forces `Registry::register`'s
error type to be a tuple carrying the rejected ring, what this crate does with
that ring, and what the relay costs — including the piece of information the
callee deliberately preserved and this crate deliberately throws away.

**Responsibility:** The `Err( ( RegistryError, Split< T > ) )` shape and its one
consumer, `Factory::build_named`.

**In Scope:** `ring_registry/src/lib.rs`, `register`'s signature and
`RegistryError::NameTaken`'s field; `ring_factory/src/lib.rs`,
`build_named`'s match arm and `BuildError::NameTaken`.

**Out of Scope:** Whether `build_named` should exist at all is
[`api/002`](../api/002_the_named_build_surface.md). The width of the resulting
error is [`item/002`](../item/002_four_nouns_two_of_them_somebody_elses.md) FC28.

---

## The Constraint

`register` takes its ring **by value**. Rust has no way to hand a moved value
back to the caller on a failure path except by putting it in the return type, and
the failure arm of a `Result` is the only place left. So the error becomes a pair:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A6 'pub fn register' ring_registry/src/lib.rs
echo '  -- every tuple-payload error in the 33 crates --'
command grep -rE 'Result< [^,]+, *\( ' --include=*.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
  pub fn register
  (
    &mut self,
    name : impl Into< String >,
    ring : Split< T >,
  )
  -> Result< (), ( RegistryError, Split< T > ) >
  -- every tuple-payload error in the 33 crates --
ring_flush/src/lib.rs:  /// is `ring_tls`'s (a widened `push( item : T ) -> Result< (), ( RingError,
ring_registry/src/lib.rs:  -> Result< (), ( RegistryError, Split< T > ) >
```

One, family-wide. Every other fallible function in the family either takes its
argument by reference or has nothing to give back.

The alternative shapes are all worse in a way the signature makes obvious.
Taking `&mut Split< T >` would mean the registry stores a borrow and outlives
nothing. Taking it by value and dropping it on failure would make a name
collision destroy the caller's ring silently. Returning `Result< Option< Split<
T > >, RegistryError >` puts the success case and the failure case in the same
`Ok` arm. The tuple is ugly and it is the only one that keeps ownership
accounted for at every exit.

---

### FC49 — The One Tuple-Payload Error in the Family, and Its One Caller Drops the Payload

`build_named` binds the returned ring as `_refused` and lets the arm end:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -B4 'RegistryError::NameTaken' ring_factory/src/lib.rs
```

Live output:

```
      Ok( () ) => Ok( () ),
      // The name is discarded rather than carried into `BuildError`: the caller
      // passed it in and still has it. Carrying it would also put a `String` in
      // a `Copy` error type for no new information.
      Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),
```

That is correct, and it is the reason the crate's "a refused registration leaks
nothing and exposes nothing" guarantee is free rather than checked: the caller
never held the ring, so there is nothing for them to leak. The guarantee is
stated in `build_named`'s doc comment as a property of this crate, and it is
really a property of the signature two crates down — `register` took the ring by
value, so a refused call could only ever return it here or destroy it there.

The cost is that the one shape in the family designed to preserve ownership
across a failure is, at its only call site, a drop. Nothing else in the 33 crates
calls `register`, so the tuple exists to serve exactly one consumer, and that
consumer discards half of it.

Deletion condition: if a second caller ever wants the ring back after a name
collision — to retry under a different name, say — the shape is already right and
nothing needs to change. Until then it is a correct design with no beneficiary.

---

### FC50 — The Name Is Preserved by the Callee and Discarded by the Door

`RegistryError::NameTaken` carries the colliding name, and its doc comment says
why: "so a caller reporting the conflict does not have to have kept it, and so
the message is specific without the caller formatting it."

`BuildError::NameTaken` carries nothing:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the registry preserves --'
command grep -A3 '  NameTaken$' ring_registry/src/lib.rs
echo '  -- what the door forwards --'
command grep -B4 '^  NameTaken,' ring_factory/src/lib.rs
```

Live output:

```
  -- what the registry preserves --
  NameTaken
  {
    /// The name that was already live.
    name : String,
  -- what the door forwards --
  /// Reachable only from [`Factory::build_named`], and only after the build
  /// half of that call already succeeded: the two variants partition by
  /// phase, so this one means a ring was built and then dropped because
  /// registration refused it, never that the build itself failed.
  NameTaken,
```

The source records the reason — "the caller passed it in and still has it.
Carrying it would also put a `String` in a `Copy` error type for no new
information" — and both halves of that are true. The consequence is still that
the two crates disagree about whether a caller can be expected to have kept the
name, four lines of source apart, and only one of them is on the export Contract.

The forced part is the `Copy`. `RegistryError` holds a `String` and is not
`Copy`; `BuildError` is `Copy` and 24 bytes ([`item/002`](../item/002_four_nouns_two_of_them_somebody_elses.md)
FC28). Adding the name to `BuildError` costs `Copy`, an allocation on a failure
path, and the niche that currently makes `NameTaken` free. That is a real trade
and the crate took the defensible side of it. What is missing is any note at the
`ring_registry` end that its most carefully-justified field does not survive the
one boundary consumers actually cross.

---

### Sources

| Source | What it establishes |
|--------|---------------------|
| `ring_registry/src/lib.rs` | `register`'s tuple error, and `NameTaken`'s name field with its justification |
| `ring_factory/src/lib.rs` | The `_refused` binding, and the fieldless `BuildError::NameTaken` it maps to |
| A workspace-wide scan of `Result< _, ( … ) >` | The tuple shape is unique in the 33 crates |

### Tests

| Test | What it holds |
|------|---------------|
| `a_refusal_drops_nothing_that_was_already_registered` | The refused build leaves the registry's existing contents intact |
| `a_refused_registration_leaves_the_original_ring_intact` | The ring already under the name is still usable after the collision |
| `a_removed_name_can_be_built_into_again` | The name, not the returned ring, is the reusable resource |
