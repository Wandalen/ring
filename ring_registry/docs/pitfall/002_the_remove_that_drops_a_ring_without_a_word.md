# Pitfall: The `remove` That Drops a Ring Without a Word

### Scope

**Purpose:** Show that F2, the `remove` hazard `pitfall/001` records as averted by
a signature change, is still reachable — and measure it producing the exact
outcome P2 predicts.

**Responsibility:** What `pitfall/001` claims the `Option< Split< T > >` return
bought; what `registry.remove( "events" );` as a statement actually does; how
`remove`'s own doc describes itself against `register`'s; and why no test in the
suite can see it.

**In Scope:** `ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md:68`,
`:73`, `:76`, `:89`; `ring_registry/src/lib.rs:194-199`;
`ring_registry/tests/registry_test.rs:241`, `:255`, `:284`, `:323`.

**Out of Scope:** The missing attribute that would warn at the call site is
[`item/002`](../item/002_eight_declarations_and_four_must_use.md). The invariant
`remove` is allowed to break is
[`invariant/001`](../invariant/001_one_name_one_ring.md).

---

## What F2 Claims, and What `remove` Says About Itself

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
p=ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md
echo '  -- how pitfall/001 records the remove hazard, and why it treats it as averted --'
# Anchored at column 0: `pitfall/001` quotes its own F rows inside indented
# regenerate output further down, and an unanchored search finds those copies.
command grep -n '^| F2 | .remove.\|^| P2 | .remove.\|^\*\*F2 is the one that was nearly written\|^signature returning' "$p" | cut -c1-92 | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- everything remove says about itself --'
command grep -m1 -A5 -F '  /// Take the ring registered under a name, freeing the name.' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- destructive words in each method own doc --'
printf '    register  drop/destroy/lose/unread: %s\n' \
  "$( command grep -m1 -A32 -F '  /// Register a ring under a name, taking ownership of it.' ring_registry/src/lib.rs | command grep -c -i -e 'drop\|destroy\|lose\|unread' || true )"
printf '    remove    drop/destroy/lose/unread: %s\n' \
  "$( command grep -m1 -A4 -F '  /// Take the ring registered under a name, freeing the name.' ring_registry/src/lib.rs | command grep -c -i -e 'drop\|destroy\|lose\|unread' || true )"
echo '  -- every remove call site in the tests, and what each does with the value --'
command grep -n 'registry.remove(' ring_registry/tests/registry_test.rs | cut -c1-88 | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- how pitfall/001 records the remove hazard, and why it treats it as averted --
    | F2 | `remove` | Returning `bool` instead of `Option< Split< T > >` — makes recovery *
    **F2 is the one that was nearly written.** `remove( &str ) -> bool` reads
    signature returning `Option< Split< T > >` is what makes
    | P2 | `remove` returns `bool` | Nothing at the call site; the records are dropped by the
  -- everything remove says about itself --
      /// Take the ring registered under a name, freeing the name.
      ///
      /// This is what makes "a name that is taken" a temporary condition rather
      /// than a permanent one, and therefore what makes [`Self::register`]'s
      /// refusal recoverable rather than final.
      pub fn remove( &mut self, name : &str ) -> Option< Split< T > >
  -- destructive words in each method own doc --
    register  drop/destroy/lose/unread: 4
    remove    drop/destroy/lose/unread: 0
  -- every remove call site in the tests, and what each does with the value --
      let taken = registry.remove( "events" ).expect( "registered" );
      assert!( registry.remove( "nothing" ).is_none() );
      let taken = registry.remove( "events" ).expect( "registered" );
      registry.remove( "b" ).expect( "registered" );
```

## F2, Reproduced Against the Shipped Signature

A ring with six unread records, registered, then removed as a bare statement.
Records are `Counted`, which increments a static on `Drop`:

```rust
// src/bin/silent_remove.rs
let mut registry = Registry::new();
registry.register( "events", filled( 6 ) ).unwrap();
println!( "    registered with 6 unread records; drops so far {}", DROPS.load( Ordering::SeqCst ) );

// The whole statement. It compiles with no warning under -D warnings.
registry.remove( "events" );

println!( "    after `registry.remove( \"events\" );` as a statement: drops {}", DROPS.load( Ordering::SeqCst ) );
println!( "    the registry now reports len {} and contains(\"events\") {}", registry.len(), registry.contains( "events" ) );
```

```
    registered with 6 unread records; drops so far 0
    after `registry.remove( "events" );` as a statement: drops 6
    the registry now reports len 0 and contains("events") false
