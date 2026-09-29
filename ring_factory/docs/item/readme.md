# item

Nine declarations, four of which a consumer names and three of which do work.
The catalogue splits on that line: the verbs are one instance, the nouns are the
other, and both instances find the same shape of gap — something the crate
argues for carefully sitting next to something equivalent it never mentions.

The verbs turn up a door that is closed on every default build in the tree, and
the one method whose refusal set is its own rather than a relay — which is also
the one whose cheap-looking error costs a full ring construction first. The nouns
turn up a zero-byte type that still moves, and a two-variant error that is
exactly as wide as the single variant it wraps.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Three Verbs, One of Them Conditional](001_three_verbs_one_of_them_conditional.md) | FC25, FC26 — a third of the surface compiled out by default, and the one refusal set that is a union rather than a forward |
| 002 | [Four Nouns, Two of Them Somebody Else's](002_four_nouns_two_of_them_somebody_elses.md) | FC27, FC28 — `Clone` without `Copy` on a zero-byte value, and a wrapper that costs nothing over its payload |

### The Nine, by Kind

| Kind | Declaration | Public | Notes |
|------|-------------|--------|-------|
| noun | `pub struct Factory` | yes | Zero-sized; `Debug + Clone`, no `Copy`, no `Default` |
| noun | `pub enum BuildError` | yes | Two variants, 24 bytes — exactly `RingError`'s width |
| noun | `pub use ring_config::RingConfig` | yes | Not this crate's; re-exported so `build`'s argument is nameable |
| noun | `pub use ring_registry::Registry` | yes | Not this crate's; re-exported so `build_named`'s result is retrievable |
| verb | `pub fn build` | yes | Relays `ring_core`'s one refusal |
| verb | `pub fn build_named` | yes | The union: relays, and adds `NameTaken` |
| verb | `pub fn build_crossbeam` | **conditional** | `#[ cfg( feature = "crossbeam" ) ]` — absent by default |
| impl | `impl Display for BuildError` | — | Interpolates the relayed payload into the message |
| impl | `impl Error for BuildError` | — | Empty body, so `source()` is `None` — [`integration/002`](../integration/002_the_crate_the_export_surface_routes_through.md) FC20 |

### Why the Count in This Crate's Own Documentation Is Right for the Wrong Reasons

`docs/readme.md` says the crate "now has six" declarations and quotes a grep to
prove it. The grep returns six. They are not the six:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the census in docs/readme.md actually matches --'
command grep -E '^\s*(pub )?(fn|struct|enum|trait|type|const) ' ring_factory/src/lib.rs
echo '  -- what it silently drops --'
command grep -E '^pub use ' ring_factory/src/lib.rs
```

Live output:

```
  -- what the census in docs/readme.md actually matches --
pub struct Factory;
  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
  pub fn build_named< S : Send >
  pub fn build_crossbeam< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
pub enum BuildError
  fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result
  -- what it silently drops --
pub use ring_config::RingConfig;
pub use ring_registry::Registry;
```

The sixth match is `fn fmt` — a private method inside a trait impl, not a
declaration a consumer can reach — and the two `pub use` re-exports are missed
entirely, because the pattern has no `use` alternative. Two errors that happen to
cancel. The number is defensible and every item behind it is wrong, which is the
harder failure to notice: a census that disagreed with the prose would have been
fixed, and one that agrees is never re-read.

This instance's own tables are built from the two anchored greps above rather
than from that pattern, which is why they report nine and not six.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

echo '  -- the four nouns --'
command grep -nE '^pub (use|struct|enum) ' ring_factory/src/lib.rs

echo '  -- the three verbs, with whatever gates them --'
command grep -n -B2 '^  pub fn build' ring_factory/src/lib.rs | command grep -E 'cfg|pub fn'

echo '  -- the two impls --'
command grep -nE '^impl .* for BuildError' ring_factory/src/lib.rs

# the feature that decides whether the surface is three verbs or two. Summed in
# `awk` on the field after the last colon, because a multi-file `grep -c` prints
# `path:count` per file rather than a bare number — and reported through
# `printf`, since `grep -c` exits 1 on a zero count and a future change removing
# the gate would otherwise fail this block for the wrong reason
printf '  crossbeam gates in src and tests: %s\n' \
  "$( command grep -c 'cfg( feature = "crossbeam" )' \
       ring_factory/src/lib.rs ring_factory/tests/factory_test.rs \
     | awk -F: '{ n += $NF } END { print n + 0 }' )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC25 | `ring_factory` | n/a — coverage | `build_crossbeam` and both of its tests are behind `#[ cfg( feature = "crossbeam" ) ]`, no manifest in the workspace enables it, and a default `cargo test -p ring_factory` reports green having compiled neither |
| FC26 | `ring_factory` | n/a — doc gap | `build_named` is the crate's only union refusal set, and its `NameTaken` arm costs a full 320-byte `Split` construction and drop before returning a 24-byte error — stated in a doc comment's sequencing clause and nowhere a caller would look |
| FC27 | `ring_factory` | n/a — observation | `Factory` derives `Clone` without `Copy`, so a zero-byte stateless value moves out of a binding; the crate argues at length for the absent `Default` and never mentions either granted derive |
| FC28 | `ring_factory` | n/a — observation | `BuildError` is 24 bytes, identical to the `RingError` it wraps, so the relay is free — the strongest argument for the design, and one the source never makes |
