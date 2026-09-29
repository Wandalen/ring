# docs

Design documentation for `ring_index`, as typed doc definitions. Scope of the
crate: sequence-to-slot index mapping for power-of-two capacities.

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

13 definitions, 26 instances, 56 findings. Every finding is indexed in
[`definition/readme.md`](definition/readme.md), and every one is verified by a
command whose output is quoted at the point it is used.

### What a Seventeen-Line Crate Is Documented For

`ring_index` is 122 lines, of which 100 are doc comment, 5 are blank, and 17 are
code. It declares no type, holds no state, has no `impl` block, and its central
function is one `and` instruction. Everything a caller acts on is prose.

Fifty-six findings came out of it, and **twenty-two are about other crates** —
`ring_types` (8), `ring_mpsc` (5), `ring_batch` (3), `ring_store` (2),
this family's own feature record (2), `ring_seqno` (1), `ring_bench` (1). That ratio is the
finding. A crate this small has almost no surface of its own to be wrong about;
what it has instead is a claim — *this fold is the only one* — that can only be
checked by reading everyone else.

### The Three Threads Running Through the Corpus

**One — the crate's job is a claim about other crates, and the claim is 3-for-4.**
`ring_store` folds through `ring_index`; `ring_batch` imports `of` and asserts
in a test that it did (IX48); `ring_spsc` reaches it transitively through
`Buffer::at` without naming it (IX10). `ring_mpsc` holds two ring-shaped storages
in one struct — `slots` wrapped in a `Buffer` and `stamps` as a bare
`Box< [ AtomicSeq ] >` seven lines below — and folded the second one by hand
(IX12). The rule that predicts exactly this is not "one owner for the fold" but
"the fold travels with the container" (IX42), and what makes the hand-written
version a hazard rather than a duplicate is that its mask reads `consumers`'
capacity while indexing an array sized from a constructor argument stored nowhere
(IX19). Nothing runs the census that finds any of this: no lint, no test, no CI
step (IX41).

**Two — the module comment is wrong about its own justification, twice.** It says
an integer `%` costs "20–40 cycles on current x86" and sits on "the claim path
and the read path of every single operation the family performs." Measured on
this host: a runtime-divisor modulo is 6.2 cycles against the mask's 0.9 — 7×,
not 20–40× — and a *constant*-divisor modulo is free, so the naive form of the
benchmark measures nothing at all (IX21, IX22). The architecture named is not the
one it runs on. And the claim path does not fold: `may_claim` compares sequences,
and no claim in the family computes a slot (IX11). The error was inherited
verbatim from this family's own cited feature record, which is still marked `planned` while three
crates implement it (IX13, IX15).

**Three — the sentences are load-bearing and three of them are false.**
`of`'s comment says two sequences "exactly one lap apart" alias, one lap narrower
than the property it computes — and the general form is stated correctly forty
lines below, on `aliases`, which nothing calls (IX33). `ring_types::Seq::next`
says release builds *saturate* where Rust's `+` wraps, and its own stated
rationale names exactly the failure the code has (IX17). `SlotIndex`'s doc says
it is "never constructed by counting" two lines above a doctest that counts one
and a `pub usize` field that permits it — though 35 of the 36 constructions
family-wide are tests and the 36th is `of` (IX39). In a crate with a 1:6
code-to-doc ratio, a wrong sentence is a defect of the same order as a wrong
expression (IX34).

### Reading Order

| If you want | Start at |
|-------------|----------|
| what the crate does, in one instruction | [`algorithm/001`](algorithm/001_one_and_of_a_mask.md) |
| the cycle count the module comment gets wrong, measured | [`non_functional_requirement/001`](non_functional_requirement/001_what_the_fold_costs.md) |
| who folds through it and who did not | [`integration/001`](integration/001_two_dependents_and_a_third_that_did_it_again.md) |
| the ways correct-looking code goes wrong | [`pitfall/002`](pitfall/002_the_second_fold_nobody_noticed.md) |
| why the crate exists at all, and what deletes it | [`workaround/001`](workaround/001_the_mask_that_lives_one_crate_up.md) |
| every finding at once, indexed by definition | [`definition/readme.md`](definition/readme.md) |
