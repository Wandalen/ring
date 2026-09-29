# Type: Factory

### Scope

- **Purpose**: Define the crate's namesake type, establish that it holds no state, and give the reason it is a type at all rather than two free functions.
- **Responsibility**: State the definition and the validation rules.
- **In Scope**: Fields (there are none); what the type carries in the registering variant; why it exists as a nominal type.
- **Out of Scope**: The operations on it (→ [`api/001`](../api/001_the_build_surface.md)); its lifetime relative to what it builds (→ [`lifecycle/002`](../lifecycle/002_the_factory_outlives_nothing.md)).

### Definition

**A `Factory` has no fields.**

```rust
pub struct Factory;
```

Every input to a build arrives in the `RingConfig` argument, by
[`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md).
A field on the factory would be a second input, and the invariant forbids one.
The empty struct is not minimalism — it is the invariant made structural.

**Which means the honest question is why it is a type at all**, since
`fn build( cfg ) -> …` as a free function would do the same work. Three
reasons, in descending strength:

| # | Reason | Weight |
|---|--------|--------|
| N1 | **It is one of five names on the export Contract.** A consumer writes `ring_factory::Factory` and has a noun to hold, import, and refer to in their own documentation. A free function gives them a verb and no vocabulary | Decisive |
| N2 | ~~**The registering variant genuinely needs state.**~~ `build_named` consults a registry — **which is passed, not held** (→ [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md)). The reason evaporated; N1 carries the type alone | ❌ **Withdrawn.** Scored *Strong* and it was worth nothing |
| N3 | Symmetry with `RingConfig` — a record in, a service out | Weak. Aesthetics, recorded so nobody mistakes it for an argument |

**N2 was called "the one that decides the shape", and it decided nothing.** Two
variants were live; the stateless one won:

| Variant | Shape | Consequence |
|---------|-------|-------------|
| ✅ **Stateless** | `struct Factory;` and the registry passed per call | The type stays fieldless and `build`/`build_named` are near-identical; the registry's lifetime is the caller's problem |
| ❌ Registry-holding | `struct Factory { registry : Registry }` | The factory becomes stateful, [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md) acquires a second input it must explicitly permit, and two factories are no longer interchangeable |

**What actually ruled it was not the argument recorded here.** This file made
the case on `invariant/001` grounds — a field is a second input — and left the
matter open because the registry-holding variant survives that objection under a
proviso ("refuse, never alter"). The ruling came from elsewhere: `build` returns
`Split< S >`, and `ring_registry::Registry< T >` **owns** the `Split< T >` values
put into it, so a factory holding a registry would own every ring it ever built.
That is not a second *input*; it is a second *lifetime*, and no proviso about
refusing-versus-altering touches it. → [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md).

**The finding is that the objection this file raised was the weaker of the two
available**, and it was the one that kept the question open. The decisive
objection needed a fact from a dependency — what a registry retains — which was
unknown when this file was written and unmentioned as a thing to go and find
out. An open question with a listed proviso reads as *awaiting a ruling*; this
one was awaiting a *fact*, and those want different follow-up.

**No `Default` derive.** ✅ Held — the implementation derives `Debug` and
`Clone` only. A `Default` impl is a second constructor
(→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)), and
while `Factory::default()` producing a fieldless factory is harmless, it would
become a way to conjure a registry-holding factory with an empty registry if N2
ever resolved the other way.

**That justification is now retrospective, and the omission survives it.** N2 is
ruled and the hazard it guarded against cannot arrive. What remains is
`invariant/002`'s plainer reason — one construction path, not two — which never
depended on N2 at all. **A rule kept for a reason that expired needs its
surviving reason stated**, or the next reader deletes it correctly by the
argument written down and incorrectly by the one that matters.

### Validation

**There is nothing to validate, and that is a property worth stating rather
than a gap.**

| Rule | Holds because |
|------|---------------|
| A `Factory` value is always usable | It has no fields, so no field can be in a bad state |
| Two `Factory` values are interchangeable | Same — ✅ **unconditionally**, N2 having resolved fieldless. Asserted by `two_factories_build_identically` |
| Constructing one cannot fail | No inputs |
| Dropping one loses nothing | It owns nothing; the rings it built are owned by their handles (→ [`lifecycle/002`](../lifecycle/002_the_factory_outlives_nothing.md)) |

**The second row was where the two variants diverged observably, and it is now
a test.** "Which factory built this ring" is not a meaningful question: a ring
built by factory A and looked up in factory B's registry is *found*, because the
registry is the caller's and neither factory is party to it. The failure mode
this row anticipated cannot be constructed.

**Derives:** `Debug` and `Clone`, as specified — ✅ matched by the
implementation. `Copy` is free too and is still withheld, though the reason
given here (the field N2 might add is a registry) has expired with N2. The
surviving reason is smaller and still sufficient: `Copy` is a promise about
every future version of the type, and this one has no call site that wants it.
Withholding costs nothing; granting is irreversible.

**`#[ non_exhaustive ]` does not apply.** It guards against a downstream crate
constructing or exhaustively matching a type across a version boundary, and
this crate is `publish = false` inside a workspace that builds as a unit:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'publish' ring_factory/Cargo.toml
```

Live output:

```
publish = false
```

Adding it would cost the ability to write `Factory` as a literal in this
workspace's own tests, in exchange for a compatibility guarantee no consumer
can currently need.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | The operations this type carries |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | N2's operation — the one that did not, in the end, give the type a field |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | What a build produces, and who owns it afterwards |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | Why the fieldless definition, and the proviso N2 must satisfy |
| [../invariant/002_construction_is_the_only_path.md](../invariant/002_construction_is_the_only_path.md) | Why no `Default` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_factory_outlives_nothing.md](../lifecycle/002_the_factory_outlives_nothing.md) | The last validation row, worked out |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_way_in.md](../pattern/002_one_way_in.md) | N1 — the vocabulary a nominal type gives the Contract |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_name_state_through_a_registration.md](../lifecycle/004_name_state_through_a_registration.md) | N2's proviso — which turned out not to be the load-bearing objection |

### Types

| File | Relationship |
|------|--------------|
| [002_build_error.md](002_build_error.md) | What the operations on this type may return instead of a ring |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | `publish = false`, which is why `#[ non_exhaustive ]` does not apply |
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | N1 — `ring_factory` as one of the five Contract names |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | `two_factories_build_identically` asserts the second validation row. Written as a real check of a variant that could have failed; N2 resolving fieldless makes it a guard against reintroducing the field rather than a discriminator between live options |

### FC45 — The Only Way to Obtain One Is the Literal, and That Is Not Stated Anywhere

`Factory` has no constructor and no `Default`:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- constructors and conversions on the type --'
for k in 'impl Default' 'fn new' 'impl From' 'impl Drop' 'derive'; do
  printf '  %-13s %s\n' "$k" \
    "$( command grep -vE '^\s*(///|//!)' ring_factory/src/lib.rs | command grep -c "$k" )"
done
echo '  -- how the tests obtain one --'
command grep -oE 'Factory[.;)]|Factory$' ring_factory/tests/factory_test.rs | sort | uniq -c
```

Live output:

```
  -- constructors and conversions on the type --
  impl Default  0
  fn new        0
  impl From     0
  impl Drop     0
  derive        2
  -- how the tests obtain one --
     23 Factory.
      2 Factory;
```

Zero `Default`, zero `new`, one derive line. Twenty-five uses in the suite: 23
write the bare unit literal and call a method on it in the same expression, 2
bind it to a name first. There is no third form, because there is no other way
to get one.

The absence of `Default` is argued for at length in the source — a `Default`
would suggest there is a non-default. The absence of `new` is not mentioned, and
it is the one a Rust reader will look for first: `Factory::new()` is the idiom,
its absence is deliberate for the same reason, and a reader who does not find it
has to infer from the unit-struct declaration that the literal is the
constructor.

Which is fine, and is exactly the sort of thing a `### Sources` row cannot carry:
the type's documentation explains one omission carefully and leaves the adjacent
one, with the same justification, to be reconstructed. Both omissions are right.
One of them is a sentence.

### FC46 — Three Methods, All `&self`, Zero Fields: the Type Is a Namespace With a Receiver

The N2 question — whether the registering variant would give the factory a field
— is settled, and it settled against:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the type and every method on it --'
command grep -E '^pub struct Factory;|^  pub fn ' ring_factory/src/lib.rs | sed 's/(.*//'
echo '  -- every receiver --'
command grep -oE '\( ?&self|\( ?&mut self|\( ?self' ring_factory/src/lib.rs | sort | uniq -c
```

Live output:

```
  -- the type and every method on it --
pub struct Factory;
  pub fn build< S : Send >
  pub fn build_named< S : Send >
  pub fn build_crossbeam< S : Send >
  -- every receiver --
      3 ( &self
```

A zero-sized struct with three `&self` methods that read nothing from the
receiver. Replacing `Factory` with three free functions would change every call
site's spelling and nothing else about what the crate does.

The type earns its keep on the naming, and the naming argument is real:
`Factory::build` reads as a construction and `ring_factory::build` reads as a
free function that might be one of several. It is a weaker justification than
the one the instance opens with — "why it is a type at all rather than two free
functions" — because that framing implies a behavioural difference and the
measurement above shows there is none.

Stating it as *namespace with a receiver* is what makes the future edit clear: a
field on `Factory` would make the type carry something, and the reason to refuse
that is [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)
FC21's zeros — not any property of the type as it stands today.
