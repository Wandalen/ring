# Data Structure: The Handle Pair as Output

### Scope

- **Purpose**: Describe what `build` hands back — the value that owns the ring, from which the caller makes the pair — and record how the three candidate shapes resolved and why the cost this instance predicted did not arrive.
- **Responsibility**: State the abstract, the structure, and the operations available on it.
- **In Scope**: The returned owner; the pair the caller derives from it; who owns the ring afterwards; the D1/D2/D3 choice as this crate experienced it.
- **Out of Scope**: What the handles do (→ [`ring_handle`](../../../ring_handle/readme.md)); registration (→ [`api/002`](../api/002_the_named_build_surface.md)).

### Abstract

**`build` returns one value, and the pair is what the caller makes from it.**
The value is `ring_handle::Split< S >`, which owns the ring. `Split::ends`
borrows it and `Ends::split` divides that borrow into a `Producer` and a
`Consumer`, each holding the only route it will ever have to the backend.
The producer/consumer split makes the split the point: who may consume is "a question answered
by ownership rather than by convention," and the ownership in question begins
here, because this is where the value the split comes out of is created.

**This instance's title says "the handle pair as output", and the pair is not
the output.** The title is kept rather than corrected because the gap between
it and the signature is the finding — see *Structure* below. What is returned
is the pair's *owner*.

**`HandlePair< S >` was never writable.** Both handles borrow from the `Ends`,
which borrows from the `Split`; a struct holding all three is self-referential.
So the question this instance posed — named type or tuple — had no live
branches: neither form can be returned, and the tuple exists only *after* the
caller has bound the owner to a local.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' ring_handle/src/lib.rs
```

Live output:

```
20
```

**`ring_handle` is no longer a skeleton**, and the count above is no longer
zero — re-run it rather than trusting a number written here, since it is the
kind of fact that goes stale silently. This instance was written against a type
that did not exist; it now describes one that does, and the differences are
recorded in place below rather than deleted.

### Structure

**One field per handle, and its type was the open question.**
[`ring_handle`'s own data structure instance](../../../ring_handle/docs/data_structure/001_two_handles_over_one_backend.md)
set out three candidate shapes for that field and explicitly declined to
choose. **D2 won**, decided by that crate being written rather than by anyone
ruling — `Split< T >` owns a `ring_core::Ring< T >` by value and both handles
borrow from it. The table below is kept as it was scored, because what it got
wrong is more useful than what it got right.

| # | Shape in `ring_handle` | What `build`'s signature becomes | Cost to this crate |
|---|------------------------|----------------------------------|--------------------|
| D1 | `Arc< Backend >` | `fn build< S >( &self, cfg ) -> ( Producer< S >, Consumer< S > )` | **None.** The ring is heap-allocated here and both handles share it; the factory returns and forgets |
| **D2 ✅ chosen** | `&'a Backend` | `fn build< S : Send >( &self, cfg ) -> Result< Split< S >, BuildError >` | **Predicted severe; actual none.** Something must outlive both handles and it is neither the factory nor caller-supplied storage — it is the returned value (→ [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md)) |
| D3 | `NonNull< Backend >` + token | Same as D1 outwardly | The `unsafe` obligation — "the ring outlives both handles" — is asserted **here**, by the code that allocated it |

**The prediction was right about the mechanism and wrong about the outcome, and
the error has a shape worth naming.** The reasoning was: under D2 the backend
must live somewhere else, and both candidates are bad — the factory holds it
(stateful, a field, every ring's lifetime on the factory's) or the caller
supplies storage (a second argument, defeating the one-argument surface). Both
of those remain true. **The list of candidates was incomplete.** The third owner
is the return value: `Split< T >` is an ordinary owning value, the factory hands
it over, and the caller was going to hold *something* regardless. One argument
in, one owned value out; the factory stays fieldless and
[`lifecycle/002`](../lifecycle/002_the_factory_outlives_nothing.md) stays true.

**The failure mode was enumerating candidates, not evaluating them.** Each of
the two candidates was scored correctly; the cost estimate was wrong because the
set was short by one. A table with a *Cost* column invites checking whether each
row's score is right and never asks whether a row is missing — which is the
weakness of scoring a closed set drawn from an open one. The check that would
have caught it is cheap and is not a scoring check at all: *name who owns the
value at every point in the call, including after the return*. Recorded in
[`decisions/001`](../decisions/001_the_owner_is_the_return_value.md).

**This crate had a stake in a decision recorded in another crate's
`decisions/`,** and that stake was real: whoever ruled D1/D2/D3 was also ruling
on whether `Factory` is fieldless, and neither document said so. It was settled
by `ring_handle` being *implemented* rather than by either document being
amended — which is the same failure of process arriving as a happy accident.

