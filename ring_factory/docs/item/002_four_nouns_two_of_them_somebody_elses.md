# Item: Four Nouns, Two of Them Somebody Else's

### Scope

**Purpose:** Catalogue the four types a consumer of this crate names — `Factory`,
`BuildError`, and the two re-exports that close the export Contract — and record
what each costs, including the one that costs nothing and the one that costs
exactly as much as the error it relays.

**Responsibility:** `pub struct Factory`, `pub enum BuildError`, `pub use
ring_config::RingConfig`, `pub use ring_registry::Registry`.

**In Scope:** `ring_factory/src/lib.rs`, the two `pub use` lines and the
two type declarations; the measured widths of all four.

**Out of Scope:** The three verbs are [`item/001`](001_three_verbs_one_of_them_conditional.md).
`BuildError`'s variant semantics and the retraction behind them are
[`type/002`](../type/002_build_error.md); this instance is about the four names
and their widths, not about what the variants mean.

---

## The Four

```sh
cd "$(git rev-parse --show-toplevel)"
# the two re-exports and the two declarations, anchored on the keyword rather
# than on a line number: the module comment above them grows
command grep -E '^pub (use|struct|enum) ' ring_factory/src/lib.rs
```

Live output:

```
pub use ring_config::RingConfig;
pub use ring_registry::Registry;
pub struct Factory;
pub enum BuildError
```

| Name | Declared where | Reached by a consumer as | Width |
|------|----------------|--------------------------|------:|
| `Factory` | here | `ring_factory::Factory` | 0 |
| `BuildError` | here | `ring_factory::BuildError` | 24 |
| `RingConfig` | `ring_config` | `ring_factory::RingConfig` | 32 |
| `Registry` | `ring_registry` | `ring_factory::Registry` | — |

Two of the four names a consumer imports from this crate are not this crate's.
That is the export Contract working as ruled: a consumer bound to the five
Contract crates could not otherwise name `build`'s only argument, nor retrieve
what `build_named` stored.

---

### FC27 — The Only Type This Crate Declares That a Consumer Constructs Is Zero Bytes and Still Moves

`Factory` is a fieldless struct, so it occupies nothing, and the crate's own doc
comment explains the design: no fields, because a field would be a second input
that `invariant/001` forbids, and no `Default`, so it has to be written as a
literal.

What is not explained is the derive list. `Factory` derives `Debug` and `Clone`
and **not `Copy`**, so a bound `Factory` moves:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -B1 'pub struct Factory;' ring_factory/src/lib.rs | tail -2
echo '  -- what that costs, measured --'
mkdir -p ./-fc_item_probe/src
printf '%s\n' '[workspace]' '[package]' 'name = "fc_item_probe"' 'version = "0.0.0"' 'edition = "2021"' 'publish = false' '' '[dependencies]' 'ring_factory = { path = "../ring_factory" }' > ./-fc_item_probe/Cargo.toml
cat > ./-fc_item_probe/src/main.rs <<'PROBE'
use ring_factory::{ BuildError, Factory, RingConfig };
fn main()
{
  println!( "size_of::<Factory>()    = {}", core::mem::size_of::< Factory >() );
  println!( "size_of::<BuildError>() = {}", core::mem::size_of::< BuildError >() );
  println!( "size_of::<RingConfig>() = {}", core::mem::size_of::< RingConfig >() );
  let a = Factory;
  let b = a.clone();
  let moved = a;
  println!( "clone and move of a 0-byte value both compile: {b:?} {moved:?}" );
}
PROBE
cargo run --quiet --manifest-path ./-fc_item_probe/Cargo.toml 2>&1 | tail -4
rm -rf -- ./-fc_item_probe
```

Live output:

```
#[ derive( Debug, Clone ) ]
pub struct Factory;
  -- what that costs, measured --
size_of::<Factory>()    = 0
size_of::<BuildError>() = 24
size_of::<RingConfig>() = 32
clone and move of a 0-byte value both compile: Factory Factory
```

`Clone` on a zero-sized fieldless struct returns a value indistinguishable from
writing `Factory` again, so it grants nothing the literal does not. Withholding
`Copy` while granting `Clone` is the combination with the least to recommend it:
it makes `Factory` move out of a binding — a helper taking `factory : Factory`
by value leaves the caller's binding unusable — for a value that carries no state
to be moved.

The crate argues carefully for the absence of `Default` and says nothing about
either derive. That is the asymmetry: the *withheld* trait is justified, the two
*granted* ones are not, and the granted pair is the one that produces a
borrow-checker error a reader cannot account for from the documentation.

---

### FC28 — `BuildError` Costs Exactly What It Relays, and Its Own Variant Is Free

`BuildError` has two variants — a unit and one carrying `RingError` — and
measures the same 24 bytes as the payload alone:

```sh
cd "$(git rev-parse --show-toplevel)"
mkdir -p ./-fc_width_probe/src
printf '%s\n' '[workspace]' '[package]' 'name = "fc_width_probe"' 'version = "0.0.0"' 'edition = "2021"' 'publish = false' '' '[dependencies]' 'ring_factory = { path = "../ring_factory" }' 'ring_types = { path = "../ring_types" }' > ./-fc_width_probe/Cargo.toml
cat > ./-fc_width_probe/src/main.rs <<'PROBE'
use ring_factory::BuildError;
use ring_types::RingError;
fn main()
{
  println!( "RingError                    = {}", core::mem::size_of::< RingError >() );
  println!( "BuildError                   = {}", core::mem::size_of::< BuildError >() );
  println!( "Result< (), BuildError >     = {}", core::mem::size_of::< Result< (), BuildError > >() );
}
PROBE
cargo run --quiet --manifest-path ./-fc_width_probe/Cargo.toml 2>&1 | tail -3
rm -rf -- ./-fc_width_probe
```

Live output:

```
RingError                    = 24
BuildError                   = 24
Result< (), BuildError >     = 24
```

Wrapping `RingError` in a second enum with an extra unit variant is free: the
discriminant fits in a niche the payload already had. So the relay this crate
chose over re-spelling the refusal costs nothing in width, which is the
strongest available argument for the choice and is not the argument the source
makes — the source argues correctness ("the backend's vocabulary survives the
relay") and never mentions that the wrapper is weightless.

The third line is the one worth keeping. `build_named` succeeds with `()` and
still returns 24 bytes, because a `Result` is as wide as its widest arm: the
success path of the crate's only registering method carries the full width of an
error it did not raise.

---

### Sources

| Source | What it establishes |
|--------|---------------------|
| `ring_factory/src/lib.rs` | The two `pub use` lines, the two declarations, and `Factory`'s derive list |
| A `cargo run` probe over this crate's own public surface | All four widths, and that `Clone`-without-`Copy` compiles both a clone and a move |

### Tests

| Test | What it holds |
|------|---------------|
| `the_contract_surface_is_reachable_without_naming_a_non_contract_crate` | The two re-exports are load-bearing: the Contract surface is usable without naming `ring_config` or `ring_registry` |
| `two_factories_build_identically` | A second `Factory` value is interchangeable with the first, which is what makes the derive list a question about ergonomics rather than about behaviour |
