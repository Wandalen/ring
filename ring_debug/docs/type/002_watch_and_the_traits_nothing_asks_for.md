# Type: Watch, and the Traits Nothing Asks For

### Scope

- **Purpose**: Define the crate's third public type — the only one that holds state — and price the trait surface all three of them share.
- **Responsibility**: `Watch`'s fields, its construction contract, and what each derived or implemented trait on the crate's public types is actually used for.
- **In Scope**: `Watch`; `Copy` on a value with a `&mut self` method; the derive sets on all three types; the `Error` impl.
- **Out of Scope**: `Violation` and `Cursor` as reported values (→ [`type/001`](001_violation.md)); when a watch is started and how long it lives (→ [`lifecycle/001`](../lifecycle/001_from_one_observation_to_a_sequence.md)).

### Definition

```rust
pub struct Watch
{
  producer : Seq,
  consumer : Seq,
  capacity : Capacity,
}

impl Watch
{
  pub fn new( pair : &CursorPair ) -> Result< Self, Violation >;
  pub fn observe( &mut self, pair : &CursorPair ) -> Result< (), Violation >;
  pub fn last( &self ) -> ( Seq, Seq );
}
```

[`type/001`](001_violation.md) covers the two types a check *reports*. This is the
one it *runs on*, and it is a different kind of thing: three private fields, a
constructor that can refuse, and a method that takes `&mut self` because observing
advances a baseline.

That baseline is the crate's entire contribution beyond `check`. D3 — a cursor
went backwards — is a statement about two moments, and nothing in a `CursorPair`
remembers the first one. The `Watch` is where the first moment lives.

#### The derive line is the same on all three

| Type | Derives | Role |
|---|---|---|
| `Cursor` | `Debug, Clone, Copy, PartialEq, Eq, Hash` | A two-variant label, inert |
| `Violation` | `Debug, Clone, Copy, PartialEq, Eq` | A report, inert |
| `Watch` | `Debug, Clone, Copy, PartialEq, Eq` | An instrument, stateful |

[`type/001`](001_violation.md)'s derive table justifies `Copy` on the first two:
*"every field is `Copy`; a diagnostic that borrowed would be awkward in exactly the
contexts diagnostics are used."* Both halves of that are true of a report and
neither is true of an instrument. `Watch` is not a diagnostic; it is the thing that
produces one, and its fields being `Copy` is a fact about `Seq`, not an argument.

#### `Copy` on a value with a `&mut self` method

`observe` takes `&mut self` and rewrites `producer` and `consumer` in place. That
receiver is the mechanism: the baseline advances because the caller's own value is
mutated.

`Copy` removes the compiler's ability to enforce that there is one such value. A
helper written as

```rust
fn drain_and_check( mut watch : Watch, pair : &CursorPair ) -> Result< (), Violation >
```

compiles, observes against a private duplicate, and leaves the caller's baseline
exactly where it was — with no move error at the caller's next use, because there
was no move. Every subsequent `observe` in the caller compares against a stale
first moment, D3 goes unchecked for the rest of the run, and the failure mode is
silence: a `Watch` that returns `Ok` forever.

Without `Copy`, that helper is a compile error the second time the caller touches
its watch. **The derive deletes the diagnostic for the one mistake this type can
make.** Nothing in the tree binds a `Watch` by value today, so the cost is
currently zero and so is the benefit.

#### The rest of the surface, priced

| Trait | On | Used by |
|---|---|---|
| `Debug` | All three | API convention; `assert_eq!` needs it on `Violation`, not on `Watch` |
| `Clone`, `Copy` | All three | Right for the two reports; see above for `Watch` |
| `PartialEq`, `Eq` | All three | The assertion shape — on `Violation`. Two `Watch` values are compared nowhere |
| `Hash` | `Cursor` | Nothing — recorded as such in [`type/001`](001_violation.md) |
| `Error` | `Violation` | `check( pair )?` in a caller returning `Box< dyn Error >` |

`impl core::error::Error for Violation {}` has an empty body, so `source` is the
default `None`. For this type that is correct rather than lazy: a `Violation` is a
leaf — it is derived from two integers, not wrapped around an underlying cause, and
there is nothing to return. The observation is that the family gives no way to tell
that apart from the alternative. Seven `ring_*` crates implement `Error` and none
of them defines `fn source`, so a correct `None` and an unconsidered one look
identical from outside.

