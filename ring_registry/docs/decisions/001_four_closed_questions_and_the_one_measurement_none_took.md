# Decisions: Four Closed Questions and the One Measurement None Took

### Scope

**Purpose:** Examine what actually closed each of the four Closed entries in
`docs/decisions/readme.md`, and follow the contract Closed 2 records out to its
only consumer, which discards it and says why.

**Responsibility:** The evidence type behind each of the four Closed entries; the
one quantity that appears in the file and where it came from; the discriminating
test Closed 1 asks for; and what `ring_factory` does with the payload Closed 2
exists to preserve.

**In Scope:** `ring_registry/docs/decisions/readme.md:9`, `:18`, `:49`,
`:58`, `:29-30`; `ring_registry/tests/registry_test.rs:152`;
`ring_factory/src/lib.rs:205-207`, `:211-213`, `:214`.

**Out of Scope:** The three Pending entries are
[`decisions/002`](002_three_pending_questions_and_the_one_consumer.md). The
measurements themselves are
[`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md)
and [`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md).

---

## What Closed Each of the Four

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
d=ring_registry/docs/decisions/readme.md
echo '  -- the four closed questions --'
command grep '^### Closed' "$d" | sed 's/^/    /'
echo '  -- every quantity that appears anywhere in the file, Closed and Pending --'
command grep -oE '\*\*[0-9]+ bytes\*\*|O\( *[a-z0-9 ]+ *\)' "$d" | sed 's/^/    /'
echo '  -- the evidence each Closed entry names --'
command grep 'indistinguishable to every\|Three tests failed to compile\|failed the build under\|permits no operation\|never inspects what it stores' "$d" | cut -c1-92 | sed 's/^/    /'
echo '  -- the discriminator Closed 1 asks for, and where it was built --'
awk '/^fn [a-z_]+\(\)/{ n = $2 } /static DROPS/{ print "    " NR ": " n }' ring_registry/tests/registry_test.rs
echo '  -- every manifest outside this crate that names it --'
# excludes ./Cargo.toml itself: that workspace manifest lists every
# crate as a member path (including ring_registry), which is not the same
# claim as a manifest declaring ring_registry as a dependency
command grep -rl 'ring_registry' --include=Cargo.toml . | command grep -v '^\./Cargo\.toml$' | sed 's|^\./||' | command grep -v '^ring_registry/' | sed 's/^/    /'
echo '  -- what that consumer does with the ring the refusal hands back --'
command grep '_refused\|The refused .Split. comes back\|The name is discarded' ring_factory/src/lib.rs | cut -c1-92 | sed 's/^/    /'
```

Live output:

```
  -- the four closed questions --
    ### Closed 1 — `Entry` or `insert`?
    ### Closed 2 — Should `register` hand the rejected ring back?
    ### Closed 3 — Is there an immutable `get`?
    ### Closed 4 — Three dependency edges or one?
  -- every quantity that appears anywhere in the file, Closed and Pending --
    **448 bytes**
    O(log n)
    O(1)
  -- the evidence each Closed entry names --
    implementations are indistinguishable to every count-based and lookup-based
    | 1 | A `Debug` bound on the record type at `.unwrap()`/`.expect()` sites | Three tests fail
    | 2 | A `Result` **448 bytes** wide, on the `Ok` path as well as the `Err` path | `clippy::r
    `new`, and `ends( &mut self )`. A `&Split< T >` therefore permits no operation
    registry never inspects what it stores — it hashes a name, holds a value, lends
  -- the discriminator Closed 1 asks for, and where it was built --
    160: a_refused_registration_does_not_drop_the_ring_already_there()
    208: dropping_the_registry_drops_every_record_still_in_every_ring()
    274: a_removed_ring_carries_its_records_to_its_new_owner()
    378: assigning_through_get_mut_drops_the_ring_it_replaces()
  -- every manifest outside this crate that names it --
    ring_factory/Cargo.toml
  -- what that consumer does with the ring the refusal hands back --
        // The refused `Split` comes back in the error payload and is dropped as
          // The name is discarded rather than carried into `BuildError`: the caller
          Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),
```

---

### RG13 — Three Signatures, One Failing Build, and No Number Anybody Chose to Take

Four questions are recorded as closed, and the evidence behind them sorts into
exactly two kinds:

| # | Question | What closed it | Kind |
|---|----------|----------------|------|
| 1 | `Entry` or `insert`? | The two forms are "indistinguishable to every count-based and lookup-based test" | Reading the API |
| 2 | Hand the rejected ring back? | Three tests failed to compile; `clippy::result_large_err` failed the build | A machine refusing |
| 3 | Is there an immutable `get`? | `Split` has two methods and one takes `&mut self` | Reading a signature |
| 4 | Three dependency edges or one? | The registry never inspects what it stores | Reading the API |

