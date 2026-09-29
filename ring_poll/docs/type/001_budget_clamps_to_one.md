# Type: `Budget` Clamps to One

### Scope

- **Purpose**: Record that `Budget::new( 0 )` yields a budget of one attempt rather than zero, and rather than an error — and why that was chosen over the two alternatives.
- **Responsibility**: The type's definition, the single validation it performs, and the two things that validation deliberately does not cover.
- **In Scope**: `Budget`'s constructors and the `attempts() >= 1` invariant they establish for every value that exists.
- **Out of Scope**: Whether a *large* budget is safe, which no constructor can settle (→ [`../invariant/002`](../invariant/002_a_budget_bounds_attempts_not_time.md)); the helpers that consume a budget (→ [`../api/001`](../api/001_tick_path_surface.md)).

### Definition

`ring_poll/src/lib.rs:115-116`

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]
pub struct Budget( usize );
```

A private-field newtype over `usize`, with three associated functions and one
hand-written trait impl:

| Item | Line | Signature |
|------|-----:|-----------|
| `Budget::once` | 125 | `pub const fn once() -> Self` — `Self( 1 )`, the tick-path default and the only budget that cannot cost more than one ring operation |
| `Budget::new` | 137 | `pub const fn new( attempts : usize ) -> Self` |
| `Budget::attempts` | 144 | `pub const fn attempts( self ) -> usize` |
| `impl Default for Budget` | 153 | `fn default() -> Self` — `Self::once()`, the only thing standing between the derive list and a spellable `Budget( 0 )` |

The clamp is the whole of `new`'s body (`:139`):

```rust
if attempts == 0 { Self( 1 ) } else { Self( attempts ) }
```

One `if`, in a `const fn`. The field is private, so `Budget( 0 )` is
unspellable outside this module and `new` is the only way in from a `usize`.

### Validation

**Zero is the only invalid input, and it is corrected rather than refused.**
Three options were available:

| Option | What `push_within( .., Budget::new( 0 ) )` would do | Rejected because |
|---|---|---|
| Accept zero | Return `Err( record )` without touching the ring | The caller reads `Err` as back-pressure. It is not — the ring may be empty. A function that reports a full ring when the ring is idle is worse than one that is merely slow |
| Reject zero | `Budget::new` returns `Result< Budget, _ >` | Puts a fallible constructor and an `unwrap` on the tick path to rule out a value nobody wants. The cost lands on every correct caller to discipline an incorrect one |
| **Clamp to one** | One attempt, honest answer | A budget of zero attempts is not a small budget; it is a different operation with a misleading name. Clamping names the nearest thing the caller can have meant |

The source states the same reasoning in `new`'s doc comment (`:132-135`): *"a
helper that 'tries zero times' is not a bounded retry, it is a no-op with a
misleading name."*

#### Why this is a type decision and not a validation decision

Clamping in the constructor means the invariant `attempts() >= 1` holds for
*every* `Budget` value that exists, so the loops that consume it do not need a
zero case and the tests do not need to cover one. If the clamp lived in
`push_within` instead, each of the four consumers would carry its own copy of
it, and the fourth one written would be the one that forgot.

This is the cheapest version of the family's recurring move — make the
representable set match the meaningful set — and it costs one `if` in a `const
fn` (→ [`../pattern/001`](../pattern/001_enforcement_by_dependency_graph.md) for
the same move applied to a dependency graph instead of a field).

#### What the clamp does not do

**It does not make a large budget safe.** The type rules out zero, not a
million; that is
[`../invariant/002`](../invariant/002_a_budget_bounds_attempts_not_time.md)'s
territory, and no constructor can settle it, because the right ceiling depends
on the caller's frame budget rather than on anything this crate knows.

**It does not report that clamping happened.** `Budget::new( 0 ).attempts()` is
`1` with no signal that `0` was asked for. That is deliberate — a warning here
would have to be a `Result`, which is option two — but it does mean a caller who
computed `0` from a bad calculation gets a working budget rather than a
complaint. The doc comment says so, and the test asserts the clamp explicitly so
the behaviour is discoverable from the suite as well as from the prose.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'the struct:                   %s\n' "$( command grep -oE 'pub struct Budget\( [a-z0-9]+ \)' src/lib.rs )"
printf 'derives on it:                %s\n' "$( awk '/^pub struct Budget/{print prev} {prev=$0}' src/lib.rs | command grep -oE '[A-Z][A-Za-z]+' | tr '\n' ' ' )"
printf 'associated fns in impl:       %s\n' "$( awk '/^impl Budget$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -oE 'pub const fn [a-z_]+' | sed 's/pub const fn //' | tr '\n' ' ' )"
printf 'trait impls written by hand:  %s\n' "$( command grep -oE '^impl [A-Za-z]+ for Budget' src/lib.rs | sed 's/impl //;s/ for Budget//' | tr '\n' ' ' )"
printf 'is Default among the derives: %s\n' "$( awk '/^pub struct Budget/{print prev} {prev=$0}' src/lib.rs | command grep -c 'Default' || true )"
printf 'what the hand-written one is: %s\n' "$( awk '/^impl Default for Budget$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -oE 'Self::[a-z_]+\(\)' )"
printf 'the clamp itself:             %s\n' "$( awk '/pub const fn new\( attempts/{f=1} f&&/^  \}$/{exit} f' src/lib.rs | command grep -oE 'if attempts == 0 .*' )"
printf 'asserts in the Default test:  %s\n' "$( awk '/fn a_budget_defaults_to_a_single_attempt/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -cE 'assert' || true )"
printf 'what that test reaches:       %s\n' "$( awk '/fn a_budget_defaults_to_a_single_attempt/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -oE 'Budget::default|Budget::new\( 2 \) >|format!|HashSet' | tr '\n' ' ' )"
printf 'the values it hashes:         %s\n' "$( awk '/fn a_budget_defaults_to_a_single_attempt/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -oE 'seen.insert\( Budget::[a-z_]+\([ 0-9]*\) \)' | sed 's/seen.insert( //;s/ )$//' | tr '\n' ' ' )"
printf 'debug_assert in this crate:   %s\n' "$( command grep -rc 'debug_assert' src tests 2>/dev/null | command grep -v ':0$' | wc -l )"
```

