# type

The crate declares two types and reasons about both at length. `RegistryError` is
an enum with one variant, chosen over a unit struct so a second failure mode can
arrive without a signature change, with four derives justified one at a time.
`Registry< T >` derives only `Debug` and carries a hand-written `Default` that a
reviewer would take for boilerplate.

The four findings here are all of one kind: a property the type actually has, or
actually lacks, that no line of the crate records. The enum's bet needs one
attribute it does not have. The claim that a caller cannot construct the error is
disproved three times by the crate's own tests. The `Default` is the only correct
way to write it and says so nowhere. And `Send`/`Sync` hold by inference in a
family that pins them with static assertions in four crates.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_registry_error.md) | `RegistryError` | The enum-over-struct bet, the four derives, and the constructibility claim |
| [002](002_what_registry_derives_and_what_it_never_asserts.md) | What `Registry` Derives and What It Never Asserts | The `Default` that cannot be derived, and the two traits nothing pins |

## A Bet Placed Against Signatures, Lost on Matches

Adding a variant to a public enum leaves every signature untouched — which is
what the section argues — and breaks every downstream exhaustive `match`. Rust
has one answer and the family already uses it, twice: `ring_types::RingError` and
`ring_testkit::Anomaly` carry `#[ non_exhaustive ]`, two of the family's 23
public enums.

`RegistryError` is among the other 21, seven lines from a derive list reasoned
through attribute by attribute. The loop closes with `integration/001`, which
declined `RingError` partly because a new variant there "would oblige every other
consumer to match an arm they can never receive" — the exhaustive-match breakage,
named precisely, as a reason to avoid one of the two types in the family immune
to it.

**Correction (2026-09-20):** this section read "alone among the family's 22
public enums" and closed on "the one type in the family immune to it". Both
halves were stale — the family has 23 public enums, not 22, and two of them carry
the attribute, `ring_testkit::Anomaly` having acquired it after this was written.
The Regenerate block below is why neither half was ever checked, and it failed
the same way the prose did: three of its commands still globbed `ring_*/`,
a path the crates left, so the two producing these counts printed `0` and the
false claim read as confirmed. All three are retargeted to `ring/` below, the
marked enums are now listed by name rather than counted, and a Live output block
is added so the next drift is a diff rather than a reading. What survives is the
finding — `RegistryError` is unmarked, and nothing records that the family's
evolution policy splits two ways — not the exclusivity, which was never the
charge.

## Two Properties Held by Inference

`impl< T > Default for Registry< T >` written out is not boilerplate:
`#[ derive( Default ) ]` on a generic struct adds `T : Default` whether the fields
need it or not, and the field is a `HashMap`, which needs nothing from `T`. The
derived form would reject `Registry< NoDefault >::default()` and keep compiling
for `u32`, which is what every test and every doctest here uses. It would have
shipped. Nothing in the crate says so.

`Send` and `Sync` hold the same way. Four crates pin them with static assertions
in six files, `ring_handle` — this crate's only dependency, and the owner of the
type in the field — among them. `ring_registry` names either trait zero times in
its source and readme together, so an `Rc`-based interner or a `RefCell` of
statistics could remove the property without failing a test, every test being
single-threaded.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the attribute the bet needs, and the enums in the family that have it --'
printf '    pub enum in ring_*/src: %s   marked non_exhaustive: %s\n' \
  "$( command grep -rc '^pub enum' --include=*.rs ring_*/src/ | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -rc '^#\[ non_exhaustive \]' --include=*.rs ring_*/src/ | awk -F: '{ s += $2 } END { print s + 0 }' )"
command grep -r -A2 '^#\[ non_exhaustive \]' --include=*.rs ring_*/src/ |
  command grep '^ring/.*pub enum' | sed 's/^/    /'
echo '  -- the state the Validation table calls unconstructible, constructed --'
command grep 'RegistryError::NameTaken {' ring_registry/tests/registry_test.rs |
  cut -c1-80 | sed 's/^/    /'
echo '  -- the impl that looks derivable and is not --'
command grep -m1 -A6 -F 'impl< T > Default for Registry< T >' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- who pins Send and Sync, and what this crate says about either --'
command grep -rl 'fn assert_send\|fn assert_sync' --include=*.rs ring_*/ |
  sed 's|ring/||' | sort | sed 's/^/    /'
printf '    Send or Sync named in ring_registry src and readme: %s\n' \
  "$( command grep -c 'Send\|Sync' ring_registry/src/lib.rs ring_registry/readme.md |
       awk -F: '{ s += $2 } END { print s + 0 }' )"
