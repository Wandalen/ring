# Workaround: The Constructor That Exists for the Test Directory

### Scope

- **Purpose**: Price what `Claim::new`'s public visibility costs the crate above this one, and identify the project convention that is the real reason for it.
- **Responsibility**: Show that a convention held without exception across 33 crates forces 32 constructors public, that this is what stops `Claim` from being an enforceable precondition, and that it is nonetheless a trade worth keeping.
- **In Scope**: What a forgeable `Claim` can and cannot guarantee, the `tests/` convention that keeps it forgeable, and the three standard escapes the family has never used.
- **Out of Scope**: That `Claim::new`'s doc names a caller which does not exist, and the call-site census proving it — owned by [`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md) § CL12 and taken as given here. Workarounds for the language rather than the project are [`workaround/001`](001_two_const_functions_the_compiler_refuses.md). The proposal being priced is [`ring_publish` `pitfall/001`](../../../ring_publish/docs/pitfall/001_publishing_a_range_you_never_claimed.md) § PB35.

### The Stated Reason

```rust
/// A claim of `len` sequences beginning at `start`.
///
/// Public because `ring_publish` and the test suites of both crates need to
/// construct one directly; a producer obtains real claims from
/// [`Claimer::claim`], which is the only path that establishes exclusivity.
pub const fn new( start : Seq, len : usize ) -> Self   // :104-118
```

Two justifications, and the load-bearing half is the second clause — *a producer
obtains real claims from `Claimer::claim`*. That sentence is doing the work a
type would otherwise do, which is the whole subject of this file.

### CL53 — What the Public Constructor Costs the Signature Above It