```

---

### RG43 — F2 Is Recorded as Averted by the Signature, and the Signature Does Not Avert It

`pitfall/001` enumerates four places the crate's central hazard could appear.
F2 is `remove`, and the document names it as the near miss: "**F2 is the one that
was nearly written.** `remove( &str ) -> bool` reads naturally, matches
`contains`, and is what a set-like API would offer. It also means the caller
cannot get their ring back, and the records go with it." The remedy is recorded
as the signature: returning `Option< Split< T > >` "is what makes
`a_removed_ring_carries_its_records_to_its_new_owner` expressible at all."

The signature makes the good path *expressible*. It does not make the bad path
*unreachable*. `registry.remove( "events" );` written as a whole statement
compiles with no warning under `-D warnings`, and measured against six unread
records it destroys all six, leaving `len 0` and `contains("events") false` — the
count-invisible destruction that is this pitfall's entire subject. P2's own
prediction of how F2 presents, "Nothing at the call site", is still exactly
correct; only the mechanism moved, from the registry dropping the ring internally
to the caller's semicolon dropping it externally.

The two routes are not equally dangerous, and the difference favours the crate:
F1, the `insert` route, was a hazard a maintainer could reintroduce silently, and
`Entry` closed it permanently. F2 is a hazard a caller reaches, once, on purpose,
by writing a statement that looks like a command rather than a query. But
`pitfall/001` presents F2 as handled and F1 as the live concern, and on the
evidence it is the other way round.

**Finding.** Recorded as a hazard mis-recorded as closed. The repair is in
`pitfall/001` rather than in the code: F2's remedy column should say the
signature makes recovery *possible*, and name what still makes discarding it
easy. The code-side fix is one line and belongs to
[RG27](../item/002_eight_declarations_and_four_must_use.md), which prescribes it
from the attribute side; this finding is why it is worth doing.

**Disposition:** applied — `pitfall/001...md`'s F2 row (`:68`) now reads "makes
recovery *possible*; a bare `registry.remove( name );` still discards it, with
no warning", replacing the "the ring is dropped by the registry, not handed
over" framing that read as the hazard already closed. Now prints: `makes recovery`

---

### RG44 — `register` Documents the Destruction It Refuses; `remove` Documents Nothing About the One It Performs

`register`'s doc opens on the hazard: "**Refuses rather than replaces.** A
registry that silently replaced would drop the previous ring — and with it every
record still unread in that ring — as a side effect of a name collision." Four
occurrences of *drop*, *destroy*, *lose* or *unread* across its thirty-three
lines, all of them warning a reader what is at stake.

`remove`'s doc is five lines and contains none. Both of its sentences are about
what the method enables — freeing a name, making "a name that is taken" a
temporary condition, making `register`'s refusal "recoverable rather than final".
Every word is true. Not one of them tells a reader that the value being returned
is the only remaining owner of a buffer full of records.

The test suite mirrors the asymmetry and therefore cannot correct it. All four
`remove` call sites in the tests bind the value or assert on it —
`let taken = registry.remove( .. ).expect( .. )`, `assert!( .. .is_none() )`,
`registry.remove( "b" ).expect( .. )`. `a_removed_ring_carries_its_records_to_its_new_owner`
does carry a drop counter and does assert 0 then 5, which proves the records
survive **when the value is held**. Nothing exercises the discarded shape, so the
suite's own guard against this pitfall covers F1's route and not F2's — the same
guard-quality question `pitfall/001` raises as P3 about empty drop-counter tests,
asked of one route and not the other.

**Finding.** Recorded as a documentation and coverage asymmetry between two
methods with symmetric destructive power. Two sentences on `remove` — that it
hands over the last owner of the ring, and that discarding the return destroys it
— bring its doc to `register`'s standard. One test that calls `remove` as a bare
statement and asserts the drop count is the guard the suite is missing; it would
have failed on the day the attribute was left off, and it is the test that makes
RG43's claim regress-proof rather than merely recorded.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](001_insert_would_have_replaced_silently.md) | F2 and P2, the entries this instance tests |
| [`item/002`](../item/002_eight_declarations_and_four_must_use.md) | The one-line fix, from the attribute side |
| [`lifecycle/001`](../lifecycle/001_a_ring_from_registration_to_drop.md) | The drop this method performs early |
| [`invariant/001`](../invariant/001_one_name_one_ring.md) | The invariant `remove` is permitted to end |
| [`api/001`](../api/001_the_registry_surface.md) | `remove` among the eight |

### Sources

| Fact | Where |
|------|-------|
| F2 and its remedy | `.../pitfall/001_insert_would_have_replaced_silently.md:68`, `:73`, `:76` |
| P2's prediction | `.../pitfall/001_insert_would_have_replaced_silently.md:89` |
| `remove`'s complete doc | `ring_registry/src/lib.rs:194-199` |
| 4 destructive words on `register`, 0 on `remove` | Census above |
| Six records destroyed, `len 0`, `contains false` | Probe above |
| Four bound `remove` call sites in the tests | `ring_registry/tests/registry_test.rs:241`, `:255`, `:284`, `:323` |

### Tests

| Test | Covers |
|------|--------|
| `a_removed_ring_carries_its_records_to_its_new_owner` | The bound path, with a drop counter |
| `removing_a_name_frees_it_for_reuse` | The capability `remove`'s doc is entirely about |
| `removing_an_absent_name_is_none` | The `None` arm, where nothing is destroyed |
