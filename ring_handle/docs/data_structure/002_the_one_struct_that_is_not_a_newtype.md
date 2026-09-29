# Data Structure: The One Struct That Is Not a Newtype

### Scope

- **Purpose**: Define `Drain` — the crate's only structure with state of its own — and account for what its second field buys and what it cannot promise.
- **Responsibility**: State the layout, the ownership, the invariants, and the operations.
- **In Scope**: `Drain`'s two fields; the borrow it holds; the count it decrements; what the count means when a producer is running.
- **Out of Scope**: The four newtypes (→ [`data_structure/001`](001_two_handles_over_one_backend.md)); the iteration protocol itself, which is `Iterator`'s.

### Layout

```rust
pub struct Drain< 'c, 'a, T >
{
  consumer : &'c mut Consumer< 'a, T >,
  remaining : usize,
}
```

**Two lifetimes, and the nesting is the whole shape.** `'a` is the ring's borrow,
held by the `Consumer`; `'c` is the borrow of that `Consumer`, held here. A
`Drain` cannot outlive the consumer it drains, and the consumer cannot outlive
the ring. Neither relation needs stating in a `where` clause — the lifetimes
carry it.

`remaining` is the only field in the crate that is not a re-named borrow of
something one crate down.

### Ownership

| Thing | Owned by | For how long |
|-------|----------|--------------|
| The ring | `Split` | Until the `Split` drops |
| The consumer's borrow of it | `Consumer< 'a, T >` | `'a` |
| The drain's borrow of the consumer | `Drain< 'c, 'a, T >` | `'c`, nested inside `'a` |
| `remaining` | `Drain` | The drain's own life; nothing else can read or write it |

**Nothing is copied and nothing is allocated.** A `Drain` is a pointer and a
count, and the records it yields move out of the ring one at a time as `next` is
called.

### Invariants

| # | Invariant | Enforced by |
|---|-----------|-------------|
| D1 | `remaining` never increases | It is only ever decremented, and only in `next` |
| D2 | At most `remaining` records are yielded | `next` returns `None` once it hits zero |
| D3 | Fewer may be yielded | `next` also returns `None` when `try_recv` does |
| D4 | The consumer is exclusively borrowed for `'c` | `&'c mut` |

**D2 and D3 together are the contract**, and the asymmetry is deliberate: the
bound is an upper bound, never a promise of how many arrive.

### Operations

`next` and `size_hint`, from `Iterator`. `size_hint` reports
`( 0, Some( remaining ) )` — lower bound zero, which is D3 stated in the type
system's own vocabulary.

There is no `len`, so `Drain` is not `ExactSizeIterator`, and that is correct:
an exact size would be a claim D3 forbids.

### Data Structures

| File | Relationship |
|------|--------------|
| [001_two_handles_over_one_backend.md](001_two_handles_over_one_backend.md) | The four newtypes this one is the exception to |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_five_nouns_four_of_them_the_same_width.md](../item/001_five_nouns_four_of_them_the_same_width.md) | Where `Drain` appears in the catalogue as the odd one |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | The rule this struct is the exception to |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/003_handle_ownership.md](../lifecycle/003_handle_ownership.md) | The borrow chain `'c` nests inside |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The declaration, the `Iterator` impl, and `Consumer::drain` |

### Tests

| Test | Relationship |
|------|--------------|
| `drain_is_bounded_at_the_call_that_made_it` | D2 |
| `drain_stops_when_the_ring_empties_first` | D3 |
| `drain_of_an_empty_ring_yields_nothing` | D2 and D3 at once, at zero |

### HD9 — The Bound Is a Snapshot, and Nothing in the Type Says So

`remaining` is set once and only ever falls:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- where remaining is written --'
command grep -vE '^\s*(///|//!)' ring_handle/src/lib.rs \
  | command grep -E 'remaining'
