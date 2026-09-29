# api

Ten inherent functions and a two-method trait. Six of the ten are `const`, six
carry `#[ must_use ]`, and the two sets do not overlap the way consequence would
suggest: the annotation marks the five cheapest functions and skips both that
hand back an owned value with no other handle.

The trait is the other half of the story. It declares `is_empty` and `clear` and
nothing else, which draws the genericity boundary exactly where a payload type
would have to be named — the right place, since the two shapes disagree about
what a payload even is. The cost lands in the bounds: three of the four generic
consumers in the family add `Default` to `Slot`, because the trait offers no
constructor.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Ten Functions, Six `const`, Seven `must_use`](001_ten_functions_six_const_seven_must_use.md) | SL17, SL18 — an annotation set inverted relative to consequence, and a justification pointing at a policy the family refuses |
| 002 | [A Trait With Two Methods](002_a_trait_with_two_methods.md) | SL19, SL20 — a boundary drawn before the payload, and the `Default` every consumer adds back |

### The Whole Surface

| Function | Shape | `const` | `must_use` | Returns |
|----------|-------|:-------:|:----------:|---------|
| `empty` | `TypedSlot` | ✔ | ✔ | `Self` |
| `set` | `TypedSlot` | ✘ | ✘ | `Option< T >` — **an owned value** |
| `get` | `TypedSlot` | ✔ | ✘ | `Option< &T >` |
| `take` | `TypedSlot` | ✘ | ✘ | `Option< T >` — **an owned value** |
| `empty` | `BytesSlot` | ✔ | ✔ | `Self` |
| `capacity` | `BytesSlot` | ✔ | ✔ | `usize` |
| `len` | `BytesSlot` | ✔ | ✔ | `usize` |
| `is_empty` | `BytesSlot` | ✔ | ✔ | `bool` |
| `write` | `BytesSlot` | ✘ | ✘ | `Result< (), RingError >` |
| `read` | `BytesSlot` | ✘ | ✔ | `&[ u8 ]` |

`Slot::is_empty` and `Slot::clear` are trait items and appear in none of these
counts.

### The Inversion

The five annotated non-constructor functions — `capacity`, `len`, `is_empty`,
`read`, and both `empty` — all return values that are pure, cheap, and
recomputable. Ignoring one wastes a cycle.

`set` and `take` return `Option< T >`: an owned payload with no other handle.
Ignoring one destroys it, silently, with no warning under the workspace's lint
table, which does not enable `unused_results`
([`pattern/002`](../pattern/002_the_displaced_value_returned.md) finds three
shipped call sites doing exactly that).

### What the Trait Cannot Say

Neither trait method takes or returns a payload, so generic code holding an
`S : Slot` can construct, inspect, and recycle a slot but never fill or drain
one. `ring_event` supplies the missing half as two further traits, `Fill< S >`
and `Peek`, two tiers up
([`pattern/001`](../pattern/001_one_trait_two_shapes.md) SL38).

And because `Slot` declares no constructor, every consumer that must fill an
array of slots bounds on `Slot + Default` instead —
`ring_store`, `ring_mpsc`, and `ring_spsc`, three of the family's four generic
consumers.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the ten functions, with their const-ness
grep -n '^[[:space:]]*pub \(const \)\?fn ' ring_slot/src/lib.rs

# the six must_use attributes
grep -n 'must_use' ring_slot/src/lib.rs

# the two owned-value returns — the line above each is a doctest fence, not an attribute
grep -n -B1 'pub fn set(\|pub fn take(' ring_slot/src/lib.rs

# the trait, entire
command grep -m1 -A7 -F 'pub trait Slot' ring_slot/src/lib.rs

# every bound on Slot in the family
grep -rhoE 'Slot \+ [A-Za-z]+' ring_*/src/*.rs | sort | uniq -c
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL17 | `ring_slot` | **latent hazard** | `#[ must_use ]` covers the five cheapest functions and omits `set` and `take`, the only two returning an owned value whose loss is silent |
| SL18 | `ring_slot` | **wrong doc** | The policy `set`'s justification serves is unreachable — `ring_core` refuses `DropOldest` at construction, and the one call site that binds the value reads it to assert the opposite of what the sentence claims |
| SL19 | `ring_slot` | n/a — observation | Neither trait method moves a value, drawing the generic boundary exactly before the point where the two shapes' payload types diverge |
| SL20 | `ring_slot` | n/a — observation | Three of four generic consumers add `Default` to `Slot` because the trait declares no constructor, and the added bound costs `const` construction |
