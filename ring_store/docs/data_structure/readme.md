# data_structure

The struct is two fields and three machine words, and it is the same three words
whether a slot costs four bytes or four kilobytes. Everything that scales lives
behind the pointer; the handle itself is constant, which is what lets two rings
with different protocols embed one by value and still have comparable layouts.

The first instance measures that constancy and then examines how the crate proves
the harder half of the reached-test — *holds no cursor and no ordering state*, a claim
about absence that no test can establish by exercising it. The suite's answer is
a `size_of` assertion computed from the permitted fields, plus a scrambled-order
sweep, plus a module comment that names exactly what each stand-in misses. The
second instance takes the other structural fact: the impl blocks split three
ways — `Default`, `Slot + Default`, no bound — and because `new` is the only way
in, *its* bound is the whole type's storability requirement. That bound is
`Default`, not the conspicuous `Slot` — a requirement `ring_slot`, which defines
the trait, never mentions.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Three Words, Whatever the Slot Costs](001_three_words_whatever_the_slot_costs.md) | BF18, BF19 — a handle whose size is independent of both parameters, and an absence pinned at one instantiation |
| 002 | [The Default Bound Is a Bound on the Type](002_the_default_bound_is_a_bound_on_the_type.md) | BF20, BF21 — a storability bound that is `Default` rather than the conspicuous `Slot`, and a family-wide requirement written in one consumer's prose |

### The Layout

| | Bytes | What |
|---|------:|------|
| `slots : Box< [ S ] >` | 16 | Pointer and length |
| `capacity : Capacity` | 8 | The validated power of two |
| **`Buffer< S >`** | **24** | The same at `TypedSlot< u32 >` and at `BytesSlot< 4096 >` |

### How an Absence Is Asserted

| Stand-in | Catches | Misses |
|----------|---------|--------|
| `size_of` against the two permitted fields | A third field of any non-zero size | A zero-sized ordering field; a size that varies with `S` |
| A sixteen-sequence scrambled write sweep | A buffer with an opinion about order | A cursor consulted only under contention |
| A human reading thirty legible lines | Everything | Nothing — but it is not automated |

The suite states all of this itself, in its module comment, and closes with
"These tests catch the drift, not the original sin." The one hole it does not
name is that the `size_of` assertion runs at a single slot type, so the
independence from `S` measured above is real and unasserted.

### Two Bounds, One Reachable

```rust
impl< S : Slot + Default > Buffer< S >   // new, clear, all_empty
impl< S > Buffer< S >                    // the other nine
```

`new` is the only constructor, the fields are private, and no `From` impl exists
— so the nine unbounded functions are reachable only for an `S` that already
satisfied `Slot + Default`. A shape implementing `Slot` alone is rejected with an
error pointing at `ring_store/src/lib.rs:65`, and `ring_slot` — where a shape
author is working — never mentions `Default` on the trait, on either shipped
shape, or in its module comment.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the struct
command grep -m1 -A5 -F '#[ derive( Debug ) ]' ring_store/src/lib.rs

# the two inherent impl blocks and their bounds
grep -n '^impl< S' ring_store/src/lib.rs

# how the suite accounts for the absence clause, and the assertion itself
command grep -m1 -A16 -F '//! Three of those four clauses are ordinary. The fourth — *holds no cursor and' ring_store/tests/buffer_test.rs
command grep -m1 -A8 -F '  // The stand-in for "holds no cursor and no ordering state": a boxed slice' ring_store/tests/buffer_test.rs

# every documentation mention of Default across both crates
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'Default' ring_store/src/lib.rs ring_slot/src/lib.rs \
  | grep -E '///|//!' | sed 's|^/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sort

# and the impls that satisfy it
grep -n 'impl.*Default for' ring_slot/src/lib.rs
```

`size_of` figures and the `Slot`-without-`Default` compile error come from
probes; both are quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF18 | `ring_store` | **measured cost** | The handle is 24 bytes at every capacity and every slot size, so a buffer contributes a fixed footprint to each ring and the benched candidates differ only in protocol state |
| BF19 | `ring_store` | n/a — coverage | The `size_of` stand-in for "holds no cursor" is instantiated at one slot type, so a field whose size depended on `S` could pass; the constancy BF18 measures is asserted nowhere |
| BF20 | `ring_store` | n/a — observation | `impl< S > Buffer< S >` advertises nine functions for any slot shape, but `new` is the only constructor, so the stronger `Slot + Default` bound governs the whole type |
| BF21 | `ring_slot` | **latent hazard** | Every slot shape must implement `Default`, stated once in this crate's prose inside a sentence about `unsafe`; the crate defining `Slot` never mentions it, so a new shape fails to compile pointing at a file its author never opened |
