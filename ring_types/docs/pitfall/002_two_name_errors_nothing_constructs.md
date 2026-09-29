# Pitfall: Two Name Errors Nothing Constructs

### Scope

- **Purpose**: Record that [`RingError::NameTaken`](../item/enum/002_ring_error.md) and `RingError::NameUnknown` are declared for the named-ring registry, that the registry declared its own error type instead — while sitting one manifest line away from this one — and that the two variants are therefore constructed nowhere in the family.
- **Responsibility**: Name the trap, the failures, and the mitigations.
- **In Scope**: The two name variants; `ring_registry`'s `RegistryError`; why the `Copy` constraint makes the two irreconcilable.
- **Out of Scope**: The registry's own design (→ [`ring_registry`](../../../ring_registry/readme.md)); the seam this opens in the export Contract (→ [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)).

### Trap

**`error.rs` opens by claiming to be the family's one error type.** The
argument is stated at the top of the module:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A5 -F '//! One enum rather than one per crate' ring_types/src/error.rs
```

Live output:

```
//! One enum rather than one per crate: a consumer sits behind the five-crate
//! export surface (`docs/decision/121_workstream_008_contract_gaps_ruled.md` § 4)
//! and never names the 28 internal crates, so per-crate error types would have
//! to be converted into a shared one at the surface anyway. This is that shared
//! one, declared once at tier 0. Crates off the ring path — `ring_bench`,
//! `ring_factory`, `ring_flush`, `ring_registry` — declare their own.
```

**Two of its nine variants exist to serve the named-ring registry**, and the
registry does not use them:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'RingError::NameTaken\|RingError::NameUnknown' . --include=*.rs
```

Live output:

```
ring_types/tests/types_test.rs:  let naming = [ RingError::NameTaken, RingError::NameUnknown ];
ring_types/tests/types_test.rs:    RingError::NameTaken,
ring_types/tests/types_test.rs:    RingError::NameUnknown,
```

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,/^\[dev-dependencies\]/p' ring_registry/Cargo.toml
```

Live output:

```
[dependencies]
ring_handle = { path = "../ring_handle" }

[dev-dependencies]
```

**The first command finds the two variants named three times, all inside this
crate's own test suite, and never constructed by a program.** The second shows
why: **`ring_registry` declares exactly one dependency, and it is not
`ring_types`.** It is one of only three crates in the family that does not
declare tier 0 — the others are `ring_types` itself and `ring_align` (which
dropped the dependency as unused; see
[`integration/001`](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md)).

```sh
cd "$(git rev-parse --show-toplevel)"
for m in ring_*/Cargo.toml; do
  command grep -q '^ring_types' "$m" || basename "$( dirname "$m" )"
done
```

Live output:

```
ring_align
ring_registry
ring_types
```

**It is not that the registry cannot reach this crate — it is one manifest line
away.** `ring_types` is already in its build closure, pulled in transitively
through `ring_handle → ring_core`:

```sh
cd "$(git rev-parse --show-toplevel)"
cargo tree -p ring_registry --prefix none --no-dedupe | sort -u | command grep -c '^ring_types'
```

Live output:

```
1
```

Rust requires a *direct* dependency before a crate can `use` another, so the
missing manifest line is what stops `RegistryError` from being `RingError` —
and adding it is a one-line change nobody made. **The shared type was declined,
not unavailable**, which is what makes the next paragraph a design choice rather
than a workaround.

**What the registry declared instead is not a rename of the same thing.** Its
`RegistryError::NameTaken` carries the conflicting name; `RingError::NameTaken`
carries nothing:

```rust
// ring_registry/src/lib.rs:56
pub enum RegistryError
{
  NameTaken
  {
    /// The name that was already live.
    name : String,
  },
}
```

**And the difference is forced, not stylistic.** `RingError` is `Copy` and
allocation-free because an error on the tick path must not allocate — the
constraint is stated in its own doc comment and asserted by
`error_is_copy` in the suite. A `String` field would end both properties. So
`RingError::NameTaken` *cannot* carry the name, and `RegistryError::NameTaken`
would be strictly worse if it did not.

**The trap is that the tick-path constraint that justifies the shared error is
exactly what disqualifies it for the one crate whose failure is a name
collision.** Registration happens once, at setup, off the tick path entirely;
its natural error carries an owned string; the shared type is forbidden from
having one. The two requirements are both correct and cannot be met by one
enum.

**`NameUnknown` fails a second way, independently.** It describes a lookup that
did not find its ring. `ring_registry` has no such error, because
`get_mut` and `remove` both return `Option`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub fn get_mut\|pub fn remove\|pub fn contains' ring_registry/src/lib.rs
```

Live output:

```
  pub fn get_mut( &mut self, name : &str ) -> Option< &mut Split< T > >
  pub fn remove( &mut self, name : &str ) -> Option< Split< T > >
  pub fn contains( &self, name : &str ) -> bool
```

