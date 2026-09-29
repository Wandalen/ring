# Invariant: One Name, One Ring

### Scope

- **Purpose**: State the properties that hold of a registry at every moment, and what each one excludes.
- **Responsibility**: The invariants, their enforcement, and the consequences of a breach.
- **In Scope**: Name uniqueness, ownership exclusivity, and the accounting between them.
- **Out of Scope**: The operations that maintain them (→ [`api/001`](../api/001_the_registry_surface.md)); when ownership moves (→ [`lifecycle/001`](../lifecycle/001_a_ring_from_registration_to_drop.md)).

### Invariant Statement

| # | Invariant | Excludes |
|---|-----------|----------|
| R1 | Each live name maps to exactly one ring | Two rings answering to one name; a name resolving to nothing while `contains` says otherwise |
| R2 | Each registered ring is owned by exactly one registry | A ring reachable from two places, or dropped while still reachable |
| R3 | `len()` equals the number of live names, which equals the number of owned rings | A registry that has lost track of what it holds |

**R1 and R2 together are what "registry" means here**, and they are not the same
property. R1 is about the key space; R2 is about ownership. A structure could
satisfy R1 with `Rc< Ring >` values and violate R2 completely.

**R3 is the accounting that makes the other two checkable.** It is why the drop
test asserts `len() == 3` before dropping and `13` records after: without R3 a
test could observe the right number of drops from the wrong number of rings.

### Enforcement Mechanism

| # | Mechanism | Covers | Gap |
|---|-----------|--------|-----|
| E1 | `HashMap< String, Split< T > >` — one value per key | R1 | Only if the write path never replaces; see E2 |
| E2 | `Entry::Occupied` → refuse, `Entry::Vacant` → insert | R1's "exactly one", and R2 at the moment of collision | None known |
| E3 | `Split< T >` is not `Clone` | R2 | None — the type system carries it |
| E4 | `HashMap::len` is the only source of `len()` | R3 | None — there is no second counter to drift |
| E5 | `get_mut` returns `&mut Split< T >` | R1 is maintained — the name still resolves to one ring | Assignment through the returned borrow replaces the ring in place; nothing refuses it, unlike E2 |

**E2 is the load-bearing one and E1 alone is not sufficient.** `HashMap` holds
one value per key either way, but `insert` maintains R1 by *destroying* the
previous value — which satisfies R1 and violates the caller's expectation
completely (→ [the pitfall](../pitfall/001_insert_would_have_replaced_silently.md)).
The `Entry` match is what makes the uniqueness a refusal rather than a
replacement.

**E3 is free and worth naming.** `Split< T >` owns a `Ring< T >` and derives no
`Clone`, so R2 cannot be violated by a caller — there is no way to obtain a
second owner of a registered ring except by `remove`, which transfers rather than
duplicates. The invariant is enforced by the type rather than by this crate.

**E4 has no drift because there is no second number.** `len()` forwards to the
map. A registry that maintained its own counter alongside would need R3 to be
tested; as written, R3 is true by construction and the test asserting it is
really asserting that nothing was silently dropped on the way in.

### Violation Consequences

| # | Violation | Detected by | Consequence |
|---|-----------|-------------|-------------|
| V1 | A registration replaces instead of refusing (R1 maintained, expectation broken) | `a_refused_registration_does_not_drop_the_ring_already_there` | The previously registered ring, and every unread record in it, is destroyed by a naming mistake |
| V2 | A refused registration mutates the map (R3) | `a_second_registration_under_a_live_name_is_refused` asserts `len() == 1` | The registry's count no longer matches what it holds |
| V3 | A dropped registry leaves a ring alive (R2) | `dropping_the_registry_drops_every_record_still_in_every_ring` | A leak — and one no compiler warning would show, since it is not a borrow error |
| V4 | A removed ring is dropped by the registry anyway (R2) | `a_removed_ring_carries_its_records_to_its_new_owner` | A double-drop, or a drop of something the caller still holds |

**V1 is the one that looks correct from the outside.** A registry that replaces
silently satisfies R1, satisfies R3, passes every count-based assertion, and
loses data. It is caught only by a drop counter — which is why this document
asks for one, not an external criterion; see the Sources row below.