Live output:

```
the struct:                   pub struct Budget( usize )
derives on it:                Debug Clone Copy PartialEq Eq PartialOrd Ord Hash 
associated fns in impl:       once new attempts 
trait impls written by hand:  Default 
is Default among the derives: 0
what the hand-written one is: Self::once()
the clamp itself:             if attempts == 0 { Self( 1 ) } else { Self( attempts ) }
asserts in the Default test:  5
what that test reaches:       Budget::default Budget::new( 2 ) > format! HashSet 
the values it hashes:         Budget::once() Budget::new( 1 ) 
debug_assert in this crate:   1
```

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | Lists `Budget::new` as one of the entries whose cost the *caller* chooses — the clamp bounds the value below, not above |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | The half this type cannot establish: attempts are bounded, elapsed time is not |

### Patterns

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforcement_by_dependency_graph.md`](../pattern/001_enforcement_by_dependency_graph.md) | The same representable-set-equals-meaningful-set move, enforced by a manifest rather than by a constructor |

### Types

| File | Relationship |
|------|--------------|
| [`002_progress_has_no_zero_made.md`](002_progress_has_no_zero_made.md) | The sibling decision, and the contrast: `Progress` makes the same guarantee by convention because its variants are public, where `Budget`'s field is not |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Lines 116 (the struct), 125 (`once`), 137-140 (`new` and the clamp), 144 (`attempts`) |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `a_budget_is_at_least_one_attempt` asserts the clamp directly; `a_budget_defaults_to_a_single_attempt` covers `Default` — and, undeclared by its name, `Ord`, `Debug`, `Hash` and `Eq` |

```text
cargo nextest run -p ring_poll a_budget_is_at_least_one_attempt
cargo nextest run -p ring_poll a_budget_defaults_to_a_single_attempt
```

### PL45 — the type has four items and the document counts three, and the fourth is the one holding the invariant up

The Definition section reads *"A private-field newtype over `usize`, with three
associated functions"*, and the table under it lists exactly three: `once`,
`new`, `attempts`. The type has a fourth item — `impl Default for Budget` —
written out by hand a dozen lines below the `impl Budget` block the document
transcribes.

Its omission would be a small tidiness matter except for what it does.
`Budget` is `Budget( usize )` with a private field, and `Default` is *not* among
the eight derives. Had it been, `#[ derive( Default ) ]` on a tuple struct over
`usize` produces `Budget( 0 )` — a budget of zero attempts, `attempts() == 0`,
the exact value `new`'s clamp exists to rule out and the one the field's privacy
otherwise makes unspellable from outside this module. The hand-written impl,
returning `Self::once()`, is the only thing standing between the derive list and
a hole straight through the type's single invariant.

