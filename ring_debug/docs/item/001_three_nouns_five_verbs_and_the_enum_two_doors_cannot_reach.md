# Item: Three Nouns, Five Verbs, and the Enum Two Doors Cannot Reach

### Scope

- **Purpose**: Catalogue the public surface as a *set*, and record the properties that are visible only once it is one.
- **Responsibility**: Every public declaration, what it returns, and which of the three entry points can reach it.
- **In Scope**: The eight public declarations, the three private ones, and the reachability relation between them.
- **Out of Scope**: What each signature guarantees (→ [`api/001`](../api/001_the_check_surface.md)); the shape of the reported value (→ [`type/001`](../type/001_violation.md)).

### Abstract

Eight public declarations: three nouns, five verbs. Three more that are private,
and four `impl` blocks. It is a small surface — small enough that the catalogue
fits on one screen, which is what makes the two properties below checkable at
all.

Both are properties of the *set*. Neither is visible from any single rustdoc
page, and `#![ deny( missing_docs ) ]` — which this crate carries — guarantees
that each of those pages exists while saying nothing about what they say
collectively.

### Data Structures

**The nouns.**

| # | Item | Kind | Shape | Reachable from |
|---|---|---|---|---|
| 1 | `Cursor` | `pub enum` | 2 unit variants: `Producer`, `Consumer` | `Watch::observe` only |
| 2 | `Violation` | `pub enum` | 4 struct variants | all three entry points |
| 3 | `Watch` | `pub struct` | 3 private fields: `producer`, `consumer`, `capacity` | constructed by `Watch::new` |

**The verbs.**

| # | Item | Kind | Returns | Door |
|---|---|---|---|---|
| 4 | `check` | `pub fn` | `Result< (), Violation >` | 1 — one observation |
| 5 | `Watch::new` | `pub fn` | `Result< Self, Violation >` | 2 — a held baseline |
| 6 | `Watch::observe` | `pub fn` | `Result< (), Violation >` | 2 |
| 7 | `Watch::last` | `pub fn` | `( Seq, Seq )` | 2 |
| 8 | `check_ends` | `pub fn` | `Result< (), Violation >` | 3 — a split ring |

**And the three that are not public.**

| # | Item | Kind | Why it is private |
|---|---|---|---|
| — | `OBSERVE` | `const Ordering` | One shared `Acquire` for every read in the crate; a caller choosing its own could report a violation of an invariant nothing violated |
| — | `check_seqs` | `fn` | The comparison over *values*, so `Watch` can ask the same two questions of sequences it has already read rather than reading them a second time |
| — | `observe_pair` | `fn` | The two loads, in one place, so the producer-first ordering is decided once instead of at every read site — [`algorithm/002`](../algorithm/002_the_third_ordering.md)'s DB23 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
printf 'public types      %s\n' "$( command grep -cE '^pub (struct|enum) ' $S )"
printf 'public free fns   %s\n' "$( command grep -cE '^pub fn ' $S )"
printf 'public methods    %s\n' "$( command grep -cE '^  pub fn ' $S )"
printf 'private items     %s\n' "$( command grep -cE '^(fn|const|static) ' $S )"
printf 'impl blocks       %s\n' "$( command grep -cE '^impl' $S )"
echo '-- every attribute, with its line --'
command grep -E '^\s*#!?\[' $S
echo '-- Cursor: the bare type, vs the two longer names it is a prefix of --'
command grep -oE '[A-Za-z_]*Cursor[A-Za-z_]*' $S | sort | uniq -c
echo '-- the reachability claim: every site that builds one, and the fn it sits in --'
command grep -E 'cursor : Cursor::' $S
echo '   (Watch::observe spans lines 325-360; the third is the rustdoc example on Watch)'
```

Live output:

```
public types      3
public free fns   2
public methods    3
private items     3
impl blocks       4
-- every attribute, with its line --
#![ deny( missing_docs ) ]
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ derive( Debug, Clone, PartialEq, Eq ) ]
  #[ must_use ]
-- Cursor: the bare type, vs the two longer names it is a prefix of --
      7 Cursor
     12 CursorPair
      7 CursorWentBackwards
-- the reachability claim: every site that builds one, and the fn it sits in --
///   Err( Violation::CursorWentBackwards { cursor : Cursor::Producer, was : Seq( 7 ), now : Seq( 2 ) } )
          cursor : Cursor::Producer,
          cursor : Cursor::Consumer,
   (Watch::observe spans lines 325-360; the third is the rustdoc example on Watch)
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_violation.md](../type/001_violation.md) | The variant that carries the only `Cursor` in the surface |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | The same eight declarations, grouped by what the caller must supply |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The declarations themselves |

### Tests

| Test | Relationship |
|------|--------------|
| `a_backwards_consumer_is_named_as_the_consumer` | DB1 — the only place a `Cursor` value is read off a returned `Violation`, and it comes from `observe` |
| `the_cursor_names_are_distinct` | DB1 — the enum's own two variants, exercised without a ring at all |

### DB1 — `Cursor` is public and two of the three doors cannot produce it


`Cursor` names which of the two cursors moved backwards, and it appears in
exactly one place in the whole public surface: the `cursor` field of
`Violation::CursorWentBackwards`. That variant is returned by `Watch::observe`
and by nothing else — `check` and `check_ends` are stateless and cannot see a
backwards move at all (`api/001`'s A4, and the reason `Watch` exists as a
separate door).

So a caller who only ever calls `check` — the smallest and most obvious of the
three entry points — imports a public enum from this crate that its code path can
never construct and never match against. The type is not dead; it is
*conditionally* live, on a condition the type system does not carry.

**This is the ordinary shape for an error enum shared across entry points**, and
it is worth naming precisely because it is ordinary: the alternative is one error
type per door, which prices a three-line `check( pair )?` at three incompatible
`From` impls. The catalogue records the cost rather than proposing the trade be
reversed.

### DB2 — exactly one `#[ must_use ]`, on the only verb that returns neither `Result` nor `Self`


Four attributes in the whole crate, and the census below shows where they are.
Three are `derive`. The fourth is a `#[ must_use ]` on `Watch::last`.

That is not an oversight in the other four verbs — it is the correct and complete
set. `check`, `Watch::new`, `Watch::observe` and `check_ends` all return `Result`,
which carries `#[ must_use ]` in `core`, so the lint already fires for them.
`Watch::last` returns a bare `( Seq, Seq )`, which does not, and it is the only
public verb in the crate that returns a plain value.

**The property is that the attribute set is exactly the complement of what `core`
provides**, with no overlap and no gap — a thing that is true of the set and
cannot be read off any one declaration, because each declaration in isolation
looks equally entitled to the attribute.
