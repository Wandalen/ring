# docs

Design documentation for `ring_consume`, as typed doc definitions. Scope of the
crate: single-consumer available-range computation and commit.

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

13 definitions, 26 instances, 55 findings. Every finding is indexed in
[`definition/readme.md`](definition/readme.md), and every one is verified by a
command whose output is quoted at the point it is used.

### What the Read Half Is Documented For

`ring_consume` is 418 lines. It defines `Available`, which is two integers, and
`Consumer`, which is two borrows and no state at all. Its whole behaviour is one
subtraction, two comparisons, and two stores — straight-line code with no loop
anywhere, which is the exact inverse of the write half.

Fifty-five findings came out of it, and **twenty-five are about other crates.**
That ratio is the point of documenting a crate this small: a Tier 5 primitive is
where four tiers of decisions arrive, so reading it carefully is the cheapest
available audit of everything beneath it. Doing so measured an allocation on
every read inside `ring_cursor` (CN34, CN53, CN54), found `ring_wait` carrying
`std` and a blocking loop into a chain for a method nothing calls (CN5, CN35),
proved four `ring_seqno` functions const-able with their bodies unchanged (CN51),
located a `must_use` gap in `ring_batch` only visible from a three-way
comparison (CN40), and caught a `ring_trace` finding prescribing a substitution
its own crate had already declined in source without recording it (CN55) — none
of which is in this crate's source.

### The Three Threads Running Through the Corpus

**One — the crate does less, and the obligation moves to the caller.** Both
rulings in `decisions/` take the cheaper mechanism and are right to: two calls
instead of a guard, plain stores instead of compare-exchange. Each converts a
guarantee into a premise. The window between `available` and `commit` cannot be
guarded by any signature this crate could offer (CN7); the single-consumer
requirement the plain stores rest on is stated in prose and contradicted by three
`Consumer`s that compile and run over one cursor (CN9, CN50). Both arguments are
good. Neither says the resulting obligation is unbacked.

**Two — the stated properties were false and the enforced one is unbreakable.**
The crate names four non-functional properties. Three were wrong when measured:
every read allocated (CN34), the chain carries `std` (CN35), the chain blocks
(CN35). The fourth — no `unsafe` — is enforced by two workspace lints in a crate
that would never have used it (CN36). Of the three, one has since become true:
`b7e075ca` removed the allocation from `ring_cursor`, so the read path measures
zero. The two that remain are still guarded by nothing, and the allocation had
already been found once before, in `ring_claim`, by the same route — twice
found, and fixed only after the second finding.

**Three — the trap is always the reasonable expectation.** Reading the names,
`commit_available` should call `commit`; it duplicates the store instead, and is
correct today for an arithmetic reason rather than a structural one (CN43).
Measuring allocation, the simplest configuration should be representative; the
empty barrier is the one case that reports zero, which is why the cost survived
in two crates (CN46). Seeing a withheld `Copy`, the type should be protecting the
single-consumer invariant; it blocks only the accidental duplicate, and its own
accessors reconstruct it in one line (CN50).

### Reading Order

| If you want | Start at |
|-------------|----------|
| what the crate does, in three steps with no branch | [`algorithm/001`](algorithm/001_position_frontier_pending.md) |
| the founding argument, and the window it opens | [`decisions/001`](decisions/001_two_calls_not_one.md) |
| the property everything else protects, and where it actually lives | [`invariant/001`](invariant/001_never_reads_past_what_was_published.md) |
| the measured costs, against the stated ones | [`non_functional_requirement/001`](non_functional_requirement/001_what_the_read_path_costs.md) |
| the ways correct-looking code goes wrong | [`pitfall/001`](pitfall/001_commit_available_does_not_call_commit.md) |
| every finding at once, ranked by severity | [`definition/readme.md`](definition/readme.md) |