**V4 is the mirror of V3** and is the reason `remove` returns the `Split< T >`
rather than a borrow: ownership must actually leave, and the test asserts it by
dropping the registry first and observing zero drops.

---

## The Third Write Path, and the Specification That Does Not Exist

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order. The pattern against this
# file is anchored at column 0 and line numbers are omitted for it: the findings
# below quote these same rows, and would otherwise match themselves.
i=ring_registry/docs/invariant/001_one_name_one_ring.md
echo '  -- what the enforcement table claims is left uncovered --'
command grep -h '^| E' "$i" |
  awk -F'|' '{ gsub( /^ +| +$/, "", $2 ); gsub( /^ +| +$/, "", $5 ); printf "    %-3s gap: %s\n", $2, $5 }'
echo '  -- every declaration, and whether it reaches the map --'
awk '
  /^  pub fn/ { n = $3; sub( /\(.*/, "", n ); buf = $0; inbuf = 1; next }
  inbuf && /^  \{/ {
    r = index( buf, "&mut self" ) ? "&mut self   -- reaches the map" :
        index( buf, "&self" )     ? "&self       -- cannot write" :
                                    "no receiver -- makes a new one"
    printf "    %-10s %s\n", n, r; inbuf = 0; next }
  inbuf { buf = buf " " $0 }
' ring_registry/src/lib.rs
echo '  -- every Registry::get_mut call site in the workspace --'
for t in ring_registry/tests/registry_test.rs ring_factory/tests/factory_test.rs
do
  command grep -n 'registry\.get_mut' "$t" | cut -c1-70 | sed "s|^|    ${t##*/}:|" | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
done
echo '  -- the specification the V1 note and the Sources row attribute these invariants to --'
f=docs/feature/181_named_ring_registry.md
printf '    %s\n      %s lines; occurrences of criteri/drop/invariant/MUST/shall: %s\n' \
  "$f" "$( wc -l < "$f" )" "$( command grep -c 'criteri\|[Dd]rop\|invariant\|MUST\|shall' "$f" || true )"
echo '      every heading it carries:'
command grep -h '^#' "$f" | sed 's/^/        /'
echo '  -- the corrected V1 note, naming this document as the source instead --'
command grep -h '^asks for one, not an external criterion' "$i" | sed 's/^/    /'
```

Live output:

```
  -- what the enforcement table claims is left uncovered --
    E1  gap: Only if the write path never replaces; see E2
    E2  gap: None known
    E3  gap: None — the type system carries it
    E4  gap: None — there is no second counter to drift
    E5  gap: Assignment through the returned borrow replaces the ring in place; nothing refuses it, unlike E2
  -- every declaration, and whether it reaches the map --
    new        no receiver -- makes a new one
    register   &mut self   -- reaches the map
    get_mut    &mut self   -- reaches the map
    remove     &mut self   -- reaches the map
    contains   &self       -- cannot write
    len        &self       -- cannot write
    is_empty   &self       -- cannot write
    names      &self       -- cannot write
  -- every Registry::get_mut call site in the workspace --
    registry_test.rs:  assert!( registry.get_mut( "events" ).is_some() );
    registry_test.rs:    assert!( registry.get_mut( wrong ).is_none(), "{wrong:?} retrie
    registry_test.rs:    let split = registry.get_mut( name ).expect( "registered" );
    registry_test.rs:    let split = registry.get_mut( "events" ).expect( "registered" )
    registry_test.rs:  let split = registry.get_mut( "events" ).expect( "still register
    registry_test.rs:  *registry.get_mut( "events" ).unwrap() = ring::< Counted >( 16 )
    factory_test.rs:  assert!( registry.get_mut( "events" ).is_some() );
    factory_test.rs:  assert!( registry.get_mut( "telemetry" ).is_none() );
    factory_test.rs:    let held = registry.get_mut( "events" ).expect( "just register
    factory_test.rs:    let held = registry.get_mut( "events" ).expect( "just register
    factory_test.rs:  let held = registry.get_mut( "events" ).expect( "still registere
  -- the specification the V1 note and the Sources row attribute these invariants to --
    docs/feature/181_named_ring_registry.md
      30 lines; occurrences of criteri/drop/invariant/MUST/shall: 0
      every heading it carries:
        # Feature: Named Ring Registry
        ## Definition
        ## If Missing
        ### Hard Problems
        ### Workstreams
        ### Sources
  -- the corrected V1 note, naming this document as the source instead --
    asks for one, not an external criterion; see the Sources row below.
```

What a `&mut Split< T >` permits, measured with the same drop counter the V1 test
uses, against a ring loaded exactly as `dropping_the_registry_drops_every_record_still_in_every_ring`
loads one:

```rust
let mut registry = Registry::new();
registry.register( "events", loaded( 6 ) ).unwrap();

// The replacement ring is built before the window so its own construction is
// not billed to the assignment.
let fresh = loaded( 2 );

// One statement. No refusal, no return value, no `remove`.
*registry.get_mut( "events" ).unwrap() = fresh;

// And the other spelling, which hands the displaced ring back instead of
// destroying it.
let displaced = core::mem::replace( registry.get_mut( "events" ).unwrap(), loaded( 3 ) );
```

Its output, identical on two runs:

```
    registered under "events": 6 unread records, drops so far 0
    after `*registry.get_mut( "events" ).unwrap() = fresh;`
      drops 6   len 1   contains("events") true
    after `core::mem::replace( registry.get_mut( "events" ).unwrap(), .. )`
      drops 6   len 1   the displaced ring is in the caller's hand
    after dropping both: drops 11   (6 replaced + 2 displaced + 3 held)
```

---

### RG21 — The Replacement E2 Exists to Prevent Is One Statement Away Through `get_mut`

E1's Gap column concedes the map alone is not enough — "Only if the write path
never replaces; see E2" — and E2 answers it with the `Entry` match and a Gap of
**None known**. The reasoning is exact for the write path it has in mind, and
[the pitfall](../pitfall/001_insert_would_have_replaced_silently.md) spends a
whole instance on why `insert` was the wrong call.

There are three write paths. `register`, `get_mut` and `remove` all take
`&mut self`; the four reads take `&self` and cannot touch the map. E2 covers one
of the three, `remove` is a deliberate transfer, and `get_mut` is unexamined —
yet a `&mut Split< T >` is an assignable place. `*registry.get_mut( "events" ).unwrap() = fresh;`
drops the registered ring where it lies: **six unread records destroyed, `len`
still 1, `contains` still true**, no refusal, no return value, nothing to ignore.
That is `insert`'s semantics precisely — R1 maintained, the caller's expectation
broken — which is V1, arriving through the one door the enforcement table does
not watch.

The second spelling is worse for E3 and better for the caller.
`core::mem::replace` through the same `&mut` hands the displaced ring back
intact, so ownership of a *registered* ring leaves the registry without
`remove` — and E3's note says there is no way to do that "except by `remove`,
which transfers rather than duplicates." `mem::replace` also transfers rather
than duplicates. R2 survives, because at every instant exactly one owner exists;
the enumeration of how ownership moves does not.

**Finding.** Recorded as a **latent hazard**: the destructive form is reachable
in one statement from the public surface, and nothing in the crate names it. All
ten `Registry::get_mut` call sites in the workspace — five in this crate's tests,
five in `ring_factory`'s — either test `is_some`/`is_none` or bind the result and
call `.ends()` on it; not one assigns through the borrow, so the hazard is
entirely unexercised as well as entirely undocumented. Two repairs, both small:
add an E5 row for `get_mut` stating that R1 is maintained but replacement is
possible through the returned borrow, and add a test that assigns through it and
asserts the drop count — the same counter V1 already uses, pointed at the path it
does not cover. Compare [RG7](../api/002_the_receiver_split_and_the_sweep_it_forbids.md),
which reaches the same three-writers/four-readers split from the type side; this
is what that split costs when only one writer is guarded.

```sh
cd "$(git rev-parse --show-toplevel)"
cargo test -p ring_registry --all-features assigning_through_get_mut_drops_the_ring_it_replaces -- --nocapture 2>&1 | command grep -E 'assigning_through_get_mut|test result'
```

Live output:

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test assigning_through_get_mut_drops_the_ring_it_replaces ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 14 filtered out; finished in 0.00s
```

**Disposition:** applied — both repairs. The Enforcement Mechanism table now
carries an E5 row for `get_mut`, stating R1 is maintained while the returned
borrow permits an unrefused replacement, unlike E2. A new test,
`assigning_through_get_mut_drops_the_ring_it_replaces`
(`tests/registry_test.rs`), registers a ring holding 5 unread records, assigns
a fresh ring through `get_mut`'s borrow, and asserts the same drop counter V1
uses reports exactly 5 — the eleventh `get_mut` call site in the workspace and
the first to exercise the path this finding names. Full crate suite
re-verified passing (`cargo test --all-features -p ring_registry`,
2026-09-04): 15 integration tests, 1 doctest, 0 failures. Now prints:
`test assigning_through_get_mut_drops_the_ring_it_replaces ... ok`

---

### RG22 — Three Invariants Attributed to an Acceptance Criterion That Was Never Written

The V1 note closes by grounding the drop counter in an upstream requirement — it
is caught only by a drop counter, "which is why the acceptance criterion asks for
one" — and the Sources table cites the same feature for "R1's first two clauses
and R2's third, as the acceptance criterion states them."

There is no acceptance criterion. `docs/feature/181_named_ring_registry.md` is 30
lines carrying six headings — Definition, If Missing, Hard Problems, Workstreams,
Sources — and the words *criterion*, *drop*, *invariant*, *MUST* and *shall*
appear in it zero times. It has no numbered clauses, so "R1's first two clauses
and R2's third" has nothing to number against. What it does contain is one
property-shaped phrase in its Definition — rings "none reachable except through
the registry that owns it" — which is R2 in other words, and is the whole of the
upstream property content. R1 and R3 have no source at all outside this file.

The attribution runs the wrong way. R1, R2 and R3 were not read off a
specification and implemented; they were discovered by implementing, and this
document is where they are first stated. That is the more valuable thing to be —
the invariant set is sharper than anything the feature asks for, and R3 in
particular ("the accounting that makes the other two checkable") is an insight,
not a requirement. Presenting them as ratified upstream hides that, and it makes
the citation unfollowable: a reader who opens feature 181 to check the wording of
a clause finds two paragraphs of motivation.

**Finding.** Recorded as a **wrong doc**. The repair is to say where each
property came from: R2 restates the feature's "none reachable except through the
registry that owns it"; R1 and R3 are this crate's, proposed here. The V1 note
then reads as it should — the drop counter is asked for by *this* document,
because a count-based assertion cannot see a silent replacement, which is a
better reason than an imaginary criterion. The corpus-wide habit this is an
instance of matters too: 402 of the 426 feature files are `Status: planned`, so
citing one as a settled criterion is a mistake that scales.

**Disposition:** applied — the V1 note (`:64-65`) now says the drop counter "is
why this document asks for one, not an external criterion", and the Sources row
(`:323`) now attributes R1–R3 to this document itself, naming no external file
at all. Now prints: `not an external criterion`

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_registry_surface.md](../api/001_the_registry_surface.md) | A1–A6 — the per-call guarantees these standing properties come from |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_ring_from_registration_to_drop.md](../lifecycle/001_a_ring_from_registration_to_drop.md) | R2 over time — where ownership sits at each moment |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_insert_would_have_replaced_silently.md](../pitfall/001_insert_would_have_replaced_silently.md) | V1 — why E2 is `Entry` and not `insert` |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_registry_error.md](../type/001_registry_error.md) | What E2 returns when it refuses |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | E1–E4 |
| This document | R1, R2 and R3 were discovered by implementing this crate, not sourced from any external document — see RG22 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/registry_test.rs` | V1 — `a_refused_registration_does_not_drop_the_ring_already_there`; V2 — `a_second_registration_under_a_live_name_is_refused`; V3 — `dropping_the_registry_drops_every_record_still_in_every_ring`; V4 — `a_removed_ring_carries_its_records_to_its_new_owner` |