**The ring itself is not in the return value.** There is no `Ring` handed back
alongside the pair, and there must not be — a third route to the backend would
make the two-handle partition advisory. The backend is reachable only through
whichever field shape wins, and only from the two values returned.

**Nothing about the config survives into the pair.** Neither handle carries a
`RingConfig`, a capacity, or a backend discriminant that a caller can read.
This is a deliberate absence in `ring_handle` — counters and flags are absent
for the same reason — and it is what makes
[`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)'s
mitigation 2 a *change*: exposing the effective config on the pair would be new
state on a type built around having none.

### Operations

**This crate performs exactly one operation on the pair: it produces it and
gives it away.**

| # | Operation | Performed by | Note |
|---|-----------|--------------|------|
| H1 | ~~Construct both halves from one backend~~ **Wrap the backend in its owner** | This crate | ✏️ **Revised.** The split does *not* happen here — `Split::new( ring )` is the whole operation. The caller splits, and may do so repeatedly: each `ends()` yields one pair and ends when the borrow ends |
| H2 | Return the owner | This crate | Ownership transfers wholly to the caller |
| H3 | Move the owner into the registry | This crate, on the naming path only | ✅ **Settled and duplication is not needed.** The registry takes the `Split< T >` by value and lends `&mut Split< T >` back from `get_mut` |
| H4 | Publish / drain | The caller, via the handles | Not this crate's |

**H3 was called "the operation that has not been thought through", and it
resolved without needing to be.** The worry was that registering means the
registry retains something, and that under D2 it would hold a borrow and acquire
a lifetime parameter. It holds the *owner* instead — `Registry< T >` stores
`Split< T >` by value, has no lifetime parameter, and hands back `&mut Split< T >`.
The duplication H3's old note demanded is not required because nothing is
duplicated: the value moves in, and lookups borrow from where it now lives.

**The prior question it raised — what does the registry hold, the pair or a
second pair? — is answered "neither".** A registry handing out `Producer` values
would issue a second producer for a ring that already has one, breaking exactly
the partition the producer/consumer split exists to create; a registry answering only "does this
name exist" would be safe and nearly useless. Lending a `&mut` to the owner is
the third option, and it preserves the partition mechanically: one borrow yields
one `Ends`, one `Ends` yields one producer and one consumer, and the borrow
checker refuses a second concurrent borrow. **The same shape of miss as the D2
cost estimate — two options enumerated where three exist**, and the third is
again "let the ordinary ownership rules do it".

**H1 is not once-only, and that correction matters.** The old text said the
factory splits and then has no further relationship with either half. The
factory has no further relationship with anything — but the *split* is
repeatable, because it is the caller's operation on a value they own, bounded by
a borrow rather than consumed. Nothing about
[`lifecycle/002`](../lifecycle/002_the_factory_outlives_nothing.md) depends on
the split being once-only; it depends on the factory keeping no reference, which
a fieldless unit struct guarantees structurally.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | The signature D1/D2/D3 would change |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | H3's operation, and the registry-holds-what question |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | Step 6, which produces this |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_the_configuration_record_as_input.md](001_the_configuration_record_as_input.md) | The other side of the same call |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_reached_closure.md](../integration/001_declared_edges_and_the_reached_closure.md) | `ring_handle` reachable only through `ring_registry` — the return type's odd position in the graph |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_factory_outlives_nothing.md](../lifecycle/002_the_factory_outlives_nothing.md) | H1's consequence, and the one shape that breaks it |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | Mitigation 2 — why exposing the effective config here is a change, not an addition |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_factory.md](../type/001_factory.md) | N2's field question — settled fieldless, but by the third owner rather than by D2 directly |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_handle/docs/data_structure/001_two_handles_over_one_backend.md`](../../../ring_handle/docs/data_structure/001_two_handles_over_one_backend.md) | D1, D2 and D3 as originally stated, with their costs to that crate |
| [`ring_handle/src/lib.rs`](../../../ring_handle/src/lib.rs) | `Split`, `Ends`, and the two-step split — the implementation that ruled D2 |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ✅ **Unblocked and written.** The prediction that the tests could not exist until D1/D2/D3 was ruled held exactly — and the ruling arrived from `ring_handle` being implemented, not from a decision being taken. The hardest blocking dependency was on a decision; it was discharged by code |

### FC11 — The Owner Is Exactly the Ring, and the Error Rides in a Niche

`Split< S >` is what `build` returns and the caller derives the pair from. It
adds nothing to what it owns, and neither does the `Result` around it:

