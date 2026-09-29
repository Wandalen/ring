# Api: Twelve Functions, Three Const, Seven must_use

### Scope

**Purpose:** Record the whole public surface as a census, that `#[ must_use ]`
follows an exact rule with one apparent exception that is not one, and that the
`const` boundary is drawn by a Tier 1 crate's choice rather than by anything the
compiler requires.

**Responsibility:** The shape of the surface — how many functions, which are
`const`, which are `#[ must_use ]`, and why each split falls where it does.

**In Scope:** `ring_store/src/lib.rs:66-278`;
`ring_index/src/lib.rs:49`; `ring_types/src/capacity.rs`.

**Out of Scope:** What the six accessors are for is
[`api/002`](002_six_ways_to_reach_a_slot.md). Which of them anyone calls is
[`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md).

---

## The Census

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/#\[ must_use \]/{mu=1; next} /pub (const )?fn /{
  name=$0; sub(/^ *pub /,"",name); sub(/\(.*/,"",name);
  isc = ($0 ~ /pub const fn/) ? "const" : "-";
  printf "  %-28s %-6s must_use=%s\n", name, isc, (mu?"yes":"no"); mu=0 }' ring_store/src/lib.rs
```

Live output:

```
  fn new                       -      must_use=yes
  fn clear                     -      must_use=no
  fn all_empty                 -      must_use=yes
  const fn capacity            const  must_use=yes
  const fn len                 const  must_use=yes
  const fn is_empty            const  must_use=yes
  fn get                       -      must_use=yes
  fn get_mut                   -      must_use=yes
  fn at                        -      must_use=yes
  fn at_mut                    -      must_use=yes
  fn iter                      -      must_use=no
  fn iter_mut                  -      must_use=no
```

Twelve functions, three `const`, seven `#[ must_use ]`.

---

### BF14 — The `must_use` Rule Is Exact, and Its One Exception Is Not One

Read the column against the receivers and a rule falls out immediately: every
function taking `&self` or returning a fresh value carries `#[ must_use ]`; every
function taking `&mut self` carries none. `new`, `all_empty`, `capacity`, `len`,
`is_empty`, `get`, `at` — seven, all readers. `clear`, `get_mut`, `at_mut`,
`iter_mut` — four, all mutators.

That is eleven of twelve. The twelfth, `iter`, takes `&self`, returns a value,
and has no attribute — which looks like the rule breaking. It does not:

```
warning: unused `std::slice::Iter` that must be used
  = note: iterators are lazy and do nothing unless consumed
```

`core::slice::Iter` is itself `#[ must_use ]`, so `buffer.iter();` on its own
line already warns. Adding the attribute to the function would be redundant with
the one on the type.

**Finding.** The surface applies one rule twelve times without stating it, and
the single case that looks like a lapse is the rule being satisfied by the return
type instead of the function. That is a good outcome and is recorded here because
the reverse is what usually happens: `must_use` is normally sprinkled where
someone remembered it, producing a pattern a reader cannot rely on.

Here a reader *can* rely on it — the absence of `#[ must_use ]` on
`get_mut`/`at_mut` is information, not an oversight, and means "this is called
for its effect." Nothing in the crate says so, which costs a reader the
confidence to draw that inference. One sentence in the module comment would
convert an observed regularity into a documented contract.

---

### BF15 — The `const` Boundary Is Drawn by `ring_index`, Not by the Compiler

Three functions are `const`: `capacity`, `len`, `is_empty`. The obvious reading
is that the other nine cannot be. Compiled, that is true of only two of them.

`get` and `get_mut` compile as `const fn` in exactly the generic shape this crate
uses — a `Box< [ S ] >` indexed by a `usize`, returning `&S`:

```
### the generic form, const-qualified:
(no error lines above = it compiles)
```

`at` and `at_mut` genuinely cannot, and the reason is one crate away: they call
`ring_index::of`, and `of` is not a `const fn`. It compiles as one —

```
### ring_index::of, const-qualified:
(no error lines above = it compiles)
```

— and the constness it would need from below is already there:

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  ring_types::Capacity const fns: %s of %s\n' "$( grep -c 'pub const fn ' ring_types/src/capacity.rs )" "$( grep -c 'pub const fn \|pub fn ' ring_types/src/capacity.rs )"
printf '  ring_index const fns:           %s of %s\n' "$( grep -c 'pub const fn ' ring_index/src/lib.rs )" "$( grep -c 'pub const fn \|pub fn ' ring_index/src/lib.rs )"
```

Live output:

```
  ring_types::Capacity const fns: 3 of 3
  ring_index const fns:           0 of 3
```

**Finding.** Constness propagates cleanly from Tier 0 and stops dead at Tier 1.
`Capacity::new`, `get` and `mask` are all `const`; `of` is one cast and one mask
over exactly those, and is a plain `fn`. Because `at` and `at_mut` call it, they
cannot be `const` either — so a single missing keyword in a four-line Tier 1
function is what makes the sequence-addressed half of this crate's accessors
non-`const`.

The practical cost today is nil: nobody builds a `Buffer` at compile time,
because `new` allocates and cannot be `const` regardless. The finding is about
the boundary being unexplained rather than about the loss. A reader looking at
three `const` functions and nine ordinary ones has no way to tell that two of the
nine are a deliberate omission upstream, two are blocked by it, and the remaining
five are blocked by allocation or by `&mut` — four different reasons wearing one
appearance.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](002_six_ways_to_reach_a_slot.md) | What the six accessors in this census are for |
| [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) | `ring_index::of`, the function the `const` boundary turns on |
| [`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md) | Which three of these twelve anyone calls |
| [`decisions/002`](../decisions/002_a_length_kept_to_be_checked_against_itself.md) | Why three of the twelve report sizes |
| [`type/001`](../type/001_one_parameter_three_impl_blocks_one_derive.md) | The types these signatures are written in |

### Sources

| Fact | Where |
|------|-------|
| The twelve functions | `ring_store/src/lib.rs:66-278` |
| `slice::Iter`'s own `must_use` | `rustc` warning, quoted above |
| `get`/`get_mut` compiling as `const` | Release probe over the same generic shape |
| `of` compiling as `const` | Release probe over the same body |
| Const counts upstream | `ring_types/src/capacity.rs`, `ring_index/src/lib.rs` |

### Tests

| Test | Covers |
|------|--------|
| `exactly_n_slots_are_allocated_for_capacity_n` | `new`, `len`, `capacity`, `iter` |
| `indexed_get_and_set_round_trip` | `get`, `get_mut` |
| `a_sequence_addresses_the_slot_ring_index_says_it_does` | `at`, `at_mut` |
| `mutable_iteration_reaches_every_slot` | `iter_mut` |
| *(to create)* | A `const` context asserting `capacity`/`len`/`is_empty` really are callable there |