### Validation

**`Watch` is the crate's only type that can refuse to be built.** `new` returns
`Result< Self, Violation >` and runs the stateless checks before adopting a
baseline, so a watch cannot come to exist over a pair that was already broken.

This is the exact inverse of [`type/001`](001_violation.md)'s position on
`Violation`, and both are right for their subject. A report of a broken invariant
must be constructible from impossible numbers — that is the case it exists to
describe. An instrument that measures against a baseline must not be, because a
broken baseline is not a measurement, it is a permanently wrong answer:
`CursorWentBackwards` would then be reported relative to a state the ring should
never have been in, and the stateless violations would be adopted rather than
raised.

The third field, `capacity`, is the caller's number kept for the life of the watch.
It is validated once by `Capacity`'s own constructor and never re-checked against
the ring, which is the same externally-supplied term
[`invariant/002`](../invariant/002_the_conservation_law_and_why_it_holds.md)
identifies in D4.

### Errors

`observe` returns `Result< (), Violation >` and can produce all three cursor
violations. A failed observation leaves the baseline untouched — stated on `last`,
whose doc comment exists for exactly that test — so a caller that logs and
continues is comparing against the last *sound* reading rather than the corrupt
one.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- the three public types and their derive lines --'
command grep -A1 '^#\[ derive' ring_debug/src/lib.rs | command grep -E 'derive|pub (enum|struct)' | sed 's/^/  /'
echo '-- only one of them has a method that mutates --'
command grep -E 'pub fn (new|observe|last)\(' ring_debug/src/lib.rs | sed 's/^/  /'
echo '-- what those derives are used for, across the whole module tree --'
printf '  signatures binding a Watch by value:   %s\n' "$( command grep -rnE ': *Watch *[,)]|-> *Watch\b' --include=*.rs | wc -l )"
printf '  sites comparing two Watch values:      %s\n' "$( command grep -rnE 'watch[0-9]? *== *watch|assert_eq!\( *watch[0-9]? *, *watch' --include=*.rs | wc -l )"
printf '  Cursor used as a hash or tree key:     %s\n' "$( command grep -rnE '(HashMap|HashSet|BTreeMap|BTreeSet)< *Cursor' --include=*.rs | wc -l )"
printf '  let mut watch bindings in the suite:   %s\n' "$( command grep -c 'let mut watch' ring_debug/tests/debug_test.rs || true )"
echo '-- the Error impl, and the family it sits in --'
command grep 'impl core::error::Error' ring_debug/src/lib.rs | sed 's/^/  /'
echo '  ring_* crates with an Error impl:'
command grep -rlE 'impl .*Error for' --include=*.rs ring_*/src | sed 's|/src/.*||;s|^|    |' | sort -u
printf '  ring_* sites defining fn source:       %s\n' "$( command grep -rn 'fn source' --include=*.rs ring_*/ | wc -l )"
```

Live output:

```
-- the three public types and their derive lines --
  #[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
  pub enum Cursor
  #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
  pub enum Violation
  #[ derive( Debug, Clone, PartialEq, Eq ) ]
  pub struct Watch
-- only one of them has a method that mutates --
    pub fn new( pair : &CursorPair ) -> Result< Self, Violation >
    pub fn observe( &mut self, pair : &CursorPair ) -> Result< (), Violation >
    pub fn last( &self ) -> ( Seq, Seq )
-- what those derives are used for, across the whole module tree --
  signatures binding a Watch by value:   0
  sites comparing two Watch values:      0
  Cursor used as a hash or tree key:     0
  let mut watch bindings in the suite:   8
-- the Error impl, and the family it sits in --
  impl core::error::Error for Violation {}
  ring_* crates with an Error impl:
    ring_bench
    ring_debug
    ring_factory
    ring_flush
    ring_registry
    ring_testkit
    ring_types
  ring_* sites defining fn source:       0
