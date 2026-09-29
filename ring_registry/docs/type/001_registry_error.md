# Type: RegistryError

### Scope

- **Purpose**: Define the one error the registry produces, and record why it is an enum with one variant and why it carries the name.
- **Responsibility**: The variant, its data, its derives, and the two shapes it was chosen over.
- **In Scope**: `RegistryError`.
- **Out of Scope**: The signature that returns it alongside the rejected ring (→ [`api/001`](../api/001_the_registry_surface.md)).

### Definition

```rust
pub enum RegistryError
{
  NameTaken { name : String },
}
```

Returned only by `register`, and only as the first element of
`( RegistryError, Split< T > )` — the pairing is `register`'s decision, not this
type's (→ [`api/001`](../api/001_the_registry_surface.md)).

#### One variant, and still an enum

A unit struct `NameTaken` would be smaller and would carry exactly the same
information today. The enum is chosen because `register` is the only fallible
operation *at present*: `get_mut` and `remove` return `Option`, because an absent
name is an ordinary answer rather than a failure. If a later operation acquires a
second failure mode, an enum absorbs it without changing any existing signature,
and a unit struct would force one.

This is a small bet on an uncertain future, and it is the kind that should
usually be refused (YAGNI). It is taken here because the cost is one line and the
alternative changes a public signature — the asymmetry is what justifies it, not
a prediction that the second variant will arrive.

#### It carries the name

`NameTaken { name : String }` rather than a bare `NameTaken`, so:

- A caller reporting the conflict does not have to have kept the name. This is
  the common case: `registry.register( config.name(), ring )` has no local
  binding to reach for afterwards.
- `Display` is specific without the caller formatting it — `a ring is already
  registered as "events"` rather than `name taken`.

**It is an owned `String`, not a `&str`.** `register` takes
`impl Into< String >`, so the name may have been constructed at the call site and
have no lifetime to borrow from. The clone happens only on the failure path,
alongside the one allocation `into()` already made — two small allocations,
not one, and neither worth avoiding.

#### Derives

| Derive | Why |
|---|---|
| `Debug` | `Result::expect` requires `E : Debug`, and the tests assert on it |
| `Clone` | Free for a `String`, and an error a caller wants to keep alongside a log line should not have to be re-created |
| `PartialEq`, `Eq` | The tests compare against a constructed value — `assert_eq!( error, NameTaken { name : "events".into() } )` — rather than matching a substring of `Display` |
| `core::error::Error` | Reports `error` via `Box< dyn Error >` (or `?`) once a caller has destructured `Err( ( error, ring ) )` |

**`PartialEq` is what keeps the tests honest.** Without it the natural assertion
is `error.to_string().contains( "events" )`, which passes for any message
mentioning the name — including a wrong one. Comparing the value asserts the
variant *and* its data.

### Validation

There is no constructor and no validation step — every part of the enum is as
public as the enum, so a caller can build `RegistryError::NameTaken` directly.
What is worth pinning is what the type does **not** validate, because each
absence was a choice:

| Not validated | Why |
|---|---|
| That `name` is non-empty | An empty name is a perfectly good key. The registry has no opinion on what a name looks like, only on whether it is taken |
| That `name` is unique | That is the *condition*, not a property of the value. An error reporting a collision that did not happen is a caller bug, not a constructible state |
| That the paired `Split< T >` is the one refused | The pairing is `register`'s tuple, not a field of this type — nothing here can check it |

**The one contract the type does carry is that `name` is the colliding name, by
value.** Not a `&str` into the registry, and not the name the caller passed: they
are equal, but the owned copy is what lets the error outlive the `register` call
and the registry itself, which is the point of taking a `String`
(→ `#### It carries the name`).

Verify the whole surface is that small:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order.
printf '  construction sites inside the crate: %s\n' \
  "$( command grep -c 'Err( ( RegistryError::' ring_registry/src/lib.rs || true )"
echo '  every mention of the variant in the source:'
command grep -n 'RegistryError::' ring_registry/src/lib.rs | cut -c1-72 | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  construction sites inside the crate: 1
  every mention of the variant in the source:
      /// [`RegistryError::NameTaken`] if the name is already live, pair
            Err( ( RegistryError::NameTaken { name }, ring ) )
