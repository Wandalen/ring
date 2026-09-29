# Item: Five Nouns, Four of Them the Same Width as What They Wrap

### Scope

- **Purpose**: Catalogue the crate's five public structs and establish what each one costs relative to the `ring_core` type it stands in front of.
- **Responsibility**: State each noun's shape, its wrapped type, and its width.
- **In Scope**: The five `pub struct` declarations; their single fields; the one that has two.
- **Out of Scope**: The methods on them (→ [`item/002`](002_twelve_verbs_eight_bare_forwards.md)); why the wrapping exists at all (→ [`pattern/001`](../pattern/001_enforce_by_withholding.md)).

### The Five

Four are single-field newtypes. One is not.

| Noun | Field | Wraps |
|------|-------|-------|
| `Split< T >` | `ring : Ring< T >` | The whole ring, by value |
| `Ends< 'a, T >` | `inner : ring_core::Ends< 'a, T >` | A borrow of the ring |
| `Producer< 'a, T >` | `inner : ring_core::Producer< 'a, T >` | The publishing end |
| `Consumer< 'a, T >` | `inner : ring_core::Consumer< 'a, T >` | The draining end |
| `Drain< 'c, 'a, T >` | `consumer : &'c mut Consumer< 'a, T >`, `remaining : usize` | Nothing — it is this crate's own |

**`Drain` is the exception in two ways at once**: it is the only noun with more
than one field, and the only one with no `ring_core` counterpart. Every other
struct in this crate exists to *re-name* something; `Drain` exists to *hold* a
count that nothing below it keeps.

### Derives

`Debug` on all five, nothing else. No `Clone` anywhere — which is the whole
point for `Producer` and `Consumer`
(→ [`invariant/001`](../invariant/001_capability_follows_the_handle.md)) and
merely consistent for the other three, since `Split` owns a ring and `Ends` and
`Drain` hold borrows.

### Items

| File | Relationship |
|------|--------------|
| [002_twelve_verbs_eight_bare_forwards.md](002_twelve_verbs_eight_bare_forwards.md) | The methods these nouns carry |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | The shape the two handle nouns realise |
| [../data_structure/002_the_one_struct_that_is_not_a_newtype.md](../data_structure/002_the_one_struct_that_is_not_a_newtype.md) | `Drain`, worked out |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer.md](../type/001_producer.md) | The publishing noun's definition and validation |
| [../type/002_consumer.md](../type/002_consumer.md) | The draining noun's |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_enforce_by_withholding.md](../pattern/001_enforce_by_withholding.md) | Why four of the five are newtypes rather than re-exports |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The five declarations and their fields |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | The four types wrapped |

### Tests

| Test | Relationship |
|------|--------------|
| `the_wrapper_costs_nothing` | Asserts the width equality — for two of the four wrappers |
| `every_handle_can_be_printed` | The `Debug` derive, exercised on the nouns that carry it |

### HD25 — The Zero-Cost Claim Is Tested for the Two Handles That Cannot Grow and Not for the One That Can

Four structs hold one field each, one holds two:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- fields per noun --'
for s in Split Ends Producer Consumer Drain; do
  printf '  %-10s %s\n' "$s" \
    "$( sed -n "/^pub struct $s/,/^}/p" ring_handle/src/lib.rs | command grep -cE '^  [a-z_]+ :' )"
done
echo '  -- which of them the width assertion covers --'
sed -n '/^fn the_wrapper_costs_nothing/,/^}/p' ring_handle/tests/handle_test.rs \
  | command grep -oE 'ring_(handle|core)::[A-Za-z]+' | sort -u
```

Live output:

```
  -- fields per noun --
  Split      1
  Ends       1
  Producer   1
  Consumer   1
  Drain      2
  -- which of them the width assertion covers --
ring_core::Consumer
ring_core::Producer
ring_handle::Consumer
ring_handle::Producer
```

A single-field struct with no `repr` attribute and nothing to pad is laid out as
its field, which is why `Split< u32 >` measures 320 bytes against
`ring_core::Ring< u32 >`'s 320, `Ends` 256 against 256, `Producer` 24 against
24, and `Consumer` 16 against 16. The whole crate is free at runtime and its
entire cost is at compile time.

**The test asserting that covers `Producer` and `Consumer` only.** `Split` and
`Ends` are not in it, and they are the two where the assertion would earn
something: `Producer` and `Consumer` are borrows that no plausible edit widens,
whereas `Split` owns the ring by value and is exactly where a future field — a
generation stamp, a closed flag, a name — would be put. The two structs under
test are the two that cannot grow.

That is not a defect in the test, which was written for a narrower claim and
says so in its own doc comment. It is a gap between what the test asserts and
what `pattern/001`'s argument rests on: withholding beats checking *because the
wrapping is free*, and two of the four wrappers have nothing watching them.

### HD26 — The One Struct With State Is the One With No Counterpart

`Drain` is not a wrapper and the catalogue is where that becomes visible:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- this crate nouns --'
command grep -E '^pub struct ' ring_handle/src/lib.rs | sed 's/<.*//'
echo '  -- ring_core nouns --'
command grep -E '^pub (struct|enum) ' ring_core/src/lib.rs | sed 's/<.*//'
echo '  -- and the trait impl that is not a forward --'
command grep -E '^impl.* for ' ring_handle/src/lib.rs
```

Live output:

```
  -- this crate nouns --
pub struct Split
pub struct Ends
pub struct Producer
pub struct Consumer
pub struct Drain
  -- ring_core nouns --
pub enum Backend
pub struct Ring
pub struct Ends
pub struct Producer
pub struct Consumer
  -- and the trait impl that is not a forward --
impl< T : Send > Iterator for Drain< '_, '_, T >
```

Three of this crate's five names — `Ends`, `Producer`, `Consumer` — appear
verbatim one crate down. `Split` fronts a differently-named `Ring`. `Drain`
fronts nothing. In the other direction `ring_core::Backend`, an enum reporting
which backend a ring selected, has no counterpart here.

**The two absences are the same absence seen from either side.** `Drain` adds a
bounded iteration `ring_core` has no notion of; `Backend` reports a fact this
crate deliberately declines to forward, so a `Split` cannot say what it wraps.
Both are consequences of the crate being a *narrowing*, and the catalogue is the
only place in the corpus where they sit next to each other.

The one rename is worth its own line: `Ring` becomes `Split`, and the new name
says what the value is *for* rather than what it *is*. That is the only place in
the crate where the wrapper changes a name rather than a capability.

`Drain` also carries the crate's only `impl Trait for Type` and its only
conditional branch. Every other line here is a forward.
