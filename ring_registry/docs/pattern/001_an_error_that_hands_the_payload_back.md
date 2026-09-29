# Pattern: An Error That Hands the Payload Back

### Scope

**Purpose:** Place `register`'s `Result< (), ( RegistryError, Split< T > ) >`
against the six other signatures in the family that return the caller's own value
on failure, and record that the family spells one contract three ways.

**Responsibility:** The seven hand-back signatures; the three spellings; what
`ring_shutdown::Refusal< T >` provides that a bare tuple cannot; and which of the
three cite each other.

**In Scope:** `ring_registry/src/lib.rs:163`;
`ring_shutdown/src/lib.rs:330-388`, `:325`; `ring_core/src/lib.rs:373`;
`ring_handle/src/lib.rs:138`; `ring_spsc/src/lib.rs:713`;
`ring_poll/src/lib.rs:286`, `:599`.

**Out of Scope:** The `Error` impl that this signature bypasses is
[`pattern/002`](002_an_error_impl_the_fallible_method_cannot_reach.md). The width
the tuple costs is
[`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md).
Why the payload comes back at all is
[`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md).

---

## Seven Signatures, Three Spellings

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order; -e is required
# because the pattern begins with a hyphen
echo '  -- every signature in the family that hands the payload back on failure --'
command grep -r -e '-> *Result< (), T >' -e '-> *Result< (), Refusal' -e '-> *Result< (), ( ' --include=lib.rs ring_*/src/ | sed 's|ring/||' | sed 's/^/    /'
echo '  -- the three spellings --'
printf '    bare payload: %s sites   named enum: %s   error-payload tuple: %s\n' \
  "$( command grep -rc -e '-> *Result< (), T >' --include=lib.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -rc -e '-> *Result< (), Refusal' --include=lib.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -rc -e '-> *Result< (), ( ' --include=lib.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
echo '  -- what the named one carries, and what it cites --'
command grep -e 'pub enum Refusal' -e 'pub fn into_record' -e 'pub const fn is_closed' -e 'own refusal contract' ring_shutdown/src/lib.rs | sed 's/^/    /'
echo '  -- and whether the tuple form cites anything --'
printf '    Refusal< or ring_shutdown named in ring_registry src and readme: %s\n' \
  "$( command grep -c -e 'Refusal<' -e 'ring_shutdown' ring_registry/src/lib.rs ring_registry/readme.md 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
printf '    accessor methods on the tuple: %s\n' \
  "$( command grep -c 'impl.*for *( RegistryError' ring_registry/src/lib.rs || true )"
```

Live output:

```
  -- every signature in the family that hands the payload back on failure --
    ring_core/src/lib.rs:  pub fn try_push( &mut self, record : T ) -> Result< (), T >
    ring_flush/src/lib.rs:  /// is `ring_tls`'s (a widened `push( item : T ) -> Result< (), ( RingError,
    ring_handle/src/lib.rs:  pub fn try_push( &mut self, record : T ) -> Result< (), T >
    ring_poll/src/lib.rs:-> Result< (), T >
    ring_poll/src/lib.rs:  -> Result< (), T >
    ring_registry/src/lib.rs:  -> Result< (), ( RegistryError, Split< T > ) >
    ring_shutdown/src/lib.rs:  pub fn try_push( &mut self, record : T ) -> Result< (), Refusal< T > >
    ring_spsc/src/lib.rs:  pub fn try_push( &mut self, record : T ) -> Result< (), T >
  -- the three spellings --
    bare payload: 5 sites   named enum: 1   error-payload tuple: 2
  -- what the named one carries, and what it cites --
    /// intact in both — `ring_core`'s own refusal contract, extended by one case.
    pub enum Refusal< T >
      pub fn into_record( self ) -> T
      pub const fn is_closed( &self ) -> bool
  -- and whether the tuple form cites anything --
    Refusal< or ring_shutdown named in ring_registry src and readme: 0
    accessor methods on the tuple: 0