```

Live output:

```
  -- the attribute the bet needs, and the enums in the family that have it --
    pub enum in ring_*/src: 23   marked non_exhaustive: 2
    ring_testkit/src/lib.rs-pub enum Anomaly
    ring_types/src/error.rs-pub enum RingError
  -- the state the Validation table calls unconstructible, constructed --
      assert_eq!( error, RegistryError::NameTaken { name : "events".to_string() } );
      let error = RegistryError::NameTaken { name : "events".to_string() };
        Err( Box::new( RegistryError::NameTaken { name : "x".to_string() } ) )
  -- the impl that looks derivable and is not --
    impl< T > Default for Registry< T >
    {
      fn default() -> Self
      {
        Self::new()
      }
    }
  -- who pins Send and Sync, and what this crate says about either --
    ring_flush/tests/flush_test.rs
    ring_handle/tests/handle_test.rs
    ring_mpsc/src/lib.rs
    ring_mpsc/tests/mpsc_test.rs
    ring_spsc/src/lib.rs
    ring_spsc/tests/spsc_test.rs
    Send or Sync named in ring_registry src and readme: 0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG45 | `ring_registry` | **latent hazard** | The enum-over-unit-struct choice is argued carefully and against YAGNI — a second failure mode "absorbs it without changing any existing signature, and a unit struct would force one" — and for signatures the asymmetry is exact, while signatures are not what breaks: adding a variant to a public enum breaks every downstream exhaustive `match` in every crate that ever matched it, a larger blast radius than the signature change the bet was placed to avoid, and Rust has one answer the family already uses, `ring_types::RingError` carrying `#[ non_exhaustive ]` as **one of only two of the family's 23 public enums** that do — `ring_testkit::Anomaly` is the other — while `RegistryError`, declared seven lines from a derive list reasoned through attribute by attribute, does not; the loop closes with RG17, since this crate declined `ring_types::RingError` partly because adding a variant there "would oblige every other consumer to match an arm they can never receive" — the exhaustive-match breakage, named precisely, as a reason to avoid a type immune to it — then built its own enum for future variants and left the immunity off; the fix is one attribute line and the prose qualifier is worth writing too, since an enum absorbs a new variant without breaking callers **only if it is `#[ non_exhaustive ]`**, and twenty-one of the twenty-three public enums are unmarked as well, making this a family-wide question this file happens to be the one arguing |
| RG46 | `ring_registry` | **wrong doc** | The Validation section rests on one claim — that `RegistryError` is "built in one place, by `register`, on one condition, and a caller cannot make one" — from which its two rows follow, that `name` needs no uniqueness check because an error reporting a collision that did not happen is "a caller bug, not a constructible state", and that the type needs no validation at all; every part of an enum variant is as public as the enum, so `RegistryError::NameTaken { name : .. }` is constructible by anyone who can name the type, and **the crate's own test file does it three times** — once inside an `assert_eq!` to compare a real refusal against a hand-built expectation, once as a bare `let error = ..` to exercise `Display` with no registry in scope at all, and once as `Err( Box::new( .. ) )` for the `Error` impl under the name `"x"`, which no registry ever refused — the last two being exactly the state the table calls unconstructible, constructed, in this crate, on purpose, because there is no other way to test `Display` and `Error` without standing up a collision; the repair improves the section rather than shrinking it, the true statement being stronger — the type has no invariant, by construction: public variant, public field, `Clone`, `PartialEq`, no constructor — which is what makes it testable in isolation and cheap to compare, and the second row's argument survives intact, a mismatched error being a caller bug that is simply reachable |
| RG47 | `ring_registry` | n/a — doc gap | `impl< T > Default for Registry< T > { fn default() -> Self { Self::new() } }` is seven lines a reviewer skims and the only correct way to write it, since `#[ derive( Default ) ]` on a generic struct adds `T : Default` to the generated impl regardless of whether the fields need it — the field here is a `HashMap`, whose own `Default` needs nothing from `T` — so the derived form rejects `Registry< NoDefault >::default()` with `error[E0277]` naming the cause outright, "type parameter would need to implement `Default`", while the hand impl accepts every `T`; the distinction matters more here than it usually would, a registry's record type being chosen by the consumer and the family's record types being ring payloads — `ring_types`' identifiers, a consumer's event struct — none with any reason to be `Default`, so a derived `Default` would have made `Registry::default()` unusable for most real `T` while continuing to compile for `u32`, which is what every test and every doctest in the crate uses, and it would have shipped; nothing records this, the crate that spends twenty lines justifying a lint suppression giving its one non-obvious impl no comment at all, so a later maintainer tidying "redundant" boilerplate into a derive would break consumers without breaking a single test here — one comment naming the spurious bound makes it verifiable in place, and a `Registry< NonDefaultRecord >::default()` line in the tests is the version a compiler enforces |
| RG48 | `ring_registry` | n/a — unenforced | `Registry< u32 >` is both `Send` and `Sync`, as is the `Split< u32 >` it holds, and none of that is accidental — a registry is built on one thread and its rings are worked by others, `ring_handle` stating the requirement in its own signatures as `impl< T : Send > Split< T >` — while **four crates pin these properties with static assertions across six files**, `ring_mpsc` and `ring_spsc` in both `src` and `tests` and `ring_flush` and `ring_handle` in tests, so the practice is not merely nearby but one edge away, on the type in the field, `ring_handle` being this crate's only dependency and the owner of the type `Registry` exists to hold; `ring_registry` has zero assertions and names `Send` or `Sync` **zero** times in source and readme together, the properties holding today by inference through `HashMap< String, Split< T > >`, which means they hold until any field or future auxiliary type stops being `Send` — an interior-mutability cache, an `Rc`-based name interner, a `RefCell` of statistics — none of which would fail a single existing test, every test building a registry and using it on the thread that built it; two lines in `tests/registry_test.rs` matching what `ring_handle` already does turn the inference into a compile-time guarantee, and the interaction with RG12 is worth noting since adding `T : Send` to the inherent impl would also bound this type's `Default`, `default()` calling `Self::new()` — the two changes are not independent, and doing the assertion first makes the bound question decidable on evidence rather than taste |