echo '  -- and the struct that holds it --'
sed -n '/^pub struct Drain/,/^}/p' ring_handle/src/lib.rs
```

Live output:

```
  -- where remaining is written --
    let remaining = self.inner.len();
    Drain { consumer : self, remaining }
  remaining : usize,
    if self.remaining == 0
    self.remaining -= 1;
    ( 0, Some( self.remaining ) )
  -- and the struct that holds it --
pub struct Drain< 'c, 'a, T >
{
  consumer : &'c mut Consumer< 'a, T >,
  remaining : usize,
}
```

Six lines mention it and none of them is a second writer: the field
declaration, the two construction lines in `Consumer::drain`, the zero check in
`next`, the single `-= 1`, and the read in `size_hint`. Set once, decremented
once, read twice.

**So `Drain< 'c, 'a, T >`'s type says "an iterator over a consumer" and its
behaviour is "an iterator over at most the records present when you asked".**
The difference matters exactly when a producer is running: a caller who reads
the signature reasonably expects to drain the ring, and what they get is a
prefix bounded by a number captured before any of their iteration happened.

The behaviour is right — an unbounded drain against a live producer never
terminates — and it is the *naming* that carries none of it. A reader who
wants the guarantee has to find `Consumer::drain`'s doc comment; the type they
are holding does not mention a snapshot, and `size_hint`'s upper bound is the
only structural hint, one `Iterator` impls routinely under-report anyway.

**Disposition:** declined — checked against current source rather than taken
on the finding's word: `Drain`'s own struct doc comment at
`ring_handle/src/lib.rs:249` already reads "The iterator
[`Consumer::drain`] returns, **bounded at the call that made it**," which
states the snapshot fact on the type itself, not only in `drain`'s doc, and
links straight to `drain`'s much fuller explanation two lines above it in the
same file ("**The bound is fixed here, not as the iterator runs.**"). The
specific gap this finding names — "the type they are holding does not mention
a snapshot" — does not hold against the crate as it stands today.

### HD10 — `Drain` Is the Only Structure Whose Correctness Is This Crate's

Every other struct here delegates its behaviour:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- this crate noun -> the ring_core declaration it fronts --'
for pair in Split:Ring Ends:Ends Producer:Producer Consumer:Consumer Drain:Drain; do
  here="${pair%%:*}"; there="${pair##*:}"
  printf '  %-10s -> %-10s %s\n' "$here" "$there" \
    "$( command grep -cE "^pub (struct|enum) $there[< ]" ring_core/src/lib.rs )"
done
echo '  -- tests whose name is about the add --'
command grep -E '^fn (drain|draining)_' ring_handle/tests/handle_test.rs
printf '  tests in the file: %s\n' \
  "$( command grep -c '^#\[ test \]' ring_handle/tests/handle_test.rs )"
```

Live output:

```
  -- this crate noun -> the ring_core declaration it fronts --
  Split      -> Ring       1
  Ends       -> Ends       1
  Producer   -> Producer   1
  Consumer   -> Consumer   1
  Drain      -> Drain      0
  -- tests whose name is about the add --
fn drain_is_bounded_at_the_call_that_made_it()
fn drain_stops_when_the_ring_empties_first()
fn drain_of_an_empty_ring_yields_nothing()
fn draining_at_the_same_point_is_deterministic()
  tests in the file: 19
```

`Split` fronts `Ring`, `Ends` fronts `Ends`, `Producer` and `Consumer` front
their namesakes. `Drain` fronts a declaration that is not there.

**That makes it the only place a bug in this crate can live.** A wrong answer
from `try_recv` is `ring_core`'s; a wrong answer from `Drain::next` is this
crate's, because the decrement, the zero check and the snapshot are all written
here. Four of the crate's eighteen tests are named for it, which is a defensible
ratio against a surface of one method — but the ratio is invisible without this
instance, and a reader looking at eighteen tests over seventeen declarations
would reasonably conclude the coverage is spread evenly when it is concentrated
on the one-twelfth of the surface that can be wrong.