```

---

### RG37 — One Contract, Three Spellings, and Only One of Them Cites Another

Seven public signatures in the family return the caller's own value on failure
rather than destroying it. They agree on the contract completely — nothing is
consumed by a refusal — and disagree on how to say it three ways:

- **Bare payload**, `Result< (), T >`, at five sites in four crates. The error
  *is* the value; there is exactly one way to fail, so no reason needs carrying.
- **Named enum**, `Result< (), Refusal< T > >`, once, in `ring_shutdown`. Two
  ways to fail, the record in each arm.
- **Error-payload tuple**, `Result< (), ( RegistryError, Split< T > ) >`, once,
  here.

The split is not arbitrary — it tracks how many failure reasons each method has,
which is exactly the right axis. What is arbitrary is that the two methods with
more than one thing to say picked different shapes for saying it, and only one of
them looked at the family first. `ring_shutdown` writes its derivation out,
directly above the enum: it exists "because the record must come back
intact in both — `ring_core`'s own refusal contract, extended by one case."
This crate's source and readme name `Refusal` and `ring_shutdown` zero times
between them.

**Finding.** Recorded as an undocumented divergence rather than a wrong choice.
The tuple is defensible: `RegistryError` has one variant today and an enum
wrapping a single case would be ceremony. What is missing is the sentence saying
so, next to the signature, in a crate that writes twenty lines about the same
signature's width — because a reader who has seen `Refusal< T >` will assume this
crate had not, and on the current evidence they are right.

---

### RG38 — The Named Form Has Four Things the Tuple Does Not, and One of Them Is Needed Here

`Refusal< T >` is what the tuple would be if it were finished. It carries
`into_record( self ) -> T`, so a caller gets the payload out by name;
`is_closed( &self ) -> bool`, `#[ must_use ]` and `const`, so a caller
distinguishes the arms without matching; a conversion to the family's `RingError`
for callers that report rather than retry; and `Debug, Clone, Copy, PartialEq,
Eq` on the type itself.

The tuple has none of these — the census finds zero impls on it, which is
unavoidable, since `( RegistryError, Split< T > )` is a foreign type here in
every sense that matters and no inherent impl can be written for it. So the
recovery a caller wants is positional:

```rust
match registry.register( "events", ring )
{
  Err( ( _, ring ) ) => { /* the ring is back, by position */ },
  Ok( () ) => {},
}
```

One of the four gaps is not cosmetic. There is no way to inspect the error and
keep the ring: matching on `Err( ( error, ring ) )` moves both out, and borrowing
the `Err` gives `&( RegistryError, Split< T > )`, from which the ring cannot be
taken. A caller that wants to log the name and then retry under a different name
must destructure first and reconstruct the message from the pieces, or clone the
error. `RegistryError` does derive `Clone`, and nothing in the crate uses it —
the only `.clone()` in the source is the `String` key of
[RG1](../algorithm/001_two_branches_and_what_the_refusal_costs.md).

**Finding.** Recorded as an ergonomics gap with one concrete consequence, not a
correctness problem. The minimal repair is an inherent `into_ring` on a named
wrapper; the cheaper one, if the tuple stays, is to say in `register`'s doc that
the error and the ring come out together and that inspecting one costs the other.
Worth noting what the derive table already says: `Clone` is justified there
because "an error a caller wants to keep alongside a log line should not have to
be re-created", which is this exact situation described without the tuple being
named as what causes it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/002`](002_an_error_impl_the_fallible_method_cannot_reach.md) | What the tuple costs at the `?` operator |
| [`type/001`](../type/001_registry_error.md) | The derive table, and the `Clone` this finding re-reads |
| [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md) | The 448 bytes the tuple makes the `Result` |
| [`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md) | Why the payload comes back at all |
| [`integration/001`](../integration/001_one_declared_edge_of_three.md) | The `Split< T >` the tuple's second slot holds |

### Sources

| Fact | Where |
|------|-------|
| The tuple signature | `ring_registry/src/lib.rs:163` |
| Five bare-payload sites | `ring_core:373`, `ring_handle:138`, `ring_poll:286`, `:599`, `ring_spsc:715` |
| The named enum and its derivation | `ring_shutdown/src/lib.rs:330`, `:329` |
| `into_record` and `is_closed` | `ring_shutdown/src/lib.rs:349`, `:370` |
| Zero citations of either precedent here | Census above |
| Zero impls on the tuple | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_refused_registration_hands_the_ring_back` | The contract all seven signatures share |
| `the_error_names_the_taken_name` | The first slot of the tuple |
| `a_refused_registration_does_not_drop_the_ring_already_there` | What the pattern exists to prevent |