```sh
cd "$(git rev-parse --show-toplevel)"
mkdir -p ./-fc_split_probe/src
printf '%s\n' '[workspace]' '[package]' 'name = "fc_split_probe"' 'version = "0.0.0"' \
  'edition = "2021"' 'publish = false' '' '[dependencies]' \
  'ring_factory = { path = "../ring_factory" }' \
  'ring_handle = { path = "../ring_handle" }' \
  'ring_core = { path = "../ring_core" }' > ./-fc_split_probe/Cargo.toml
cat > ./-fc_split_probe/src/main.rs <<'PROBE'
use ring_core::Ring;
use ring_factory::BuildError;
use ring_handle::Split;
fn main()
{
  println!( "Ring< u32 >                        = {}", core::mem::size_of::< Ring< u32 > >() );
  println!( "Split< u32 >                       = {}", core::mem::size_of::< Split< u32 > >() );
  println!( "Result< Split< u32 >, BuildError > = {}", core::mem::size_of::< Result< Split< u32 >, BuildError > >() );
}
PROBE
cargo run --quiet --manifest-path ./-fc_split_probe/Cargo.toml 2>&1 | tail -3
rm -rf -- ./-fc_split_probe
```

Live output:

```
Ring< u32 >                        = 320
Split< u32 >                       = 320
Result< Split< u32 >, BuildError > = 320
```

Three identical numbers. `Split` is a newtype whose entire content is the ring,
so the "owner" this instance argues for costs nothing over the thing it owns —
and the 24-byte `BuildError` fits in a niche the ring already had, so `build`'s
fallibility is free at the ABI as well as in the argument
(→ [`item/002`](../item/002_four_nouns_two_of_them_somebody_elses.md) FC28,
which measures the same freeness from the error's side).

That is the strongest available answer to D3 — "does wrapping the ring in an
owner cost anything" — and this instance predicted a cost that did not arrive.
Worth stating in the width rather than in the reasoning: the shape was chosen
because a self-referential `HandlePair` cannot be written, and it turns out to
also be the shape with no overhead to justify.

**Disposition:** declined — this instance's own text states the measured
cost is zero ("this instance predicted a cost that did not arrive... no
overhead to justify") and draws no design action from it beyond crediting the
shape already chosen; `Split<S>` in `ring_handle/src/lib.rs` is
already the minimal newtype this finding measures as zero-overhead, so no
source or doc change in `ring_factory/src/lib.rs` is implied by this
measurement itself.

### FC12 — A Name Collision Constructs and Drops 320 Bytes to Return 24

`build_named` builds first and registers second. Both orders are defensible and
the chosen one means the whole ring exists before the name is checked:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the order --'
command grep 'let split = self.build( cfg )?;\|match registry.register' ring_factory/src/lib.rs
echo '  -- and what the refusal arm drops --'
command grep '_refused' ring_factory/src/lib.rs
```

Live output:

```
  -- the order --
    let split = self.build( cfg )?;
    match registry.register( name, split )
  -- and what the refusal arm drops --
      Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),
```

So the cost of a refused name is a 320-byte allocation-bearing construction, a
move into `register`, a move back out through the error tuple, and a drop —
before a 24-byte `BuildError::NameTaken` reaches the caller.

Reversing the order would make the refusal nearly free, and it is not available:
`Registry::register` takes the ring by value and is the only operation that can
decide the name atomically against its own map, so a `contains`-then-`register`
prelude would be a check that another thread can invalidate between the two
calls. The current order is a correctness choice paying a measured price, and
the price is not stated anywhere in the source — the doc comment describes the
guarantee the order buys ("a refused registration leaks nothing") without
mentioning what it costs to buy it.

This is the number [`item/001`](../item/001_three_verbs_one_of_them_conditional.md)
FC26 cites when it calls `NameTaken` the expensive-looking-cheap variant.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A6 -F 'That guarantee is not free' ring_factory/src/lib.rs
```

Live output:

```
    /// exposes nothing. **That guarantee is not free: a full `Split<S>` is
    /// constructed, moved into the registry, moved back out, and dropped before
    /// the much smaller [`BuildError`] reaches the caller** — building first is a
    /// correctness choice, since registration is the only operation that can
    /// decide the name atomically, and a check-then-build order could be
    /// invalidated by another thread between the two calls.
    ///
```

**Disposition:** applied — `build_named`'s doc comment in
`ring_factory/src/lib.rs` now states the price this instance measures:
a full `Split<S>` is constructed, moved into the registry, moved back out,
and dropped before the much smaller `BuildError` reaches the caller, and
names why the order is chosen anyway — registration is the only operation
that can decide the name atomically, so a check-then-build order could be
invalidated by another thread between the two calls. Now prints: `That
guarantee is not free`