Three of four were settled by reading. That is the correct method for all three —
whether `&Split< T >` permits an operation is a fact about a signature, not
something to benchmark, and Closed 3 and Closed 4 are right for exactly that
reason.

Closed 2 is the interesting one, and the file is unusually honest about it: both
of its costs "surfaced by a failing build rather than by reasoning about the
signature". That is the only place in the file where evidence arrived from
outside somebody's head. It is also the only quantity in the file — `**448
bytes**`, in Closed 2's cost table — and it came from a lint that was then
suppressed. The other two notations in the file, `O( log n )` and `O( 1 )` in
Pending 3's unordered-map rationale, are asymptotic classes rather than
measurements of anything.

**Finding.** Recorded as an evidence-type imbalance, not a wrong decision — every
one of the four conclusions survives measurement. What is missing is that the
crate's own reasoning apparatus has no habit of producing a number: 448 got in
because clippy printed it in a build failure, and the decisions file has no entry
whose settling evidence is something a person went and measured. Worth noting
against Closed 1 specifically, since
[RG2](../algorithm/001_two_branches_and_what_the_refusal_costs.md) finds the
`Entry` form measurably slower on the path Closed 1 is about — a conclusion the
file still reaches correctly, by an argument that never touched the thing that
turns out to be false.

Closed 1's own follow-through is the counter-example worth crediting: it names
the discriminator it lacks — a drop counter — and one was built.
`a_refused_registration_does_not_drop_the_ring_already_there` is that test, and
two more in the same file carry counters for the drop paths either side of it.

---

### RG14 — The Contract Closed 2 Records Is Discarded, Deliberately, by Its Only Consumer

Closed 2 is the crate's most argued decision: `register` returns the rejected
ring alongside the error so a name collision does not destroy it. Three costs are
paid for that — a 448-byte `Result` on both paths, a `Debug` bound on `T` at
`.expect()` sites, and a suppressed lint with a written `reason =`.

One manifest outside this crate names `ring_registry`: `ring_factory`. Its single
call site, the arm matching `RegistryError::NameTaken`, reads:

```rust
Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),
```

Both halves of the payload are dropped on the floor — the ring into `_refused`,
the name into `{ .. }` — and both discards carry a comment saying why. The ring
"is dropped as this arm ends. That is what makes the 'no ring was exposed'
guarantee free rather than checked — the caller never held it." The name is
discarded because "the caller passed it in and still has it", and carrying it
"would also put a `String` in a `Copy` error type for no new information."

Both reasons are good, and neither is specific to `ring_factory`. Any caller that
builds a ring immediately before registering it never held the ring either, and
any caller that passes a name in still has the name. That is the natural shape of
the call, and it is the shape the crate's own module doc, readme and doctests all
use. So the capability is available precisely to callers that construct a ring at
one point in a program and register it at another — of which there are none.

**Finding.** Recorded as an unexercised contract, not a wrong one. The decision
is cheap to keep and expensive to reverse, and preserving optionality on a
setup-time call is a reasonable trade. What is missing is the round trip:
`ring_factory` wrote down a careful consumer-side argument for why it does not
need the payload, `ring_registry` argues for four paragraphs that the payload is
essential, and neither file mentions the other. Closed 2 should carry one line —
that the sole consumer to date discards both halves and has documented why — so
the entry records the state of the world rather than only the intent.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](002_three_pending_questions_and_the_one_consumer.md) | The three open questions and the consumer they all wait on |
| [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md) | Closed 1's conclusion, measured |
| [`pattern/001`](../pattern/001_an_error_that_hands_the_payload_back.md) | The shape Closed 2 chose, against the family's six others |
| [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md) | The 448 bytes, and what it is a constant of |
| [`integration/001`](../integration/001_one_declared_edge_of_three.md) | Closed 4's edge count |

### Sources

| Fact | Where |
|------|-------|
| The four Closed entries | `ring_registry/docs/decisions/readme.md:9`, `:18`, `:49`, `:58` |
| The two costs and how each surfaced | `ring_registry/docs/decisions/readme.md:29-30` |
| The drop counter Closed 1 asks for | `ring_registry/tests/registry_test.rs:152` |
| The sole external manifest | `ring_factory/Cargo.toml` |
| Both payload halves discarded, with reasons | `ring_factory/src/lib.rs:205-207`, `:211-213`, `:214` |

### Tests

| Test | Covers |
|------|--------|
| `a_refused_registration_does_not_drop_the_ring_already_there` | Closed 1's discriminator |
| `a_refused_registration_hands_the_ring_back` | The contract Closed 2 records |
| `the_error_names_the_taken_name` | The name half of the payload |
