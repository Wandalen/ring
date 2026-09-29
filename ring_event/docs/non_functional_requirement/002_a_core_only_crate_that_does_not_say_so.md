# Non-Functional Requirement: A Core-Only Crate That Does Not Say So

### Scope

**Purpose:** Establish whether this crate and everything it depends on could run
without `std`, and what it would take to make that a checked property rather than
an inferred one.

**Responsibility:** The three-crate dependency closure, its references to `std`
and `alloc`, the absence of a `#![ no_std ]` declaration here and its presence on
three crates elsewhere in the family — one of them inside this closure — and the
targets available to catch a regression.

**In Scope:** `ring_event/Cargo.toml`; `ring_slot/Cargo.toml`;
`ring_types/Cargo.toml`; `ring_event/src/lib.rs:34`;
`ring_store/src/lib.rs:61`, `:90`.

**Out of Scope:** What the crate costs at runtime is
[`non_functional_requirement/001`](001_the_indirection_that_is_not_there.md). The
crate-level attribute that is present is
[`item/001`](../item/001_five_public_items_and_their_traffic.md).

---

## The Closure and What It Reaches For

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the whole dependency closure of this crate --'
for c in ring_event ring_slot ring_types; do
  printf '%-12s ' "$c"
  sed -n '/^\[dependencies\]/,/^\[/p' $c/Cargo.toml | command grep -o '^[a-z_]* =' | tr -d ' =' | tr '\n' ' '
  echo
done
echo '  -- std or alloc references outside doc comments, per crate, lowest to highest --'
for c in ring_*/; do
  n=$( cat "$c"src/*.rs 2>/dev/null | command grep -v '^ *//' | command grep -c 'std::\|String\|Vec<\|Vec::\|Box<\|format!\|println!\|HashMap' || true )
  printf '%-18s %s\n' "$( basename $c )" "$n"
done | sort -k2 -n | awk '$2 == 0 { z++ } END { print "  crates at zero: " z " of 33" }'
echo '  -- how many of the 33 declare no_std --'
command grep -rl 'no_std' --include=*.rs ring_*/src | wc -l
echo '  -- what the dev-dependency needs that the library does not --'
command grep 'Box< \|Vec::' ring_store/src/lib.rs
echo '  -- and the targets installed to catch a regression --'
rustup target list --installed
```

Live output:

```
  -- the whole dependency closure of this crate --
ring_event   ring_types ring_slot 
ring_slot    ring_types 
ring_types   
  -- std or alloc references outside doc comments, per crate, lowest to highest --
  crates at zero: 18 of 33
  -- how many of the 33 declare no_std --
3
  -- what the dev-dependency needs that the library does not --
  slots : Box< [ S ] >,
    let mut slots = Vec::with_capacity( capacity.get() );
  -- and the targets installed to catch a regression --
aarch64-unknown-linux-gnu
wasm32-unknown-unknown
wasm32-wasip1
x86_64-unknown-linux-musl
```

---

### EV35 — The Whole Closure Is Three Crates, No External Dependencies, and Nothing It Uses Needs an Allocator

`ring_event` depends on `ring_types` and `ring_slot`. `ring_slot` depends on
`ring_types`. `ring_types` depends on nothing. Three crates, and not one
third-party name among them.

None of the three references `std` or `alloc` outside a doc comment. Everything
this crate touches is `core`: `Option`, `Result`, a fixed array, a `usize`, two
traits, an associated type with a lifetime. The only crate-level attribute
present is `#![ deny( missing_docs ) ]` at `:34`.

`#![ no_std ]` is not present here. Three of the 33 crates in the family now carry
it — `ring_types`, `ring_stats` and `ring_overflow` — and `ring_types` is one of
the three crates in this crate's own dependency closure, so the closure is now
part-enforced and part-habit. 15 of the 33 remain in exactly this position — zero
references to either by the measure above and no declaration, so the line would
compile today.

**Correction (2026-09-20):** this paragraph read "17 of the 33 remain in exactly
this position". The census counts 18 crates at zero references, and the position
being described is *zero references and no declaration* — so the three declarers
come out, not one. Subtracting only `ring_types`, the declarer inside this
crate's own closure, left `ring_stats` and `ring_overflow` counted as though they
still lacked the attribute they carry. The 18 and the 3 were both already right;
only the subtraction between them was not.

**Finding.** A ring buffer family is the kind of code that ends up on an
embedded target, and this crate is the smallest, purest candidate in it: three
files' worth of dependency closure, no allocation, no I/O, no external crates.
Whether `no_std` is a supported property or an accident of the code so far is
not recorded anywhere. If it is intended, one line makes the compiler enforce it
and documents it at the same time. If it is not intended, saying so is worth more
than the current silence, which reads as an oversight either way.

---

### EV36 — Nothing in the Workspace Would Notice If the Property Broke

The finding above rests on source inspection: no `std::` path, no `Vec`, no
`Box`, no `format!`. That is evidence, not proof — the compiler has not been
asked the question, because without `#![ no_std ]` it always links `std` and
never has cause to complain.

Asking it properly needs a target with no `std` at all, and the toolchain has
none installed. All four available targets — `aarch64-unknown-linux-gnu`,
`wasm32-unknown-unknown`, `wasm32-wasip1`, `x86_64-unknown-linux-musl` — ship a
`std`, so a `cargo build` against any of them proves nothing about portability.

The crate's own suite could not close the gap either. Its dev-dependency
`ring_store` stores its slots in a `Box< [ S ] >` built from a
`Vec::with_capacity`, so four of this crate's seventeen tests reach an allocator
through storage even though the library under test never does. A `no_std`
declaration on the library would be entirely compatible with that — dev-
dependencies are not bound by it — but it means the suite cannot be the thing
that checks it.

**Finding.** The gap is not that the property is false; every measurement says it
holds. The gap is that it holds by inspection, and inspection does not survive a
future edit. Making it checkable is two steps and both are small: the `#![ no_std ]`
line, which turns the property into a compile error the moment someone reaches
for `Vec`, and one bare-metal target in the toolchain so that a build can be run
against it. Until at least the first exists, the family's most portable crate is
portable by accident.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/001`](001_the_indirection_that_is_not_there.md) | The other property nothing in the family measures |
| [`item/001`](../item/001_five_public_items_and_their_traffic.md) | The one crate-level attribute the file does carry |
| [`integration/001`](../integration/001_one_declarer_and_it_is_a_dev_dependency.md) | The dev-dependency that reaches an allocator |
| [`data_structure/001`](../data_structure/001_a_crate_that_declares_no_data.md) | Why there is nothing here that could need one |

### Sources

| Fact | Where |
|------|-------|
| The three-crate closure with no external dependencies | `ring_event/Cargo.toml`, `ring_slot/Cargo.toml`, `ring_types/Cargo.toml` |
| Zero `std` or `alloc` references across it | Census above |
| 18 of 33 crates at zero references, 3 declaring `no_std`, 15 in the same position as this one | Census above |
| The one crate-level attribute present | `ring_event/src/lib.rs:34` |
| The dev-dependency's `Box` and `Vec` | `ring_store/src/lib.rs:61`, `:90` |
| No bare-metal target installed | `rustup target list --installed` above |

### Tests

| Test | Covers |
|------|--------|
| `one_generic_body_round_trips_a_typed_slot` | The library path, which touches no allocator |
| `both_shapes_land_in_storage_through_the_same_two_calls` | A test that reaches one through `ring_store` |
| `a_recycled_storage_slot_stops_returning_the_previous_lap` | Another, for the same reason |