An absent name is not a failure in that design — it is a `None`. So
`NameUnknown` describes an API shape the family did not build, and would remain
unconstructed even if the registry did depend on this crate.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| N1 | A consumer matches on `RingError::NameTaken` to handle a registration conflict | Compiles, is exhaustive-checked, and the arm is dead. The real conflict arrives as `RegistryError` from a different module path |
| N2 | The two variants are read as evidence the registry routes through this crate | The dependency edge does not exist. Reasoning from the error set to the graph is inverted here |
| N3 | Someone converts `RegistryError` → `RingError` at the export surface, as `error.rs` anticipates | Loses the name — the only payload the registry error carries and the only reason it exists |
| N4 | The variants are deleted as dead | `RingError` is `#[ non_exhaustive ]` for consumers, but deletion is still breaking *within* the workspace, and it forecloses a future registry that does route through here |
| N5 | A future crate constructs `NameTaken` for a different name-like conflict | The Display string says "a ring is already registered under this name", which will be wrong for whatever that crate's names are |

**N3 is the one `error.rs`'s own argument walks into.** The module doc says
per-crate error types "would have to be converted into a shared one at the
surface anyway" — and here that conversion is lossy in the direction that
matters. The premise assumed conversion is free; for the one crate that
actually declared its own type, it costs the payload.

**N1 is the measurable one.** A dead match arm on a `#[ non_exhaustive ]` enum
produces no warning, because the wildcard the attribute forces absorbs it. The
arm is indistinguishable from a live one by any mechanism short of grepping for
constructors.

### Mitigation

**What does not work:**

| Attempt | Why it fails |
|---------|--------------|
| Add `ring_types` to `ring_registry` and use the shared variants | Loses the name. The registry's error is better than the shared one for its own case, which is why it was written |
| Put a `&'static str` in `RingError::NameTaken` | Registry names are runtime strings; a `'static` bound does not fit them. Anything else ends `Copy` |
| Delete both variants | N4 — breaking within the workspace, and it removes the record that this question was asked |
| Convert at the export surface | N3 — that is the lossy step, not the fix |

**What works:**

1. **Rule whether "one error type" is a design rule or an observation**, and
   record it. If it is a rule, the registry is in violation and the conversion
   cost is accepted. If it is an observation, `error.rs`'s opening paragraph
   overstates it and should say "one error type for the ring path" instead.
   The current state — a stated rule with a live counterexample and no ruling —
   is what produces N2 and N3.
2. **Annotate both variants as reserved rather than active.** The rustdoc on
   each currently reads as a description of a live condition. One clause —
   that no crate constructs it and `ring_registry` uses its own type — turns
   N1 from a silent dead arm into a documented one.
3. **If the variants are kept, assert their emptiness.** A test that greps
   nothing is not possible, but a test asserting `RegistryError` is what
   registration returns pins the actual contract, so a later change that starts
   constructing `RingError::NameTaken` is a visible behaviour change rather
   than a quiet convergence.

**Mitigation 1 is the one that closes N2 and N3 together**, and it is a
decision this crate cannot make alone — it is recorded as a pending question in
[`decisions/`](../decisions/readme.md) rather than settled here. Tier 0 can
declare a shared vocabulary; it cannot make a crate two tiers up use it.

**The generalisable half:** a shared type declared for consumers that do not
exist yet accumulates variants nobody validates against a caller. Seven of
`RingError`'s nine variants have constructors; the two that do not are the two
whose consumer was written last and chose otherwise. **The declaration order —
vocabulary first, consumers later — is what let the mismatch through**, and it
is the same order every tier-0 crate in this family is built in.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_classifying_an_error_into_configuration_or_traffic.md](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) | Both name variants fall in neither class — the classifier's silent third bucket |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | The exported set 30 crates import, and the two that did not |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | The `Copy` constraint that makes N3 lossy, and the 24-byte budget it buys |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_registry_that_declined_the_shared_error.md](../integration/002_the_registry_that_declined_the_shared_error.md) | The same finding as a seam — which crate depends on which, and what the export Contract sees |

### Items

| File | Relationship |
|------|--------------|
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The declaration, variant by variant, with the constructor count for each |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_discriminants_here_handlers_elsewhere.md](../pattern/001_discriminants_here_handlers_elsewhere.md) | The arrangement that works for the two policy enums and did not extend to the error set |

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_seq_next_wraps_where_its_doc_says_it_saturates.md](001_seq_next_wraps_where_its_doc_says_it_saturates.md) | The other trap — a comment with the wrong behaviour behind it, rather than a declaration with none |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | Lines 3–8, the "one error type" argument; 59–62, the two variants; 173–174, their Display strings |
| [`ring_registry/src/lib.rs`](../../../ring_registry/src/lib.rs) | Line 56, `RegistryError`; 172, the only `NameTaken` constructed anywhere in the family |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ⚠️ `every_error_displays_distinctly` covers both name variants, so they are *tested* while being unconstructed — the suite asserts their Display strings differ from the other seven and cannot observe that nothing produces them. **A test over a declaration is not a test over a behaviour**, and here the distinction is the whole finding |

### TY50 — Both Unconstructed Variants Are Fully Tested

Every test that enumerates the nine variants includes these two, so they appear
in the classification test, the distinctness test and the `Error`-trait test.
Line coverage over `error.rs` will show their `Display` arms hit.

**A coverage number cannot distinguish a variant the family raises from one only
its own suite constructs**, which is the specific way this pitfall hides: the
signal that would reveal it is a search for construction sites in *other* crates,
and no coverage tool performs one.