So the document's Validation section is right that clamping in the constructor
makes `attempts() >= 1` hold for every value that exists — but the reason it
holds is two mechanisms, not one, and only the first is written down. The
document even cites the second's test: its Tests row names
`a_budget_defaults_to_a_single_attempt` and describes it as covering `Default`,
so the item is present in the file everywhere except the section whose job is
enumerating it.

The fix is a row and a sentence. What the finding records is the shape: a doc
that transcribes one `impl` block and counts what it finds there will miss every
trait impl written outside it, and the ones written outside it are exactly the
ones chosen deliberately over a derive — which is to say, the load-bearing ones.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F "the only thing standing between the derive list and a spellable" ring_poll/docs/type/001_budget_clamps_to_one.md
```

Live output:

```
| `impl Default for Budget` | 153 | `fn default() -> Self` — `Self::once()`, the only thing standing between the derive list and a spellable `Budget( 0 )` |
```

**Disposition:** applied — the Definition section's table above now lists
four items, not three: the hand-written `impl Default for Budget` at line 117
has its own row, and the prose above the table now says "three associated
functions and one hand-written trait impl" instead of undercounting the type
by the one item that keeps `attempts() >= 1` from having a derive-shaped
hole. Now prints: `the only thing standing between the derive list and a spellable`

### PL46 — the only test of the derive surface is named after one of its five assertions, and stops one digit short of the interesting one

`a_budget_defaults_to_a_single_attempt` carries five assertions. The first is
its name: `Budget::default() == Budget::once()`. The other four are not
mentioned in the name, in the document's Tests row (*"covers `Default`"*), or
anywhere else — they exercise `Ord` (`Budget::new( 2 ) > Budget::once()`),
`Debug` (the output contains `1`), and `Hash` with `Eq` (two budgets inserted
into a `HashSet`, the second rejected). Four of the type's eight derives are
covered here and nowhere else in the crate.

The `HashSet` assertion is the one worth looking at. It inserts
`Budget::once()`, then asserts `Budget::new( 1 )` collides with it — true, and
true for the uninteresting reason that both are literally one.

`Budget::new( 0 )` would have collided too, and that fact is the clamp's real
consequence. Since `new` corrects rather than records, `Budget::new( 0 )` and
`Budget::once()` are `Eq`, hash together, and compare equal under `Ord` — so a
value arrived at by a bad calculation is indistinguishable from a deliberate
default, in a map, in a sorted list, and in an assertion. The document names
half of this under *What the clamp does not do* — *"no signal that `0` was asked
for"* — and frames it as a choice about warnings. The derives make it stronger
than that: the information is not merely unreported, it is unrecoverable, and
eight derived traits propagate the collapse into every collection the type
enters.

Three lines already in the suite would have pinned it, one digit apart from the
lines that are there. The finding is not that the test is wrong — it passes and
what it asserts is true. It is that the assertion chosen is the one that would
still pass if the clamp were deleted.
