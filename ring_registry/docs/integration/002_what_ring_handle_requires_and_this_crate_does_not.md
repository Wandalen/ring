# Integration: What `ring_handle` Requires and This Crate Does Not

### Scope

**Purpose:** Characterize the one declared edge by what actually crosses it —
zero method calls, six type positions — and show that the crate's most-cited
design conclusion depends on a fact about the other side that nothing checks.

**Responsibility:** Every `Split` occurrence in compiled library code; the two
methods `ring_handle` exposes on it; the `get_mut` justification and the decisions
entry resting on it; and whether the edge is acknowledged in the other direction.

**In Scope:** `ring_registry/src/lib.rs:47`, `:90`, `:161`, `:163`,
`:184-188`, `:189`, `:199`; `ring_handle/src/lib.rs:69`, `:76`, `:86`;
`ring_registry/docs/decisions/readme.md:56`.

**Out of Scope:** Which edges were declared and which survived is
[`integration/001`](001_one_declared_edge_of_three.md). The missing `T : Send`
bound is
[`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md).

---

## What Crosses the Edge

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every Split occurrence in compiled library code, doc lines excluded --'
command grep -v '^ *//[/!]' ring_registry/src/lib.rs | command grep 'Split' | sed 's/^/    /'
echo '  -- how many of those are method calls --'
printf '    Split:: or .ends( or any method call on a Split value: %s\n' \
  "$( command grep -v '^ *//[/!]' ring_registry/src/lib.rs | command grep -c 'Split::\|\.ends(' || true )"
echo '  -- every method the wrapped type exposes, and the receiver each takes --'
awk -v n1="$( command grep -n -m1 -F 'impl< T : Send > Split< T >' ring_handle/src/lib.rs | cut -d: -f1 )" -v n2="$( command grep -n -m1 -F '/// The two ends, before they are separated.' ring_handle/src/lib.rs | cut -d: -f1 )" 'NR >= n1 && NR <= n2 && ( /^impl/ || /fn / ) { print "    ring_handle:" NR ":" $0 }' ring_handle/src/lib.rs
echo '  -- the doc that depends on that surface, and the entry that depends on the doc --'
command grep -m1 -A4 -F '  /// **There is no immutable counterpart, and that is deliberate.**' ring_registry/src/lib.rs | sed 's/^/    /'
command grep 'Would become worth revisiting' ring_registry/docs/decisions/readme.md | cut -c1-92 | sed 's/^/    decisions:/'
echo '  -- and whether the edge is acknowledged from the other side --'
printf '    ring_registry named in ring_handle src or readme: %s\n' \
  "$( command grep -c 'ring_registry' ring_handle/src/lib.rs ring_handle/readme.md 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

Live output:

```
  -- every Split occurrence in compiled library code, doc lines excluded --
    use ring_handle::Split;
      rings : HashMap< String, Split< T > >,
        ring : Split< T >,
      -> Result< (), ( RegistryError, Split< T > ) >
      pub fn get_mut( &mut self, name : &str ) -> Option< &mut Split< T > >
      pub fn remove( &mut self, name : &str ) -> Option< Split< T > >
  -- how many of those are method calls --
    Split:: or .ends( or any method call on a Split value: 0
  -- every method the wrapped type exposes, and the receiver each takes --
    ring_handle:69:impl< T : Send > Split< T >
    ring_handle:76:  pub const fn new( ring : Ring< T > ) -> Self
    ring_handle:86:  pub fn ends( &mut self ) -> Ends< '_, T >
  -- the doc that depends on that surface, and the entry that depends on the doc --
      /// **There is no immutable counterpart, and that is deliberate.**
      /// `Split::ends` takes `&mut self`, so a `&Split< T >` can do nothing at all
      /// — an immutable `get` would be a method that compiles, returns something,
      /// and permits no operation. [`Self::contains`] is the immutable query that
      /// is actually answerable.
    decisions:Would become worth revisiting if `Split` ever grew a `&self` accessor.
  -- and whether the edge is acknowledged from the other side --
    ring_registry named in ring_handle src or readme: 0
```

---

### RG19 — The Edge Carries a Type and No Behaviour

`Split< T >` appears six times in the crate's compiled library code: the import,
the map's value type, `register`'s parameter, `register`'s error payload,
`get_mut`'s return, `remove`'s return. All six are type positions. The crate
calls **zero** methods on a `Split` — no `new`, no `ends`, nothing. The readme
says as much in prose — "the registry never inspects what it stores — it hashes a
name, holds a value, lends it back, and drops it" — and the census makes it
literal.

That is a stronger statement than the readme's. A crate that never inspects its
values could still construct or convert them; this one does not touch the type at
all. Everything `Registry` does works on `String` keys and on ownership. The one
operation that is genuinely about `Split` — dropping it, and with it the records
— is `Drop`, which the compiler writes.

So the declared dependency is satisfied entirely by naming the type. A
`Registry< V >` over any owned value would compile with the same body, the same
tests modulo their constructor, and no `ring_handle` in the manifest. What the
concrete type buys is not capability but domain meaning: the error payload is a
*ring*, the drop is *records*, and the crate's whole argument about the 448 bytes
and the hand-back contract is about a specific thing worth not destroying.

**Finding.** Recorded as a characterization the crate has not written down, not a
defect. `integration/001` establishes *which* edges exist; nothing says what
crosses this one. A sentence — that `ring_handle` is a type dependency, not a
behavioural one, and that the concreteness is chosen for meaning rather than
required by the code — is the fact a reader needs to judge whether the coupling is
worth it, and it is the fact that makes
[RG12](../data_structure/002_the_only_map_in_thirty_three_crates.md)'s missing
`T : Send` bound explicable: a crate that never calls a `Split` method never
meets the bound that guards those methods.

---

### RG20 — The Conclusion the Crate Repeats Most Rests on a Fact About Another Crate That Nothing Checks

"There is no immutable `get`" is stated three times: in `get_mut`'s own doc, in
the module doc's "Two things it deliberately does not do", and as Closed 3 in the
decisions file. Every statement of it reduces to the same premise —
`Split::ends` takes `&mut self`, so a `&Split< T >` permits no operation.

The premise is true. `Split`'s inherent impl has exactly two functions:
`new( ring : Ring< T > ) -> Self`, which takes no receiver, and
`ends( &mut self )`. There is no `&self` method to have.

It is also a fact about a different crate, and the decisions file knows it:
"Would become worth revisiting if `Split` ever grew a `&self` accessor." Nothing
would notice. `ring_registry`'s library calls no `Split` method, so no signature
change there breaks a compile here; its tests construct a `Split` and call
`ends`, which would keep working; and no test asserts anything about the shape of
`Split`'s surface. A `&self` accessor added upstream would leave this crate
compiling, passing, and documenting a design constraint that had stopped being
true. The edge is unacknowledged in that direction too — `ring_handle` names
`ring_registry` zero times, so a maintainer adding the accessor has nothing
pointing back.

**Finding.** Recorded as an unguarded cross-crate premise. The guard is cheap and
belongs in this crate's tests, because this is the crate that would be wrong: a
compile-fail expectation is heavy, but a one-line comment on
`registry_test.rs`'s `Split` usage naming the assumption, or a test that binds
`&Split< T >` and demonstrates it can do nothing, converts a claim into something
a reader can check without opening `ring_handle`. Worth doing precisely because
the conclusion is repeated three times — a premise cited that often should not be
verifiable only by reading another crate's source.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_one_declared_edge_of_three.md) | Which edges were declared and which survived |
| [`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md) | The `Send` bound this crate never meets, and why |
| [`api/001`](../api/001_the_registry_surface.md) | The `get_mut`-without-`get` shape this premise justifies |
| [`decisions/001`](../decisions/001_four_closed_questions_and_the_one_measurement_none_took.md) | Closed 3, settled by reading this signature |
| [`type/002`](../type/002_what_registry_derives_and_what_it_never_asserts.md) | The other property inferred through `Split` and never pinned |

### Sources

| Fact | Where |
|------|-------|
| Six type positions, zero method calls | Census above |
| `Split`'s two methods and their receivers | `ring_handle/src/lib.rs:69`, `:76`, `:86` |
| The `get_mut` justification | `ring_registry/src/lib.rs:184-188` |
| The revisit condition | `ring_registry/docs/decisions/readme.md:56` |
| The edge unacknowledged upstream | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_retrieved_ring_keeps_what_was_written_to_it` | The only test that calls a `Split` method |
| `a_registered_ring_is_retrievable_by_its_name_and_by_no_other` | `get_mut`, the method the premise shapes |
| `dropping_the_registry_drops_every_record_still_in_every_ring` | The one `Split` behaviour the crate relies on |
