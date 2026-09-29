# Data Structure: Five Public Types and Their Derive Sets

### Scope

- **Purpose**: Fix what each of the crate's five public types derives, what those derives let a caller do, and where the sets disagree without a stated reason.
- **Responsibility**: The five types, their derive lists, the trait bounds each list implies on a generic parameter, and the element type the suite actually exercises them at.
- **In Scope**: `Shutdown`, `Stopped`, `Refusal`, `Guarded`, `Wake`.
- **Out of Scope**: Why `Stopped` exists (→ [`../type/001`](../type/001_stopped_proof_token.md)); why `Refusal` carries the record rather than an error (→ [`../type/002`](../type/002_refusal_carries_the_record.md)).

### Layout

| Type | Derives | Generic | What the set permits |
|---|---|---|---|
| `Shutdown` | `Debug` | — | Printing. Not `Clone` — a second copy of the flag is the failure mode the crate exists to prevent |
| `Stopped< 'a >` | `Debug` | — | Printing. Not `Clone`, or the proof could be duplicated past a `reopen` |
| `Guarded< 'a, T >` | `Debug` | `T` | Printing. Not `Clone` — two guards over one producer would be two producers |
| `Refusal< T >` | `Debug, Clone, Copy, PartialEq, Eq` | `T` | Comparison and copying, each gated on `T` having the same |
| `Wake` | `Debug, Clone, Copy, PartialEq, Eq, Hash` | — | All of the above, plus use as a map key |

The three `Debug`-only types are `Debug`-only for the same reason stated three
different ways: each is a unique capability, and a derive that duplicates it
would dissolve the guarantee it carries. That much is coherent and deliberate.

The two value types are where the sets stop agreeing.

### What the Derives Cost

`Refusal`'s `Copy` and `PartialEq` are not free — they are conditional bounds.
`Refusal< T >` is `Copy` only when `T` is, `PartialEq` only when `T` is, and so
on. For a record type that is neither, `Refusal< T >` is a plain `Debug` enum,
and every operation the suite performs on it stops compiling.

That is the ordinary derive contract and it is not a defect. It is worth stating
because the crate's tests are written entirely at one element type, and that
type has every bound.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'public types:                  %s\n' "$( command grep -ohE '^pub (struct|enum) [A-Za-z]+' ring_shutdown/src/lib.rs | awk '{ print $3 }' | tr '\n' ' ' )"
awk '/^#\[ derive/{ d=$0; getline; if ( $0 ~ /^pub (struct|enum)/ ) { n=$3; sub( /[<(].*/, "", n ); gsub( /^#\[ derive\( | \) \]$/, "", d ); printf "  %-10s derives: %s\n", n, d } }' ring_shutdown/src/lib.rs
printf 'types deriving Hash:           %s\n' "$( awk '/^#\[ derive/{ d=$0; getline; if ( $0 ~ /^pub (struct|enum)/ && d ~ /Hash/ ) { n=$3; sub( /[<(].*/, "", n ); printf "%s ", n } }' ring_shutdown/src/lib.rs )"
printf 'public enums in the crate:     %s\n' "$( command grep -ohE '^pub enum [A-Za-z]+' ring_shutdown/src/lib.rs | awk '{ print $3 }' | tr '\n' ' ' )"
printf 'element types the suite uses:  %s\n' "$( command grep -ohE 'Ring< [A-Za-z0-9_]+ >' ring_shutdown/tests/shutdown_test.rs ring_shutdown/src/lib.rs | sort -u | tr '\n' ' ' )"
printf 'of those, not Copy:            %s\n' "$( command grep -ohE 'Ring< [A-Za-z0-9_]+ >' ring_shutdown/tests/shutdown_test.rs ring_shutdown/src/lib.rs | sort -u | command grep -vcE 'Ring< (u|i)(8|16|32|64|size) >' || true )"
printf 'Refusal assertions in tests:   %s\n' "$( command grep -cE 'assert[_a-z]*!\(.*[Rr]efus' ring_shutdown/tests/shutdown_test.rs || true )"
printf 'of those needing PartialEq:    %s\n' "$( command grep -cE 'assert_eq!\( refused,' ring_shutdown/tests/shutdown_test.rs || true )"
printf 'a stated rule for the derives: %s\n' "$( command grep -rc 'derive' ring_shutdown/docs/api/001_shutdown_surface.md || true )"
```

Live output:

```
public types:                  Shutdown Stopped Refusal Guarded Wake 
  Shutdown   derives: Debug
  Stopped    derives: Debug
  Refusal    derives: Debug, Clone, Copy, PartialEq, Eq
  Guarded    derives: Debug
  Wake       derives: Debug, Clone, Copy, PartialEq, Eq, Hash