That the first justification is false — `ring_publish` constructs zero `Claim`s
— is established with its full call-site census in
[`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md) § CL12
and is not re-derived here. Taken as given: all 23 sites are inside this crate,
and the only justification that survives is **the test suites**, which are
separate crates and therefore see only what `pub` exposes.

This file asks the next question instead. `ring_publish`'s
[§ PB35](../../../ring_publish/docs/pitfall/001_publishing_a_range_you_never_claimed.md)
proposes the fix in the other direction — put `Claim` on `publish`'s signature,
so the two values arrive as a unit that a `Claimer` produced. What does a public
constructor cost that proposal?

`Claim` is two integers with a `pub` constructor, so anyone can write:

```rust
let forged = Claim::new( Seq( 9_999 ), 4 );   // never claimed, never gated
```

Which makes the type a *label*, not a *capability*:

| If `publish` took a `Claim` | Enforced? |
|-----------------------------|:---------:|
| the two values arrive together, in the right order | ✔ |
| the range is half-open and self-consistent | ✔ |
| the range was actually granted by a `Claimer` | **✘** — `Claim::new` is public |
| the range is published once | ✘ — `Claim` is `Copy` |

**Two of four, not four of four.** The signature change is still worth making —
two enforced preconditions beat zero — but the crate above would inherit a type
that *looks* like proof of a grant and is not, which is a worse failure mode than
an honest pair of integers if anyone believes it. PB35 argues for the change and
does not price it; this is the price.

Only the third row is recoverable, by making `new` private. What prevents that is
not the language.

### CL54 — A Convention Held 33 of 33 Times Forces 32 Constructors Public

The rule is the project's, not the language's: tests live in the crate's
`tests/` directory. It is held without a single exception.

```sh
cd "$(git rev-parse --show-toplevel)"

# real (non-comment) #[ cfg( test ) ] modules anywhere in the family's sources
for f in ring_*/src/*.rs; do
  n=$( grep -vE "^[[:space:]]*//" "$f" | grep -cE 'cfg\( *test *\)' )
  [ "$n" -gt 0 ] && printf '%-42s %s\n' "$f" "$n"
done

ls -d ring_* | wc -l
ls -d ring_*/tests | wc -l
ls  ring_*/tests/*.rs | wc -l
```

Live output:

```
33
33
41
```

| | Count |
|--|------:|
| Crates | 33 |
| …with a `tests/` directory | **33** |
| Integration test files | 41 |
| …`#[ cfg( test ) ]` modules in any `src/` | **0** |

The single textual match for `cfg(test)` in the family's sources is inside a doc
comment in `ring_flush` (`:245`), where it is *named and rejected* for a
different purpose — "`cfg(test)` (invisible to integration tests)". The family
has therefore reasoned about this exact property once, correctly, and drawn the
consequence nowhere else.

Because an integration test is a separate crate, everything it touches must be
`pub`. That is a per-crate tax paid 32 times:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rcE '^  pub (const )?fn new\(' ring_*/src/*.rs | awk -F: '{s+=$2} END {print s}'
grep -rc  'doc( hidden )'            ring_*/src/*.rs | awk -F: '{s+=$2} END {print s}'
grep -rc  'pub( crate ) fn new'      ring_*/src/*.rs | awk -F: '{s+=$2} END {print s}'
```

Live output:

```
32
0
0
```

| | Count |
|--|------:|
| `pub fn new` constructors family-wide | **32** |
| …restricted by `#[ doc( hidden ) ]` | **0** |
| …restricted to `pub( crate )` | **0** |
| …behind a test-only cargo feature | **0** |

Three standard mechanisms exist for *public to the test crate, closed to
everyone else* — `doc( hidden )`, a `test-util` feature, or a `pub( crate )`
constructor plus an in-`src` test module. The family uses none of them, in any
crate, for any type. Four crates do have a `[features]` section, so the
machinery is not unfamiliar; it has simply never been pointed at this.

So the third row of CL53's table is held open by a convention, not by a language
rule — and that convention is a genuine trade, not an oversight to correct. A
rule that keeps 37 test files honest by forcing every one of them through the
public API is worth more than one type's unforgeability, and it is the same rule
that makes every doctest in this crate a real compiled example rather than a
private-internals demonstration. The tax is 32 public constructors; the return is
that no test in the family can assert something a caller could not also observe.

What is missing is not the decision. It is that the decision was never noticed
as one — `Claim::new`'s doc explains its visibility by naming a caller that does
not exist, so the reader never learns that a project convention, not a
requirement, is what holds the door open.

### Workarounds

| File | Relationship |
|------|--------------|
| [001_two_const_functions_the_compiler_refuses.md](001_two_const_functions_the_compiler_refuses.md) | The constraints that come from the language rather than the project |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seventeen_items_and_nothing_that_drops_silently.md](../api/001_seventeen_items_and_nothing_that_drops_silently.md) | **§ CL12 owns the false-justification finding and the full call-site census**; this file takes it as given and prices the consequence |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_dropping_a_claim.md](../pitfall/001_dropping_a_claim.md) | The `Copy` half of the table above — why one published range can be published twice |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_two_producers_hold_one_sequence.md](../invariant/001_no_two_producers_hold_one_sequence.md) | The property a non-forgeable `Claim` would help carry across the crate boundary |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependents_that_split_one_feature.md](../integration/001_two_dependents_that_split_one_feature.md) | The dev-dependency edge that makes `ring_publish` a test-time consumer only |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_seq_a_usize_and_three_casts.md](../type/001_a_seq_a_usize_and_three_casts.md) | `Capacity` — the family's one newtype that *does* hide its field, and why |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:104-118` | The `pub const fn new` whose visibility this file prices |
| `ring_publish/Cargo.toml` `[dev-dependencies]` | `ring_claim` as a test-time edge, with the acyclicity note |
| `ring_flush/src/lib.rs:257` | The family's one written consideration of `cfg(test)`, rejecting it for a different reason |
| `ring_types/src/capacity.rs:23,40` | The counter-example — a private field because there is an invariant to hold |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs` | The pure-value tests that are the real and only reason the constructor is public — census in [`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md) § CL12 |
| `tests/manual/readme.md § C5` | The dependency check, which walks the edge that makes `ring_publish` a dev-dependency |
| — | **No test constructs a `Claim` that a `Claimer` never granted**, so nothing exercises the gap this file records |
