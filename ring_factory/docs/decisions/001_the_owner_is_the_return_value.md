# Decision: The Owner Is The Return Value

**Status:** accepted, 2026-08-28. Rules Pendings 1, 2, 3 and 4 together, which
[`readme.md`](readme.md) recommended doing in one ADR in the order 2 → 1 → 3.
Also closes the question
[`ring_handle/docs/decisions/001`](../../../ring_handle/docs/decisions/001_what_this_crate_is_for.md)
left to it — "the question belongs to `ring_factory`, which decides what a
caller actually constructs."

### Scope

- **Purpose**: Rule Pendings 1, 2, 3 and 4 in one place, and record that three of the four were settled by code written while they sat open rather than by a choice between live alternatives.
- **Responsibility**: What forced each ruling, the ownership shape that survived, whether the registry is passed or held, and which door re-exports follow from it.
- **In Scope**: What `build` returns and who owns it; Pendings 1–4.
- **Out of Scope**: Which door a caller enters through (→ [`002_two_doors_not_one_that_routes.md`](002_two_doors_not_one_that_routes.md)); the handle pair's own shape (→ [`ring_handle/docs/data_structure/001`](../../../ring_handle/docs/data_structure/001_two_handles_over_one_backend.md)).

### What forced it

**Three of the four were ruled by code that was written while they sat open.**
That is worth stating first, because it changes what this ADR is: not a choice
between live alternatives, but a record of which alternatives stopped existing
and why the survivor is still the right one.

`ring_handle` shipped. Its shape settles Pending 2 outright:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub struct Split\|pub fn new\|pub fn ends\|pub fn split' ring_handle/src/lib.rs
```

Live output:

```
pub struct Split< T >
  pub fn ends( &mut self ) -> Ends< '_, T >
  pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