```

### Types

| File | Relationship |
|------|--------------|
| [001_violation.md](001_violation.md) | The two reported types, and the derive table this prices against |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | The three entry points, and which violations each can produce |
| [../api/002_the_message_is_a_second_api.md](../api/002_the_message_is_a_second_api.md) | The rendering half of the same public surface |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_the_conservation_law_and_why_it_holds.md](../invariant/002_the_conservation_law_and_why_it_holds.md) | The externally supplied `capacity` term, in its other appearance |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_watch_does_not_latch.md](../decisions/002_a_watch_does_not_latch.md) | Why a watch keeps observing after a violation, which is what makes the baseline's identity matter |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Watch`, its three methods, and the derive lines |
| [`ring_types/src/id.rs`](../../../ring_types/src/id.rs) | `Seq` and `Capacity` — the three fields |

### Tests

| Test | Relationship |
|------|--------------|
| `a_watch_refuses_to_baseline_a_broken_pair` | The construction refusal — the crate's only validated constructor |
| `a_failed_observation_leaves_the_baseline_alone` | The `&mut self` contract on the failure path |
| `a_watch_follows_an_ordinary_run_quietly` | The baseline advancing across observations |
| `a_stateless_check_cannot_see_a_reset_and_a_watch_can` | What the baseline buys over `check` |

### DB43 — the one type that holds a baseline is the one type that can be duplicated without a move

**What was found.** `Watch` derived `Copy` and has a `&mut self` method whose
whole purpose is to advance state the caller owns. The two were in tension: `Copy`
means any by-value use — an argument, a `let`, a `move` closure — silently
duplicates the baseline rather than moving it, and the original stays usable and
stale.

The consequence is specific to what this type does. A duplicate `Watch` still
answers `observe` correctly for the copy's own short life, so the mistake produced
no error at the call and no error afterwards; the caller simply stopped checking
D3, which is the only thing a `Watch` adds over `check`, and kept getting `Ok`.
The compiler would have caught it — a move out of a non-`Copy` value makes the
next use a hard error — and the derive removed that.

It was recorded as a latent hazard rather than a defect because the measurement
found no by-value binding anywhere in the module tree, and none of the two
comparisons or zero hash uses that the rest of the derive set would have bought
either. **The derive cost the one compile-time guarantee the type would benefit
from and bought nothing at all** — which is what made it cheap to change, and
which is the whole argument for changing it rather than the reason it was safe to
leave.

**This finding and DB6 are the same line read from two directions, and both were
right.** `data_structure/001` reached it by asking what a `Watch` holds;
this one reached it by asking what its derive list is for. Neither reading alone
would have moved it — one describes a hazard, the other prices it — and the
measurement that made the decision easy is the one in this document: a guarantee
worth having, against a derive nothing in the tree uses.

**Disposition:** applied — the derive list is `Debug, Clone, PartialEq, Eq`, so
the by-value uses that would have silently duplicated a baseline are now moves and
the compiler enforces the single-record property the type depends on; the zero
by-value signatures and zero comparisons this document measured are why nothing
had to change to accommodate it. Now prints: `#[ derive( Debug, Clone, PartialEq, Eq ) ]`

### DB44 — three types share one trait surface and the stateful one exercises none of it

`Cursor`, `Violation` and `Watch` carry near-identical derive lines. Measured
against the whole module tree, the surface divides cleanly by role and not by
declaration:

- **On `Violation`** — `Debug`, `PartialEq` and `Eq` are the assertion shape the
  crate is designed around, used in five tests; `Error` is used by `?`.
- **On `Cursor`** — `Display` and `PartialEq` are pinned by
  `the_cursor_names_are_distinct`; `Hash` has zero uses, which
  [`type/001`](001_violation.md) already records.
- **On `Watch`** — nothing takes one by value, nothing compares two, and the six
  bindings in the suite are all `let mut`. `Debug` is defensible on API convention
  grounds; `Copy`, `PartialEq` and `Eq` have no call site, and `Copy` has a cost
  (DB43).

The pattern worth naming is that the derive line was written once and repeated,
which is ordinary and usually harmless — a fieldless enum deriving `Hash` costs
nothing, as `type/001` argues. It stops being harmless at the type where a derive
changes what the compiler will let a caller do. **A trait surface chosen by
symmetry is right by accident on the inert types and wrong by accident on the one
that is not**, and nothing in the crate distinguishes the three cases because the
declaration does not.

The `Error` impl is the same shape seen from the other side: seven `ring_*` crates
implement it and none defines `fn source`, so this crate's correct `None` — a
`Violation` is a leaf error with nothing beneath it — is indistinguishable from six
possible omissions.
