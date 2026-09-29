# Decision: "One Error Type" Is a Rule the Family Does Not Keep

### Scope

- **Purpose**: Record what `src/error.rs`'s opening claim is worth once it is measured against the workspace rather than against this crate, and what the three available readings each cost the crates downstream.
- **Responsibility**: State the claim, the measurement that contradicts it, the three readings, and which one the family's own code already implements.
- **In Scope**: The sentence at `src/error.rs:1`, the six error enums the workspace declares, and the absent conversions between them.
- **Out of Scope**: The `Copy` requirement that forces the split — see [`non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md); the two unconstructed name variants downstream of it — see [`002`](002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md).

### The Claim

```rust
// ring_types/src/error.rs:1-7
//! The one error type the whole ring family returns.
//!
//! One enum rather than one per crate: a consumer sits behind the five-crate
//! export surface … and never names the 28 internal crates, so per-crate error
//! types would have to be converted into a shared one at the surface anyway.
//! This is that shared one, declared once at tier 0.
```

The argument is sound and the premise is false. Per-crate error types **were not**
converted into a shared one at the surface. They were declared alongside it.

### The Measurement

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rhoE '^pub enum [A-Za-z]*Error' ring_*/src/*.rs | sort
```

Live output:

```
pub enum BuildError
pub enum ConfigError
pub enum RegistryError
pub enum RingError
pub enum RunError
pub enum WorkloadError
```

Six, not one. Five of them are outside this crate, and this crate's own
documentation of its `decisions/` index records **one** of the five
(→ [TY2](#ty2--the-p2-entry-names-one-counterexample-of-five)).

| Enum | Crate | Public? | Wraps `RingError`? |
|------|-------|---------|--------------------|
| `RingError` | `ring_types` | yes | — |
| `WorkloadError` | `ring_bench` | yes | no |
| `RunError` | `ring_bench` | yes | **yes** — `Ring( RingError )` |
| `BuildError` | `ring_factory` | yes | **yes** — `Unsupported( RingError )` |
| `ConfigError` | `ring_flush` | yes | no |
| `RegistryError` | `ring_registry` | yes | no |

### The Three Readings

| Reading | What it obliges | What it costs |
|---------|-----------------|---------------|
| A rule, and five crates violate it | Fold all five in | `RegistryError` needs a `String`; folding it in puts an allocation behind the tick path or deletes the colliding name from the message |
| A rule scoped to the ring path | One clause added to the sentence, and the five become documented exceptions | Nothing — but somebody must say which crates are "the ring path" |
| An observation that expired | Correct the sentence to describe what is true | Nothing — and the design argument in the following paragraph survives intact, because it was always an argument about *the ring path* |

**The second and third are the same edit** made for different reasons, and the
family's code already implements the second: `ring_bench` and `ring_factory` both
wrap `RingError` rather than replacing it, which is precisely the behaviour a
ring-path-scoped rule would predict.

### What the Absent Conversions Cost

The sentence's own argument is that per-crate errors "would have to be converted
into a shared one at the surface anyway". Nothing converts:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'impl From' ring_bench/src/lib.rs ring_factory/src/lib.rs \
  ring_flush/src/lib.rs ring_registry/src/lib.rs ring_types/src/*.rs | wc -l
```

Live output:

```
0
```

Zero `From` impls across all six enums. `ring_bench::RunError` therefore unions
three of them by hand — `Build( BuildError )`, `Ring( RingError )`,
`Flush( ConfigError )` — which is the conversion the module doc predicted,
written as a variant list instead of a trait impl, in the one crate that happens
to need all three.

### TY1 — The Opening Sentence Is False as Written

`src/error.rs:1` states that this is the one error type the whole ring family
returns. The workspace declares six, five of them in other crates, and two of
those five wrap this one rather than replacing it. The sentence's *argument*
holds for the ring path; its *scope* does not hold for the family.

The module doc now takes the third reading — scoped to the ring path, and
naming the four crates that declare their own:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F '//! The one error type the ring path returns.' ring_types/src/error.rs
```

Live output:

```
//! The one error type the ring path returns.
//!
//! One enum rather than one per crate: a consumer sits behind the five-crate
//! export surface (`docs/decision/121_workstream_008_contract_gaps_ruled.md` § 4)
//! and never names the 28 internal crates, so per-crate error types would have
//! to be converted into a shared one at the surface anyway. This is that shared
//! one, declared once at tier 0. Crates off the ring path — `ring_bench`,
//! `ring_factory`, `ring_flush`, `ring_registry` — declare their own.
```

**Disposition:** applied — `src/error.rs`'s opening module doc now states the
third reading this instance's own Status section recommends, naming the four
crates outside the ring path that declare their own error types. Now prints:
`The one error type the ring path returns.`

### TY2 — The P2 Entry Names One Counterexample of Five

[`../decisions/readme.md`](readme.md)'s pending question P2 records the same
contradiction and names `ring_registry` as the counterexample. Four more exist —
`ring_bench` twice, `ring_factory`, `ring_flush` — and none is named anywhere in
this crate's corpus. A reader who acts on P2 will fold in one enum and believe
the question closed.

### TY3 — No Error Enum in the Family Converts Into Another

Not one of the six carries an `impl From` for any other. The composition the
module doc anticipated exists only as `RunError`'s three hand-written wrapping
variants, in the single crate that consumes all three producers.

### TY4 — `ring_factory` Hands Out the Shared Error Only Nested

`ring_factory` is on the five-crate export Contract and returns
`BuildError::Unsupported( RingError )`, never a bare `RingError`. A
Contract-bound consumer therefore receives the family's shared error type from
the ring-building crate only by destructuring another crate's enum, and cannot
name `RegistryError` at all (→ P4).

### TY5 — `RingError` Is Marked `#[ non_exhaustive ]` and Its Two Neighbours Are Not

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'public enums:            '; grep -rhE '^pub enum' ring_*/src/*.rs | wc -l
printf 'non_exhaustive attrs:    '; grep -r '^#\[ non_exhaustive \]' ring_*/src/*.rs | wc -l
grep -r '^#\[ non_exhaustive \]' ring_*/src/*.rs
```

Live output:

```
public enums:            23
non_exhaustive attrs:    2
ring_testkit/src/lib.rs:#[ non_exhaustive ]
ring_types/src/error.rs:#[ non_exhaustive ]
```

**Twenty-three public enums in the family, and two `#[ non_exhaustive ]` attributes
between them** — this one and `ring_testkit::Anomaly`. `WaitKind` and
`OverflowPolicy` are declared fifty lines away in this same crate, sit on the
same export Contract, and are not marked — so adding a variant to the error is a
non-event for consumers, and adding one to either policy enum is a compile error
in every crate that matches it. Two evolution policies, one crate, nothing
documenting the split.

The attribute currently costs nothing and buys nothing: **no crate in the family
matches on a `RingError` value at all.** The four production lines mentioning
`RingError::` in dependent sources are all *constructions* appearing in match
arms of some other match — `ring_overflow:204` on an `OverflowPolicy`,
`ring_shutdown:385-386` on its own `Refusal`, `ring_shutdown:629` on a `Result`.
Consumers destructure the error nowhere, so the exhaustiveness the attribute
forbids is a thing none of them has yet tried to do.

**Correction (2026-09-20):** the heading above read "`RingError` Is the Family's
Only `#[ non_exhaustive ]` Enum" and the paragraph under the measurement read
"one `#[ non_exhaustive ]` attribute between them". Both were false, and this
file contained its own disproof: the Live output four lines above the sentence
prints `non_exhaustive attrs: 2` and names both files —
`ring_testkit/src/lib.rs` and `ring_types/src/error.rs`. `ring_testkit::Anomaly`
acquired the attribute after the prose was written, the recipe kept pace because
it is a recipe, and nothing ever read the two against each other. The heading now
names the charge instead of a count, so the next drift in the family's marked-enum
roster cannot falsify it. What survives is the whole of the finding — `RingError`
is marked, `WaitKind` and `OverflowPolicy` are not, the split is undocumented, and
the attribute still costs nothing and buys nothing because no crate matches on a
`RingError` value. Exclusivity was never the charge.

### Status

**Open.** This instance replaces P2's prose entry with the measurement; the
question it asks — which of the three readings the family means — is unchanged
and still unruled. The recommended reading is the third, on the grounds that it
is the only one requiring no crate outside `ring_types` to change.