```

`Split< T >` owns a `ring_core::Ring< T >` by value. `ends( &mut self )` borrows
it, `split( &'a mut self )` separates the borrow into two. No `Arc`, no
`NonNull`, no token — **D2**, of the three candidates
[`ring_handle`'s own data structure instance](../../../ring_handle/docs/data_structure/001_two_handles_over_one_backend.md)
declined to choose between.

### The consequence nobody predicted

[`data_structure/002`](../data_structure/002_the_handle_pair_as_output.md) scored
D2's cost to this crate as **severe**, and it was right about the mechanism and
wrong about the outcome. Its reasoning: under D2 something must outlive both
handles, and the only two candidates are both bad —

| Candidate it saw | Why it is bad |
|---|---|
| The factory holds the ring | Makes the factory stateful; gives it a field [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md) forbids; puts every ring's lifetime on the factory's |
| The caller supplies storage | Changes `build` from "give me a config" to "give me a config and a place to put it", defeating the one-argument surface |

**There is a third owner and it is the return value.** `Split< T >` is an
ordinary owning value. The factory allocates the ring, wraps it, and hands
ownership to the caller — who was going to hold *something* regardless. One
argument in, one owned value out. The one-argument surface survives intact and
the factory stays fieldless.

So `build`'s signature is not the specified one:

```rust
// specified, and unwritable
pub fn build< S >( &self, cfg : RingConfig ) -> Result< HandlePair< S >, BuildError >;
// implemented
pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >;
```

**`HandlePair` cannot be written at all**, under D2 or any other shape that
keeps `ring_handle`'s two-step split. Both handles borrow from the `Ends`, which
borrows from the `Split`; a struct holding all three is self-referential.
`ring_handle`'s own doc comment says so: "Collapsing the steps would need a
self-referential struct." The pair is not a return type, it is what the caller
makes from the return value.

### Pending 1 — the registry is passed, not held

Follows directly. `ring_registry::Registry< T >` owns the `Split< T >` values
put into it, and `build` now produces exactly such a value. A `Factory` holding a
registry would therefore own every ring it ever built, which

- makes [`lifecycle/002`](../lifecycle/002_the_factory_outlives_nothing.md)'s
  title false,
- makes ["which factory built this ring"](../type/001_factory.md) a question
  with an answer, and a ring built by factory A and looked up in factory B's
  registry a new failure mode, and
- gives the factory the field [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)
  exists to forbid.

`build_named( cfg, name, registry )` takes it per call. `Factory` stays
`pub struct Factory;` — `type/001`'s fieldless definition, now ruled rather than
provisional. Its second validation row ("two `Factory` values are
interchangeable") holds unconditionally and is asserted by
`two_factories_build_identically`.

### Pendings 3 and 4 — the door re-exports

Both are the same problem: a consumer bound by the five-name export Contract
cannot name a type they need.

| Pending | The unnameable type | Ruling |
|---|---|---|
| 4 | `RingConfig`, which is `build`'s only argument | `pub use ring_config::RingConfig;` |
| 3 | `Registry`, which is where a named build's ring goes and the only way to get it back | `pub use ring_registry::Registry;` |

**Re-export, not a wrapper**, and the distinction is the whole ruling. Pending 3
framed the choice as "is `get( name )` on this crate or on `ring_registry`?" —
which presupposes that routing the named registry *through* the factory means
reimplementing its surface here. It does not. `ring_registry::get_mut` already
exists, is tested, and works; a `Factory::get` forwarding to it would be
duplication with a second place to be wrong.

This is Pending 4's option **W3** ("re-export it from here — smallest"), applied
to both. W2 (move `RingConfig` into `ring_types`) is more honest about where the
record belongs and remains the better long-term answer; it is not taken here
because it is a change to two crates that are shipped and tested, to buy a
property W3 already delivers. Recorded rather than dismissed — if `ring_config`
ever grows past a validated record, W2 is the fallback.

**What re-export does not fix:** `RingConfig::with_overflow` takes an
`OverflowPolicy`, which `ring_config` does not re-export. It lives in
`ring_types`, which *is* on the Contract, so a consumer can name it — but they
touch two crates to set one field. Not this crate's to fix and not worth a
third re-export, since adding one here would give the family two paths to the
same type.

### Verification

```bash
cd "$(git rev-parse --show-toplevel)"
cargo test -p ring_factory --all-features
```

| Claim | Test |
|---|---|
| The Contract surface is reachable without naming a non-Contract crate | `the_contract_surface_is_reachable_without_naming_a_non_contract_crate` |
| Two factories are interchangeable | `two_factories_build_identically` |
| A ring outlives the factory that built it | `a_ring_outlives_the_factory_that_built_it` |
| A named build is retrievable through the re-exported registry | `a_named_build_is_retrievable_by_that_name_and_by_no_other` |

The third is the one that would fail under a registry-holding factory, and it is
written as a scope block rather than as prose so the compiler checks it.

### Sources

| File | Relationship |
|------|--------------|
| [`ring_handle/src/lib.rs`](../../../ring_handle/src/lib.rs) | `Split`, `Ends`, and the two-step split that makes `HandlePair` unwritable |
| [`ring_registry/src/lib.rs`](../../../ring_registry/src/lib.rs) | `Registry< T >`, which owns `Split< T >` |

### FC13 — The Ruling Is Longer Than the Crate It Rules

Four pendings closed, and the code that implements the ruling is one expression:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the ruling produced --'
command grep 'Ok( Split::new( ring ) )\|^pub use ' ring_factory/src/lib.rs
echo '  -- lines, ruling versus crate --'
# the ruling proper, stopping at the findings appended below it, so this count
# does not grow every time a finding is added to the file it measures
printf '  this ADR ruling:     %s\n' \
  "$( sed -n '1,/^### FC13 /p' ring_factory/docs/decisions/001_the_owner_is_the_return_value.md | wc -l )"
printf '  src/lib.rs total:    %s\n' "$( wc -l < ring_factory/src/lib.rs )"
printf '  src/lib.rs non-doc:  %s\n' \
  "$( command grep -vcE '^\s*(///|//!)' ring_factory/src/lib.rs )"
```

Live output:

```
  -- what the ruling produced --
pub use ring_config::RingConfig;
pub use ring_registry::Registry;
    Ok( Split::new( ring ) )
    Ok( Split::new( ring ) )
  -- lines, ruling versus crate --
  this ADR ruling:     155
  src/lib.rs total:    285
  src/lib.rs non-doc:  76
```

That ratio is not a complaint. The decision spans four questions that were open
across two crates, and the artefact it produced is two `pub use` lines and a
return expression precisely *because* the questions were settled before anything
was written — the cheapness of the code is the evidence the ruling worked.

What it does establish is where this crate's substance actually lives. Seventy-six
non-doc lines carry three methods and an error type; the reasoning that fixed
their shape is in `docs/`, and the source's own doc comments carry a summary of
it rather than the argument. A reader who changes the return type will not find
the four pendings from the code, and nothing in `src/lib.rs` points here.

### FC14 — The Decision Was Forced Before It Was Taken

The ADR reads as a choice between an owner and a pair. It was not available as a
choice by the time it was written — `ring_handle`'s two borrow signatures make
the alternative unwritable:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub fn ends\|pub fn split' ring_handle/src/lib.rs
echo '  -- and what build returns instead --'
command grep 'pub fn build< S : Send >' ring_factory/src/lib.rs
```

Live output:

```
  pub fn ends( &mut self ) -> Ends< '_, T >
  pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
  -- and what build returns instead --
  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
```

`ends` borrows `&mut self` and `split` borrows from that borrow, so a struct
holding the ring and both handles refers to itself and cannot be constructed in
safe Rust. The ADR's own status line dates it after `ring_handle` was
implemented.

The distinction matters for how much this ADR is worth trusting on the questions
it *did* decide. The return type was determined by a dependency; the re-export
rule (→ [`api/001`](../api/001_the_build_surface.md) FC6) and the
registry-as-argument shape were genuinely open and genuinely ruled. Presenting
all four as one decision makes the forced one lend its inevitability to the
other three, and a future reader reopening any of them will find a single
accepted ADR rather than one closed question and three live ones.