types deriving Hash:           Wake 
public enums in the crate:     Refusal Wake 
element types the suite uses:  Ring< u32 > 
of those, not Copy:            0
Refusal assertions in tests:   7
of those needing PartialEq:    1
a stated rule for the derives: 0
```

### Types

| File | Relationship |
|------|--------------|
| [`../type/002_refusal_carries_the_record.md`](../type/002_refusal_carries_the_record.md) | The argument for the shape whose derives this document counts |
| [`../type/001_stopped_proof_token.md`](../type/001_stopped_proof_token.md) | Why three of the five derive nothing but `Debug` |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_shutdown_surface.md`](../api/001_shutdown_surface.md) | The surface these five types make up, with the by-construction/by-convention column |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The five declarations and their derive attributes |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `a_full_guarded_producer_refuses_with_the_transient_arm` — the one test that compares a `Refusal` by value, and so the one that needs the derives |

### SD11 — Two Public Enums, One Hashable, and No Rule Anywhere Saying Which

`Wake` derives `Hash`. `Refusal` does not. They are the crate's only two public
enums, both small, both returned from operations a caller dispatches on, and
both plausible map keys — a producer tallying refusal reasons wants
`HashMap< Refusal< T >, usize >` at least as much as a waiter tallying wake
reasons wants `HashMap< Wake, usize >`.

Nothing prevents it. `#[ derive( Hash ) ]` on a generic enum adds a `where T : Hash`
bound and costs nothing when unused, exactly as the existing `Copy` and
`PartialEq` derives already do on the same type. So the asymmetry is available
to fix and was not a constraint.

It is also unexplained. [`api/001`](../api/001_shutdown_surface.md) fixes the
whole surface in one table and marks each item by-construction or by-convention,
and mentions derives nowhere; neither type's doc comment gives a reason. The
result is a difference a reader must assume is meaningful — the two enums *look*
deliberately distinguished — when the evidence available says only that they
were written at different times.

### SD12 — The Suite Can Only Exercise `Refusal` in the Shape Where Its Purpose Is Moot

[`type/002`](../type/002_refusal_carries_the_record.md) argues that a guarded
push refuses with a two-armed type carrying the record rather than with a
`RingError`, because the record must come back intact. That argument is about
*not losing the payload*: for a record type that is expensive to rebuild or
impossible to reconstruct, handing it back is the difference between a retry and
a data loss.

Every ring in the suite and in every doctest is a `Ring< u32 >`. `u32` is `Copy`,
so nothing is at stake in the hand-back — the caller still has the value whether
or not the type returns it, and `into_record` is a bit-copy from a value that
remains usable afterwards.

This is not a gap that a wider test would trivially close, which is the
interesting part. One of the suite's seven `Refusal` assertions is a whole-value
comparison — `assert_eq!( refused, Refusal::Full( 3 ) )` — and it compiles only
because `Refusal< T >`'s `PartialEq` derive is satisfied at `T = u32`. At a
non-`Copy`, non-`PartialEq` record type the type behaves exactly as designed and
that assertion stops compiling, so widening the element type is a rewrite of the
tests rather than an addition to them. The shape the type was built for is the
one shape its suite would have to be re-authored to reach, and it has not been.