```

The second command is the honest one: a bare search for the variant finds two
hits, and only one of them constructs anything. A reader who checked the count
alone would be checking the wrong number.

---

## The Bet, and Who Can Take It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order. Patterns against this
# file are anchored at column 0 and carry no line numbers: the findings below
# quote these same sentences, and would otherwise match themselves.
i=ring_registry/docs/type/001_registry_error.md
echo '  -- the bet the enum takes, and the promise the validation section makes --'
command grep -h '^A unit struct .NameTaken.\|^second failure mode, an enum absorbs\|^public as the enum, so a caller can build\|^| That .name. is unique |' "$i" | cut -c1-98 | sed 's/^/    /'
echo '  -- every public enum in the family, and how many are protected against a new variant --'
printf '    pub enum declarations in ring_*/src: %s\n' \
  "$( command grep -rc '^pub enum' --include=*.rs ring_*/src/ | awk -F: '{ s += $2 } END { print s + 0 }' )"
printf '    of those, marked non_exhaustive:     %s\n' \
  "$( command grep -rc '^#\[ non_exhaustive \]' --include=*.rs ring_*/src/ | awk -F: '{ s += $2 } END { print s + 0 }' )"
command grep -r -B 1 '^pub enum RingError\|^pub enum RegistryError' --include=*.rs ring_types/src ring_registry/src |
  sed 's|ring/||' | sed 's/^/    /'
echo '  -- and who constructs this error with no collision behind it --'
command grep 'RegistryError::NameTaken {' ring_registry/tests/registry_test.rs | cut -c1-84 | sed 's/^/    /'
```

Live output:

```
  -- the bet the enum takes, and the promise the validation section makes --
    A unit struct `NameTaken` would be smaller and would carry exactly the same
    second failure mode, an enum absorbs it without changing any existing signature,
    public as the enum, so a caller can build `RegistryError::NameTaken` directly.
    | That `name` is unique | That is the *condition*, not a property of the value. An error reporting
  -- every public enum in the family, and how many are protected against a new variant --
    pub enum declarations in ring_*/src: 23
    of those, marked non_exhaustive:     2
    ring_types/src/error.rs-#[ non_exhaustive ]
    ring_types/src/error.rs:pub enum RingError
    --
    ring_registry/src/lib.rs-#[ derive( Debug, Clone, PartialEq, Eq ) ]
    ring_registry/src/lib.rs:pub enum RegistryError
  -- and who constructs this error with no collision behind it --
      assert_eq!( error, RegistryError::NameTaken { name : "events".to_string() } );
      let error = RegistryError::NameTaken { name : "events".to_string() };
        Err( Box::new( RegistryError::NameTaken { name : "x".to_string() } ) )
```

---

### RG45 — The Bet the Enum Takes Is One Attribute Short of Paying Off

The enum-over-unit-struct choice is argued carefully and against YAGNI: a second
failure mode "absorbs it without changing any existing signature, and a unit
struct would force one." The asymmetry — one line now against a public signature
change later — is the stated justification, and for signatures it is exact.

Signatures are not what breaks. Adding a variant to a public enum breaks every
downstream exhaustive `match`, in every crate that ever matched it, which is a
larger blast radius than the signature change the bet was placed to avoid. Rust
has one answer to this and the family already uses it, twice: `ring_types::RingError`
and `ring_testkit::Anomaly` each carry `#[ non_exhaustive ]` above their own
declarations. They are **two of the family's 23 public enums**, and
`RegistryError` — declared seven lines from a `derive` list that was reasoned
through attribute by attribute — is not among them.

The loop closes with [RG17](../integration/001_one_declared_edge_of_three.md).
This crate declined `ring_types::RingError` partly on the grounds that adding a
variant "would oblige every other consumer to match an arm they can never
receive" — the exhaustive-match breakage, named precisely, as a reason to avoid a
type that is immune to it. Then it built its own enum for future variants and
left the immunity off.

**Finding.** Recorded as a **latent hazard** with an unusually cheap fix: one
attribute line, matching `ring_types::RingError`, and the bet the section argues
for becomes the bet it describes. Worth saying explicitly in the
prose too, since the reasoning is otherwise good enough to be copied: an enum
absorbs a new variant without changing signatures **and without breaking callers
only if it is `#[ non_exhaustive ]`**. Twenty-one of the family's twenty-three
public enums are unmarked as well, so this is a family-wide question rather than
this crate's alone — but this is the file that argues the case, so this is where
the qualifier belongs.

