# Type Doc Definition

### Scope

- **Purpose**: The arguments behind this crate's types — what each one makes unspellable, and what it does not.
- **Responsibility**: Two, both about the gap between the representable set and the meaningful one.
- **In Scope**: `Budget`'s clamp; `Progress`'s missing zero-made.
- **Out of Scope**: Data layout — nothing here is larger than a `usize` or a two-variant enum.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [`Budget` Clamps to One](001_budget_clamps_to_one.md) | Why zero attempts is clamped rather than accepted or rejected | 🔄 |
| 002 | [`Progress` Has No Zero-Made](002_progress_has_no_zero_made.md) | A constructor that cannot lie, and the public variants that still can | 🔄 |

**Both types shrink the representable set to the meaningful one, and they get
there by opposite routes.** `Budget` makes the bad value unspellable — a private
field, one constructor, a clamp inside it — so `attempts() >= 1` holds by
construction for every value that exists. `Progress` cannot do that without
giving up the thing it is for: the variants have to stay public because
`match`ing on them is the whole point, so `Made( 0 )` remains spellable and the
guarantee is graded down to convention and recorded as such.

That difference is the reason they are two documents rather than one. The
argument in `001` is about what a constructor should do with an input nobody
means (correct it, refuse it, or accept it); the argument in `002` is about what
to do when the language will not let you close a hole at all and the honest move
is to say so.

The four findings pair off the same way, and each pair follows the same shape:
the mechanism is sound and the document's account of it is one item short.
`001`'s type has four items where the doc counts three, and the fourth — a
hand-written `Default`, chosen over a derive that would have produced the very
`Budget( 0 )` the clamp forbids — is half of what holds the invariant up (PL45);
its derive surface is covered by exactly one test, named after one of its five
assertions, whose `HashSet` case uses `Budget::new( 1 )` where `Budget::new( 0 )`
would have pinned the clamp's real consequence (PL46). `002`'s documented hole
turns out to close itself on the first `then`, which routes through the
constructor rather than matching variants — stronger than the document claims,
and asserted nowhere (PL47) — and the surface already carries a caller-side
mitigation, `count()` rather than `is_made()`, that the document never names
(PL48).

Two of the four are the document understating its own type's guarantee, which is
the opposite of the failure this corpus usually finds and worth recording as
such: a file written to avoid over-claiming can land the other side of accurate.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/type
printf 'instances:                   %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:     %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:     %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe:  %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'public types in the crate:   %s\n' "$( command grep -oE '^pub (struct|enum) [A-Za-z]+' ../../src/lib.rs | sed 's/pub struct //;s/pub enum //' | tr '\n' ' ' )"
printf 'of those, documented here:   %s\n' "$( for n in $( command grep -oE '^pub (struct|enum) [A-Za-z]+' ../../src/lib.rs | sed 's/pub struct //;s/pub enum //' ); do command grep -qlE "\`$n\`" [0-9][0-9][0-9]_*.md 2>/dev/null && printf '%s ' "$n"; done )"
printf 'Budget: field declared pub:  %s\n' "$( command grep -cE 'pub struct Budget\( pub' ../../src/lib.rs || true )"
printf 'Progress: public variants:   %s\n' "$( awk '/^pub enum Progress$/{f=1} f&&/^\}$/{exit} f' ../../src/lib.rs | command grep -cE '^  [A-Z]' || true )"
printf 'hand-written trait impls:    %s\n' "$( command grep -oE '^impl [A-Za-z]+ for [A-Za-z]+' ../../src/lib.rs | sed 's/^impl //' | tr '\n' ' ' )"
printf 'both normalizing ctors:      %s\n' "$( command grep -oE 'if (attempts|count) == 0' ../../src/lib.rs | tr '\n' ' ' )"
printf 'what they return instead:    %s\n' "$( command grep -oE 'if (attempts|count) == 0 \{ [A-Za-z:()0-9 ]+ \}' ../../src/lib.rs | sed 's/.*{ //;s/ }//' | tr '\n' ' ' )"
printf 'either one written at all:   %s\n' "$( command grep -rhE 'Budget\( 0 \)|Made\( 0 \)' ../../src/lib.rs ../../tests/poll_test.rs | wc -l )"
printf 'of those, outside a comment: %s\n' "$( command grep -rhE 'Budget\( 0 \)|Made\( 0 \)' ../../src/lib.rs ../../tests/poll_test.rs | command grep -vcE '^ *(///|//)' || true )"
```

Live output:

```
instances:                   2
finding headings inside:     4
rows in the table below:     4
each instance has a recipe:  2
public types in the crate:   Budget Progress Tick 
of those, documented here:   Budget Progress 
Budget: field declared pub:  0
Progress: public variants:   2
hand-written trait impls:    Default for Budget Default for Tick 
both normalizing ctors:      if attempts == 0 if count == 0 
what they return instead:    Self( 1 ) Self::None 
either one written at all:   2
of those, outside a comment: 0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL45 | the type has four items, the document counts three, and the fourth holds the invariant up | **misleading doc** | The Definition section reads *"three associated functions"* and lists `once`, `new`, `attempts`, transcribing the `impl Budget` block; the type's fourth item is a hand-written `impl Default for Budget` returning `Self::once()`, written out rather than derived because `#[ derive( Default ) ]` on a tuple struct over `usize` yields `Budget( 0 )` — `attempts() == 0`, the exact value the clamp forbids and the private field otherwise makes unspellable — so the `attempts() >= 1` invariant rests on two mechanisms and the document records one, while its own Tests row cites the missing item's test. |
| PL46 | the only test of the derive surface is named after one of its five assertions | n/a — coverage | `a_budget_defaults_to_a_single_attempt` asserts `Default`, then `Ord` (`Budget::new( 2 ) > Budget::once()`), `Debug`, and `Hash` with `Eq` — four of the eight derives, covered here and nowhere else — while its name, and the document's Tests row (*"covers `Default`"*), advertise the first only; its `HashSet` case inserts `Budget::once()` and asserts `Budget::new( 1 )` collides, which is true for the uninteresting reason that both are one, where `Budget::new( 0 )` would have pinned the clamp's actual consequence — that a bad calculation is `Eq`, `Ord`- and `Hash`-indistinguishable from a deliberate default — and every assertion in the test would still pass with the clamp deleted. |
| PL47 | the documented hole closes itself on the first `then`, and nothing says so | n/a — doc gap | *The honest limit* correctly grades the no-`Made( 0 )` promise down to convention because the variants are public, and marks it as the one guarantee row a caller can break; `then` is `Self::of( self.count() + other.count() )`, re-deriving from counts rather than matching variants, so a hand-written `Made( 0 )` normalizes to `None` the moment it is folded — the guarantee is convention-strength where a value is constructed and construction-strength one operation in, a malformed value can be spelled but not accumulated — and `Made( 0 )` is built zero times in `src` and zero times in the suite, appearing only in two doc comments, so the laundering is unasserted and free to break under any rewrite of `then` that matches on variants. |
| PL48 | the surface already carries the mitigation for its own documented hole | n/a — doc gap | *"`Made( 0 )` reads as yes and means no"* is a statement about `is_made`, which is `matches!( self, Self::Made( _ ) )` and so tests the discriminant and ignores the payload; `count` matches and returns the payload, giving `0` for both `Made( 0 )` and `None`, so the two accessors disagree on exactly the value the type excludes and the payload-reading one is correct — `count() > 0` is a zero-cost caller-side mitigation using only the existing surface, and the document weighs `Option< NonZeroUsize >`, explains why the variants must stay public, and grades the guarantee down without ever mentioning it. |
