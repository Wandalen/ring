# Decision: The Record Type Is A `u32`

### Scope

- **Purpose**: Promote [`readme.md`](readme.md) Pending 1 to an instance, because the crate has since grown a measurement the register entry does not account for.
- **Responsibility**: The decision, the argument for it, the half of the crate that already contradicts it, and what the settlement condition actually costs.
- **In Scope**: `Script::run`'s `Ring< u32 >`; the generic surface beside it.
- **Out of Scope**: The scripting shape itself (→ [`data_structure/001`](../data_structure/001_the_script_as_a_flat_step_list.md)); the audit that consumes the record type (→ [`algorithm/002`](../algorithm/002_the_four_passes_of_an_audit.md)).

### The decision

`Script::run` takes `&mut Ring< u32 >`. A caller whose real records are `MyEvent`
drives the same *shape* of sequence and reads the outcome; they cannot drive
their own records through it.

**Status: pending.** The register entry is [`readme.md`](readme.md) Pending 1 and
remains open. This document does not settle it — it records what has been
measured since it was written.

### The argument as recorded

| # | Claim | Where |
|---|---|---|
| R1 | A generic script must be told how to produce records — a closure, a seed, an iterator | Pending 1 |
| R2 | Every one of those is a way for two runs to differ | Pending 1 |
| R3 | Consecutive `u32`s from `0` make identical minting true by construction | Pending 1, and `Step`'s enum doc |
| R4 | `Outcome`'s `PartialEq` — the comparison the claim rests on — would need `T : PartialEq` | Pending 1 |

R1–R3 are sound and are the reason the decision is right today: the crate's
central claim is that two runs of one script agree, and a caller-supplied record
source is the one thing that could break it without the ring being at fault.

### What has been measured since

**The crate is already half generic.** `leak` and `leak_ends` are
`fn ...< T : Send >( ring : Ring< T > )` and sit in the same file, eighty-one
lines above `Script::run`. The loom bridge — the half of this crate that exists
to reach `loom`'s model — is parametric in the record type today; only the
scripted half is not.

**The provenance pass needs more than `PartialEq`.** `audit_received`'s first
loop is `value >= minted`. That is an ordering comparison between a record and a
mint counter, and no `T : PartialEq` supplies it. A generic `Outcome` would need
a way to ask *"could this value have been minted by this run"* that does not
assume the value is a number — which is a trait the crate would have to define,
not one it could name.

### The settlement condition, restated

Pending 1 says the condition is *"a consumer whose record type has behaviour the
fixture needs to exercise — a `Drop` impl whose runs should be counted, say, or a
payload whose bytes should be checked on the way out."*

That is still the right trigger. What the measurement adds is the price:

| Piece | Today | Generic |
|---|---|---|
| `leak`, `leak_ends` | `T : Send` | unchanged |
| `Outcome::received` | `Vec< u32 >` | `Vec< T >`, `T : PartialEq + Debug` |
| minting | `record = minted; minted += 1` | a caller-supplied source, and R2 |
| `audit_received`'s provenance pass | `value >= minted` | a new trait, or the pass is dropped |

Three of the four rows are mechanical. The fourth is a design question the
register does not currently name.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'generic public functions:      %s\n' "$( command grep -cE '^pub fn [a-z_]+< T : ' src/lib.rs || true )"
printf 'monomorphic on u32:            %s\n' "$( command grep -c 'Ring< u32 >' src/lib.rs || true )"
printf 'lines from leak to run:        %s\n' "$( awk '/^pub fn leak</{a=NR} /  pub fn run\(/{print NR-a; exit}' src/lib.rs )"
printf 'the provenance comparison:     %s\n' "$( awk '/pub fn audit_received/{f=1} f && /value >= minted/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'register entries, closed:      %s\n' "$( command grep -c '^### Closed ' docs/decisions/readme.md )"
printf 'register entries, pending:     %s\n' "$( command grep -c '^### Pending ' docs/decisions/readme.md )"
```

Live output:

```
generic public functions:      2
monomorphic on u32:            5
lines from leak to run:        101
the provenance comparison:     if value >= minted
register entries, closed:      5
register entries, pending:     4
```

### Decisions

| File | Relationship |
|------|--------------|
| [readme.md](readme.md) | Pending 1, the register entry this promotes |
| [002_the_model_lives_in_tests.md](002_the_model_lives_in_tests.md) | Closed 2, the other entry that grew a measurement |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | The signatures the record type appears in |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_four_passes_of_an_audit.md](../algorithm/002_the_four_passes_of_an_audit.md) | The pass that would need a new trait |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Script::run`, `leak`, `leak_ends`, `audit_received` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | `a_script_runs_the_same_against_a_leaked_ring` — the one test that crosses both halves |

### TK13 — half the surface is already generic

`leak< T : Send >` and `leak_ends< T : Send >` are parametric in the record type.
`Script::run` is not. They are in the same file, eighty-one lines apart, and
both are part of the same feature.

Pending 1's argument is entirely about minting: a generic script has to be told
how to produce records, and every way of telling it is a way for two runs to
differ. That argument is correct and it does not reach the two functions that are
already generic, because neither of them mints anything — they take a ring the
caller built and hand back its ends.

The register presents the choice as "should a `Script` be generic", which reads
as a question about the crate. It is a question about one function. The crate's
answer to the same question, asked about the loom bridge, was yes, and that is
not recorded in the entry.

### TK14 — the settlement condition names one trait bound and needs two

Pending 1 identifies the cost as `Outcome`'s `PartialEq` needing `T : PartialEq`.
That is real and it is not the binding constraint.

`audit_received`'s provenance pass is `if value >= minted`. Generically, that is
the question *"is this a value this run could have produced"*, and it has no
answer for an arbitrary `T`: `PartialOrd` would type-check against a `T`-valued
`minted` but is meaningless for a `MyEvent`, and the concept the pass actually
needs — *mintedness* — is not a standard trait at all.

So a generic `Script` has three outcomes and the register names none of them:
define a `Minted` trait the fixture owns, drop the provenance pass for generic
record types and keep it only for `u32`, or make the pass a caller-supplied
predicate — which is R1's closure arriving through a different door.

The entry is not wrong. It understates its own cost by the one piece that is a
design decision rather than a mechanical bound.