**Disposition:** declined — measured the fix's cost rather than assuming it.
Temporarily added `#[ non_exhaustive ]` to `RegistryError` and ran
`cargo check -p ring_factory` — the crate's own module doc names `ring_factory`
as "the only consumer" — with an isolated `CARGO_TARGET_DIR`. It broke:
`error[E0004]: non-exhaustive patterns: \`Err((_, _))\` not covered`, on the
`match` at `ring_factory/src/lib.rs:208`, whose `NameTaken` arm at line
214 (`Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),`)
has no wildcard. "One attribute line" is the true cost only inside this crate;
paid for real, it is that line plus a new arm in `ring_factory`, a second
crate this pass has no authorization to edit. The probe edit was reverted
(`git diff` on `src/lib.rs` afterward showed only the unrelated RG31 change
already in progress, confirming a clean revert). This does not contest the
finding — the attribute is still the right shape, and RG17 already used the
same exhaustive-match argument against a different type — only that the
one-line estimate undersells the true cost by exactly the one downstream match
this crate has. Now prints:
`error[E0004]: non-exhaustive patterns: \`Err((_, _))\` not covered`

**Correction (2026-09-20):** the paragraphs above read "It is the **only one of
the family's 22 public enums** that does", "Nineteen of the family's other
twenty public enums are unmarked as well", and "matching the family's only other
error enum". All three were wrong. The family has 23 public enums and **two**
carry the attribute — `ring_types::RingError` and `ring_testkit::Anomaly`, the
latter having acquired it after this was written — leaving 21 unmarked; and six
of the 23 are error-named (`WorkloadError`, `RunError`, `BuildError`,
`ConfigError`, `RegistryError`, `RingError`), not two, so `RingError` is the
only *marked* error enum rather than the only other one. The Regenerate block
above is the part worth noting: it already printed `pub enum declarations in
ring_*/src: 23` and `of those, marked non_exhaustive: 2`, and re-running it today
still prints exactly that. The evidence was correct and current the whole time;
nothing ever compared it to the prose below it. (The `type/`
rollup carrying the same finding failed the opposite way — three of its recipe
commands still globbed `ring_*/`, a path the crates left, so it printed
`0` and the false claim read as confirmed.) None of this touches the Disposition
above, which stands at *declined*: the finding is live, and what was wrong is the
cardinal, not the charge — `RegistryError` is unmarked, the exhaustive-match
blast radius is real, and the RG17 loop closes exactly as described, except that
the type RG17 declined is one of two immune types rather than the only one.

---

### RG46 — "A Caller Cannot Make One", Made Three Times by the Crate's Own Tests

The Validation section rests on one claim — that `RegistryError` is "built in
one place, by `register`, on one condition, and a caller cannot make one." From
it follow the two rows below: that `name` needs no uniqueness check because an
error reporting a collision that did not happen is "a caller bug, not a
constructible state", and that the type needs no validation at all.

Every part of an enum variant is as public as the enum, so
`RegistryError::NameTaken { name : .. }` is constructible by anyone who can name
the type. The crate's own test file does it three times: at `:125` to compare a
real refusal against a hand-built expectation, at `:360` to exercise `Display`
with no registry in scope at all, and at `:365` to box one for the `Error` impl
under the name `"x"`, which no registry ever refused. The last two are exactly
the state the table calls unconstructible, constructed, in this crate, on
purpose, because there is no other way to test `Display` and `Error` without
standing up a collision.

**Finding.** Recorded as a **wrong doc** whose repair improves the section rather
than shrinking it. The true statement is stronger and more useful: the type has
no invariant, by construction — public variant, public field, `Clone`,
`PartialEq`, no constructor — and that is what makes it testable in isolation and
cheap to compare. The Validation table then reads as it should: nothing is
validated because there is nothing to validate, not because nothing else can
build one. Note also that the second row's argument survives intact under the
correction — an error reporting a collision that did not happen *is* a caller
bug; it is just a reachable one, which is a reason to state the contract in
[`Display`](../api/001_the_registry_surface.md)'s terms rather than to rely on
unreachability.

**Disposition:** applied — the Validation intro (`:69-72`) now reads "every
part of the enum is as public as the enum, so a caller can build
`RegistryError::NameTaken` directly", replacing the "a caller cannot make one"
claim the crate's own tests contradict three times. Now prints: `so a caller can build`

---

### Errors

This is the error type. There is no wrapping, no source chain, and nothing it
converts from — the registry has one failure and it originates here.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_registry_surface.md](../api/001_the_registry_surface.md) | Where it is returned, and the two costs the pairing imposes on callers — a `Debug` bound, and a 448-byte `Result` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_name_one_ring.md](../invariant/001_one_name_one_ring.md) | R1 — the invariant whose enforcement produces this value |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_insert_would_have_replaced_silently.md](../pitfall/001_insert_would_have_replaced_silently.md) | The implementation that returns nothing at all, and why that is the failure |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The definition and its `Display` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/registry_test.rs` | `a_second_registration_under_a_live_name_is_refused` compares the whole value; `the_error_names_the_taken_name` covers `Display` and the `Error` impl |
