# docs

Design documentation for `ring_store`, as typed doc definitions. Scope of the
crate: power-of-two slot storage array.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Step sequences and their termination arguments |
| `api/` | The public surface as a contract |
| `data_structure/` | Memory layout and what it costs |
| `decisions/` | Rulings taken, with the alternatives priced |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Edges to other crates, in and out |
| `invariant/` | Properties that must hold, and what verifies them |
| `item/` | Public items one at a time |
| `lifecycle/` | States and transitions over time |
| `non_functional_requirement/` | Properties of the crate rather than its behaviour |
| `pattern/` | Shapes this crate shares with siblings |
| `pitfall/` | Correct-looking code that goes wrong |
| `type/` | What the types commit to |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

13 definitions, 26 instances, 53 findings. Every finding is indexed in
[`definition/readme.md`](definition/readme.md), and every one is verified by a
command whose output is quoted at the point it is used.

### What a Tier 2 Storage Crate Is Documented For

`ring_store` is 261 lines. It holds a `Box< [ S ] >` and a `Capacity`, publishes
twelve functions, and owns no protocol, no atomics, no cursor and no policy. Even
the fold from a sequence to a slot — the one computation the name suggests — is
delegated to `ring_index`.

Fifty-three findings came out of it, and **eight are about other crates**, every
one an immediate neighbour: the crate below that defines the slot, the one that
owns the fold, and the one that manufactures indices against a capacity of its
own choosing. Nothing was found further away, which is the finding. A storage
tier reaches nowhere — and is reached into by everything: all twelve `unsafe`
blocks in the family dereference into this array (BF5).

### The Three Threads Running Through the Corpus

**One — the code is smaller than its documentation.** Almost every doc sentence
that reaches past the boxed slice reaches into a neighbour, and about half of
those are wrong about the neighbour. `clear` says it exists for `ring_shutdown`,
a crate with no dependency on this one that never mentions a buffer (BF32), and
states the family's strongest payload-visibility guarantee at a tier that can
only forward `Slot::clear` — which for `BytesSlot` leaves every byte resident
(BF33). `get`'s `# Panics` explains an out-of-range index as two rings mixed, a
claim about provenance the type cannot carry and the crate's own tests violate
twenty-five times (BF7). And this crate's reached-test, the contract it
is verified against, names an indexed `set` that does not exist (BF23). The code
is right in each case; the sentence beside it is not.

**Two — the strongest properties have the weakest tests.** One allocation per
buffer, sized exactly `capacity * size_of::< S >()`, and zero allocations across
ten thousand subsequent operations — measured here for the first time, asserted
by no test (BF42, BF12). A 24-byte handle at every capacity and every slot type,
where the stand-in test pins one instantiation (BF18, BF19). And the two
arguments to `with_capacity` and `resize_with`, which must stay equal or
`into_boxed_slice` silently adds a reallocation, with nothing checking they do
(BF51). Meanwhile the test that looks like it guards non-aliasing asserts a
property of slices rather than of `Buffer` (BF24).

**Three — two questions share the name `is_empty`.** `Buffer::is_empty()` returns
`false` for a buffer holding nothing; it is `const`, `#[ must_use ]`, correct,
and answers a question no caller is asking, so `if buffer.is_empty()` guards a
branch that can never be taken (BF38). The question a caller means is
`all_empty`. There are seventeen `is_empty` methods family-wide, and where the
two buffer types meet, `ring_tls`'s tests resolve the collision by binding the
`ring_store::Buffer` to a variable named `ring` — a convention with no comment
and nothing enforcing it (BF39). Beside it sits the same shape one level down:
`get_mut` and `at_mut` carry no `#[ must_use ]` where their shared-borrow twins
do, so discarding either compiles clean under `-D warnings` and does nothing
(BF40).

### Reading Order

| If you want | Start at |
|-------------|----------|
| what the crate does, in one mask and one slice index | [`algorithm/001`](algorithm/001_one_mask_no_modulo.md) |
| the properties that make it worth a crate, measured | [`non_functional_requirement/001`](non_functional_requirement/001_allocate_once_then_never_again.md) |
| who depends on it, and who reaches into it | [`integration/002`](integration/002_every_unsafe_block_in_the_family.md) |
| the ways correct-looking code goes wrong | [`pitfall/001`](pitfall/001_the_two_questions_named_is_empty.md) |
| why three lines of construction are three lines | [`workaround/001`](workaround/001_three_steps_to_avoid_two_bounds.md) |
| every finding at once, ranked by severity | [`definition/readme.md`](definition/readme.md) |
