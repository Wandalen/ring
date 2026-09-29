# docs

Design documentation for `ring_slot`, as typed doc definitions. Scope of the
crate: slot payload views — typed and raw bytes.

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

13 definitions, 26 instances, 52 findings. Every finding is indexed in
[`definition/readme.md`](definition/readme.md), and every one is verified by a
command whose output is quoted at the point it is used.

### What a Tier 1 Leaf Is Documented For

`ring_slot` is 288 lines. It declares `Slot`, a trait with two methods and no
payload; `TypedSlot< T >`, which is an `Option`; and `BytesSlot< N >`, which is
an array and a length. There is no loop, no atomic, no `unsafe`, no allocation,
and exactly one branch in the whole crate.

Fifty-two findings came out of it, and **nine are about other crates** — a much
smaller share than a Tier 5 primitive yields, and for the opposite reason. With
one dependency beneath it there is almost nothing to audit downward; what this
crate finds, it finds *upward*, in the consumers its shape reaches. Reading it
carefully located a claimed-slot protocol check compiled out of release builds in
`ring_core` (SL40), a mandatory recycle step in `ring_event` whose omission
produces no error of any kind (SL29), a `Buffer::is_empty` in `ring_store` that
is always `false` and exists to satisfy a lint (SL16), and a promised bench in
`ring_bench` that does not exist (SL4). None of those is in this crate's source.

### The Three Threads Running Through the Corpus

**One — one trait, two shapes, and one word for both.** The founding ruling is
right and enforced harder than it was asked to be: a trait rather than a
convention, which is why four crates stayed generic across three layers (SL5).
What it does not carry is any acknowledgement that the two implementors are
different kinds of thing. They disagree about what emptiness means (SL8, SL12),
about what `clear` does (SL31, SL43), about what `Clone` costs (SL35), about
which of the family's two publish models applies (SL30), and about why `Copy` is
absent (SL45). Every one of those disagreements is correct. Not one is written
down, and the prose everywhere says "a slot".

**Two — the tail nobody erases.** A `BytesSlot` holds the longest payload ever
written to its position, not the current one (SL44). Four accessors define the
slot's value as its first `len` bytes; two derives once defined it as all `N`
(SL42). `clear` moves the length and leaves the bytes (SL32, SL43). The residue
used to be unreachable through `read()` and fully reachable through `Debug`, so
it surfaced in exactly the place a reader is least prepared for it — a
diagnostic dump or an assertion failure message. The one test positioned to
catch the split, `slots_compare_by_payload_not_by_tail`, used to assert the
opposite and pass, because both its fixtures were written once from empty
(SL41). `Debug`, `PartialEq`, and `Eq` are now hand-written over `read()`
instead, so all six of `BytesSlot`'s public observations agree; only `Clone`
still copies the full tail, unpinned by any test (SL44's disposition).

**Three — the second shape is barely used, and the argument for it was never
measured.** `BytesSlot` exists because copying a large payload into a
`TypedSlot` was judged too expensive — an argument made in prose, calling for
its own measurement (SL6), and measured nowhere: `ring_bench` benches
`TypedSlot` twice and never instantiates the other shape (SL4). Meanwhile
`BytesSlot` has exactly one library consumer in the entire family (SL3), and the
unified handle at `ring_core` hard-wires `TypedSlot` at thirteen sites, so the
traffic the second shape exists for has no route through it (SL2). The design is
defensible on every reading; it is unfalsifiable in its own repository.

### Reading Order

| If you want | Start at |
|-------------|----------|
| what the crate does, in one bound check and one copy | [`algorithm/001`](algorithm/001_write_is_a_bound_check_and_a_copy.md) |
| the founding argument, and what was never measured | [`decisions/001`](decisions/001_two_shapes_rather_than_one.md) |
| where the second shape actually reaches | [`integration/002`](integration/002_where_the_second_shape_stops.md) |
| the measured costs of a fixed-width slot | [`data_structure/001`](data_structure/001_sixteen_bytes_to_carry_eight.md) |
| the ways correct-looking code goes wrong | [`pitfall/001`](pitfall/001_the_test_that_names_a_property_the_type_lacks.md) |
| every finding at once, ranked by severity | [`definition/readme.md`](definition/readme.md) |
