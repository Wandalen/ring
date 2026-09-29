# Type: `Progress` Has No Zero-Made

### Scope

- **Purpose**: Record that `Progress::of( 0 )` is `Progress::None`, so this crate never produces `Made( 0 )` — a value that claims progress and carries none.
- **Responsibility**: The type's definition, what its constructor guarantees, and the exact strength of that guarantee: convention rather than construction.
- **In Scope**: `Progress`, its four associated functions, and the one hole the public variants leave open.
- **Out of Scope**: What a scheduler should *do* with the answer, which is the caller's decision; the budget that bounds the tick producing it (→ [`001_budget_clamps_to_one.md`](001_budget_clamps_to_one.md)).

### Definition

`ring_poll/src/lib.rs:173-180`

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub enum Progress
{
  /// Records moved, and this many of them.
  Made( usize ),
  /// Nothing moved.
  None,
}
```

| Item | Line | Signature |
|------|-----:|-----------|
| `Progress::of` | 189 | `pub const fn of( count : usize ) -> Self` |
| `Progress::is_made` | 206 | `pub const fn is_made( self ) -> bool` — `matches!( self, Self::Made( _ ) )` |
| `Progress::count` | 217 | `pub const fn count( self ) -> usize` — zero for `None` |
| `Progress::then` | 241 | `pub const fn then( self, other : Self ) -> Self` |

**What the type is for.** A scheduler asks one question of a tick: *did anything
happen?* If the answer is no, running the same systems again immediately is
wasted work, and the scheduler can back off, sleep the frame, or run something
else. The whole value of `Progress` is that the answer is trustworthy.

`Made( 0 )` breaks exactly that. It reads as yes and means no, and the code that
gets it wrong is the ordinary code — `Progress::Made( moved )` where `moved`
happens to be zero is the natural spelling, and it is right in every case except
the one that matters.

### Validation

**The constructor is the validation**, and it is two branches (`:191`):

```rust
pub const fn of( count : usize ) -> Self
{
  if count == 0 { Self::None } else { Self::Made( count ) }
}
```

Every internal producer of a `Progress` goes through it — `Tick::progress` is
`Progress::of( self.moved )`, and `then` is
`Progress::of( self.count() + other.count() )` (`:243`), so combining two
`None`s stays `None` without a special case.

#### The honest limit

**The variants are public, so a caller can still write `Progress::Made( 0 )`.**
Making them private and exposing only `of` plus accessors would close that, at
the cost of the thing the enum is actually good for: `match`ing on it. A caller
writing

```rust
match progress
{
  Progress::Made( n ) => schedule_again( n ),
  Progress::None => back_off(),
}
```

is the intended use, and it needs the variants.

So this is a *convention* guarantee rather than a *construction* one, and it is
marked that way in [`../api/001`](../api/001_tick_path_surface.md)'s guarantee
table — row 4, the only row on that list a caller can break. The alternative
would be a guarantee that reads stronger than it is, which is the failure this
family keeps choosing to avoid.

#### Why not `Option< NonZeroUsize >`

It would close the hole by construction, and it was rejected on naming: the type
would be `Option< NonZeroUsize >` at every call site, which says "maybe a number"
rather than "did the tick do anything". The domain word is worth the weaker
guarantee, given the guarantee is stated plainly.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'the enum variants:            %s\n' "$( awk '/^pub enum Progress$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -oE '^  [A-Z][A-Za-z]*(\( usize \))?,' | sed 's/^ *//;s/,$//' | tr '\n' ' ' )"
printf 'associated fns:               %s\n' "$( awk '/^impl Progress$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -oE 'pub const fn [a-z_]+' | sed 's/pub const fn //' | tr '\n' ' ' )"
printf 'the constructor body:         %s\n' "$( awk '/pub const fn of\( count/{f=1} f&&/^  \}$/{exit} f' src/lib.rs | command grep -oE 'if count == 0 .*' )"
printf 'what then routes through:     %s\n' "$( awk '/pub const fn then/{f=1} f&&/^  \}$/{exit} f' src/lib.rs | command grep -oE 'Self::of\(.*\)' )"
printf 'is_made asks:                 %s\n' "$( awk '/pub const fn is_made/{f=1} f&&/^  \}$/{exit} f' src/lib.rs | command grep -oE 'matches!\(.*\)' )"
printf 'count asks:                   %s\n' "$( awk '/pub const fn count/{f=1} f&&/^  \}$/{exit} f' src/lib.rs | command grep -oE 'Self::Made\( count \) => count' )"
printf 'Made( 0 ) built in src:       %s\n' "$( command grep -cE '(Progress|Self)::Made\( 0 \)' src/lib.rs || true )"
printf 'Made( 0 ) built in tests:     %s\n' "$( command grep -cE '(Progress|Self)::Made\( 0 \)' tests/poll_test.rs || true )"
printf 'files that only mention it:   %s\n' "$( command grep -rlE 'Made\( 0 \)' src tests | tr '\n' ' ' )"
printf 'internal producers via of:    %s\n' "$( command grep -cE '(Progress|Self)::of\(' src/lib.rs || true )"
printf 'tests the doc cites:          %s\n' "$( for n in progress_of_zero_is_no_progress progress_sums_across_the_steps_of_one_tick a_tick_counts_nothing_when_nothing_moved; do command grep -q "fn $n" tests/poll_test.rs && printf '%s ' "$n"; done )"
```

