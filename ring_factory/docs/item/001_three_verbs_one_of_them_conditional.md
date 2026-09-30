# Item: Three Verbs, One of Them Conditional

### Scope

**Purpose:** Catalogue the crate's three build methods one at a time — what each
takes, what each returns, what each refuses — and record that one of the three
is compiled out by default, so the public surface a consumer sees depends on a
cargo feature this crate's own documentation counts as though it did not.

**Responsibility:** `Factory::build`, `Factory::build_named`,
`Factory::build_crossbeam`.

**In Scope:** `ring_factory/src/lib.rs`, the three `pub fn` declarations
inside `impl Factory` and the `#[ cfg ]` above the third; the two feature
declarations that reach them, in `ring_factory/Cargo.toml` and
`ring_bench/Cargo.toml`.

**Out of Scope:** The four nouns are [`item/002`](002_four_nouns_two_of_them_somebody_elses.md).
What the surface *guarantees* — its stability, its error contract — is
[`api/001`](../api/001_the_build_surface.md) and
[`api/002`](../api/002_the_named_build_surface.md); this instance is about what
the three methods are, not what a consumer may rely on.

---

## The Three

```sh
cd "$(git rev-parse --show-toplevel)"
# anchored on the declarations themselves, with the two lines above each, so the
# `#[ cfg ]` that gates the third is visible rather than inferred
command grep -B2 '^  pub fn build' ring_factory/src/lib.rs
```

Live output:

```
  /// );
  /// ```
  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
--
  /// assert_eq!( registry.len(), 1 );
  /// ```
  pub fn build_named< S : Send >
--
  /// nothing in either signature stops it from being written.
  #[ cfg( feature = "crossbeam" ) ]
  pub fn build_crossbeam< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
```

| Method | Takes | Returns | Refuses | Compiled |
|--------|-------|---------|---------|----------|
| `build` | `RingConfig` | `Split< S >` | `Unsupported` | always |
| `build_named` | `RingConfig`, `&str`, `&mut Registry< S >` | `()` | `Unsupported`, `NameTaken` | always |
| `build_crossbeam` | `RingConfig` | `Split< S >` | nothing, today | **only under `crossbeam`** |

All three take `&self` on a zero-sized receiver, so the receiver is not an
argument in any sense that costs anything — it is there to give the family's
export Contract a noun ([`item/002`](002_four_nouns_two_of_them_somebody_elses.md)
FC27).

Two of the three return the ring's owner and one returns `()`. That asymmetry is
the whole of `build_named`: the ring it builds goes into the registry the caller
passed, so there is nothing left to hand back, and the name — not a returned
value — is how the caller reaches it again.

---

### FC25 — The Third Verb Is Compiled Out by Default, and Both Its Tests With It

`build_crossbeam` sits behind `#[ cfg( feature = "crossbeam" ) ]`, and so do the
only two tests that exercise it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'cfg( feature' ring_factory/src/lib.rs ring_factory/tests/factory_test.rs \
  | sed 's|ring_factory/||' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the two tests those cfgs gate --'
# `-A2`, not `-A1`: `#[ test ]` sits between the cfg and the signature, so a
# one-line window prints the attribute and reports zero gated tests
command grep -n -A2 'cfg( feature' ring_factory/tests/factory_test.rs | command grep -- '-fn ' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
src/lib.rs:  #[ cfg( feature = "crossbeam" ) ]
tests/factory_test.rs:#[ cfg( feature = "crossbeam" ) ]
tests/factory_test.rs:#[ cfg( feature = "crossbeam" ) ]
  -- the two tests those cfgs gate --
fn the_crossbeam_door_accepts_the_policy_the_in_house_backends_refuse()
fn the_two_doors_agree_on_capacity()
```

A default `cargo test -p ring_factory` compiles neither the method nor its
tests, and reports every remaining test as passing. Nothing in the output says
that a third of the crate's verbs was not built.

This is not a live defect: the family's own verification levels all pass
`--all-features`, so the two tests do run under every command the project
actually uses to grade itself. It is a defect in what a *reader* can conclude
from a green run they invoked themselves — the number of passing tests is the
same either way, and the difference is invisible unless you already know to look
for it.

The narrower and more durable half is that the feature is opt-in **and nothing
in the workspace opts in**. `ring_bench` forwards it (`crossbeam = [
"ring_factory/crossbeam" ]`) and no manifest enables it, so the door exists,
compiles under `--all-features`, and is closed on every default build in the
tree.

---

### FC26 — `build_named` Is the Only Verb Whose Refusal Set Is Not Its Callee's

`build` relays: its one refusal, `Unsupported`, is `ring_core`'s decision passed
through unchanged, and the crate documents that as deliberate — "duplicating the
backend's policy would silently go wrong the moment a fourth backend disagrees."
`build_crossbeam` relays too, and currently relays nothing, because
`Ring::new_crossbeam` refuses nothing.

`build_named` is the exception. It can refuse for a reason no callee raised —
`NameTaken` is this crate's own — and it can refuse for the relayed reason as
well, which makes it the one method in the crate whose error set is a union
rather than a forward:

```sh
cd "$(git rev-parse --show-toplevel)"
# every construction of a BuildError variant in the crate, with its origin
command grep 'BuildError::' ring_factory/src/lib.rs | command grep -v '///'
```

Live output:

```
//! [`BuildError::Unsupported`], never re-decided here: duplicating the
//! [`BuildError::NameTaken`] is the one refusal this crate owns outright.
        let ring = Ring::new(&cfg).map_err(BuildError::Unsupported)?;
            Err((RegistryError::NameTaken { .. }, _refused)) => Err(BuildError::NameTaken),
        let ring = Ring::new_crossbeam(&cfg).map_err(BuildError::Unsupported)?;
```

The consequence is a sequencing guarantee stated in the doc comment and nowhere
else: `Unsupported` is raised "before any ring exists", and `NameTaken` only
after one has been built and dropped. A caller who reads the two variants as
peers will assume both are cheap; one of them costs a full ring construction
first. `ring_handle::Split< u32 >` measures 320 bytes at this capacity
([`data_structure/002`](../data_structure/002_the_handle_pair_as_output.md)
FC12), so a name collision allocates, populates and drops that before returning
a 24-byte error.

---

### Sources

| Source | What it establishes |
|--------|---------------------|
| `ring_factory/src/lib.rs` | The three declarations, the `#[ cfg ]` on the third, and both `BuildError` construction sites |
| `ring_factory/tests/factory_test.rs` | The two feature-gated tests, and that no non-gated test names `build_crossbeam` |
| `ring_factory/Cargo.toml`, `ring_bench/Cargo.toml` | The feature is declared in both and enabled in neither |

### Tests

| Test | What it holds |
|------|---------------|
| `the_crossbeam_door_accepts_the_policy_the_in_house_backends_refuse` | The third verb's whole reason to exist — and is itself behind the feature |
| `the_two_doors_agree_on_capacity` | The two doors produce the same free capacity — likewise gated |
| `the_unnamed_path_can_never_return_name_taken` | `build`'s refusal set is strictly narrower than `build_named`'s |
| `both_paths_relay_the_same_refusal` | The relayed variant is identical across the two always-compiled verbs |