Live output:

```
the enum variants:            Made( usize ) None 
associated fns:               of is_made count then 
the constructor body:         if count == 0 { Self::None } else { Self::Made( count ) }
what then routes through:     Self::of( self.count().saturating_add( other.count() ) )
is_made asks:                 
count asks:                   Self::Made( count ) => count
Made( 0 ) built in src:       0
Made( 0 ) built in tests:     0
files that only mention it:   src/lib.rs tests/poll_test.rs 
internal producers via of:    5
tests the doc cites:          progress_of_zero_is_no_progress progress_sums_across_the_steps_of_one_tick a_tick_counts_nothing_when_nothing_moved 
```

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | Guarantee row 4 — where this type's convention-strength promise is recorded as convention rather than quietly promoted |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | The other place this crate states a guarantee narrower than its name suggests |

### Types

| File | Relationship |
|------|--------------|
| [`001_budget_clamps_to_one.md`](001_budget_clamps_to_one.md) | The sibling decision. Same goal — representable set equals meaningful set — but `Budget`'s field is private, so its version holds by construction and this one does not |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Lines 173-180 (the enum), 189-192 (`of`), 206, 217, 241-244 (`then`, which routes through `of`) |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `progress_of_zero_is_no_progress` asserts the constructor; `progress_sums_across_the_steps_of_one_tick` covers `then`; `a_tick_counts_nothing_when_nothing_moved` covers the path a scheduler actually sees |

```text
cargo nextest run -p ring_poll progress_of_zero_is_no_progress
cargo nextest run -p ring_poll progress_sums_across_the_steps_of_one_tick
cargo nextest run -p ring_poll a_tick_counts_nothing_when_nothing_moved
```

### PL47 — the hole the document is honest about closes itself on the first `then`, and nothing says so

*The honest limit* is the section this file is proudest of: the variants are
public, a caller can write `Progress::Made( 0 )`, and rather than claim
otherwise the document downgrades the promise to *convention* and points at
[`../api/001`](../api/001_tick_path_surface.md)'s guarantee table, row 4 — the
only row a caller can break. That is the right call and it is stated plainly.

It is also stronger than stated, in the type's favour. `then` is
`Self::of( self.count().saturating_add( other.count() ) )` — it does not
pattern-match its inputs, it re-derives from the counts and re-enters through
the constructor. So
`Made( 0 ).then( Progress::None )` is `of( 0 )`, which is `None`. A hand-written
`Made( 0 )` does not propagate: it survives exactly until it meets the operation
this type exists to support, which is folding one subsystem's answer into
another's, and it is normalized away on contact.

The accurate statement is therefore two-part rather than one: the guarantee is
convention-strength *at the boundary where a caller constructs a value*, and
construction-strength *one operation in*. A malformed value can be spelled, and
cannot be accumulated.

Nothing in the crate pins this. `Made( 0 )` is constructed zero times in `src`
and zero times in the suite — it appears only inside two doc comments, both
explaining why it is not constructed. So the laundering property is real, is
load-bearing for how far the documented hole actually reaches, is free to break
under any future rewrite of `then` that matches on variants instead of routing
through `of`, and is asserted nowhere. A three-line test would fix that; the
document would gain a sentence and lose nothing.

### PL48 — the API already contains the mitigation for its own documented hole, and the document names the unsafe accessor only

`Made( 0 )` *"reads as yes and means no"*, says the Definition section, and the
whole file follows from that sentence. It is a statement about one accessor.
`is_made` is `matches!( self, Self::Made( _ ) )` — it tests the discriminant and
ignores the payload, so it answers `true` for `Made( 0 )`. That is the reading
that breaks.

`count` does not. It is a `match` that returns the payload for `Made` and `0`
for `None`, so `Made( 0 ).count()` is `0` — the same answer `None` gives, which
is the correct answer. On the single value the type is built to exclude, the two
accessors disagree, and the one that reads the payload is right.

That makes `count() > 0` a caller-side mitigation for the documented hole, using
nothing but the existing surface, at no cost. The document never says it. It
explains why the variants must stay public (`match` is the intended use), it
grades the guarantee down to convention, it weighs and rejects
`Option< NonZeroUsize >` — and it does not mention that a caller who cares can
have construction-strength today by asking the other question.

The omission is understandable: the file is organized around the *type's*
guarantee, and this is a fact about how a caller reads it. But the audience is
the same either way, and *"the variants are public, so you can break this"*
lands very differently when the next sentence is *"and here is the accessor that
cannot be broken."* Recorded rather than folded into the prose because it also
argues for a doc-comment change on `is_made` itself, which is a change to the
published API surface rather than to this document.

**Correction (2026-09-28):** `is_made`'s implementation changed since this
finding was written — it is no longer `matches!( self, Self::Made( _ ) )`.
`Fix(progress_is_made_classification_not_exhaustive)` in `src/lib.rs` replaced
it with an exhaustive `match` (`Self::Made( _ ) => true, Self::None => false`)
so a future third variant fails to compile here instead of silently reading as
no-progress — an unrelated forward-compatibility fix, not a response to this
finding. The exhaustive form still answers `true` for `Self::Made( 0 )` exactly
as `matches!` did, so the reading this finding calls out — `is_made` cannot see
the difference between `Made( 0 )` and real progress — is unchanged, and
`count() > 0` remains the caller-side mitigation the rest of this finding
describes.
