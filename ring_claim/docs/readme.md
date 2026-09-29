# docs

Design documentation for `ring_claim`, as typed doc definitions. Scope of the
crate: sequence-range claiming without waiting.

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

### What a Crate of Two Structs Is Documented For

`ring_claim` is 461 lines. It defines `Claim`, which is two integers, and
`Claimer`, which is a cursor and a borrowed gate. Its whole behaviour is three
comparisons and a compare-exchange in a loop.

Fifty-five findings came out of it, and **twenty are about other crates.**
That ratio is the answer to why a crate this small is documented this heavily:
a Tier 5 primitive is where four tiers of decisions arrive and one tier of
consumers depends, so it is the cheapest place in the family from which to audit
everything it touches. Documenting it found a false sentence in `ring_types`
(CL34), the rejected design shipped public and uncalled in `ring_batch` (CL20),
an unbounded spin in `ring_publish` whose termination rests on a lint two
tiers away (CL31), and a heap allocation on every `claim` coming out of
`ring_cursor` (CL55, since removed by `b7e075ca`) — none of which is in this
crate's source.

### The Three Threads Running Through the Corpus

**One — the code is right and the prose is not.** Eleven of thirteen definitions
reach this independently. Two docs state something measurably false (CL34,
CL46); two name a caller or purpose that does not exist (CL12, CL30); four
describe an exceptionless discipline that is written down nowhere (CL35, CL47,
CL50, CL54). Not one is a bug. The uniform cost is that a reader who trusts the
prose over the source is misled, and in four cases the source is two crates away.

**Two — the silent failures are silent at the point of the mistake.** Eight
findings are reachable hazards, and they share a shape: nothing fires where the
error is made. Four routes to a stranded claim compile with zero warnings
(CL43); the resulting stall surfaces in a different thread, at a later call, as
an ordinary-looking `Full` (CL44); one omission fails only in a downstream crate
(CL52); one requires reading a crate two tiers up to know it is wrong (CL20);
one is a single line of safe code against a public trait (CL33).

**Three — what is enforced is not what is fragile.** The one property with teeth
is `unsafe`, guarded by two lints, in a crate that would never have used it. The
properties a plausible change actually breaks — no allocation, never blocks, and
the `no_std`-cleanliness of a seven-crate chain — were guarded by nothing (CL35,
CL36), and the first of those was not merely unguarded but already false: every
`claim` allocated, two crates down, and only measuring it found that — the grep
standing in for the check covered one file where the claim covered seven (CL55).
That one now has teeth: `tests/allocation_test.rs` counts allocations across the
gate read, both claim paths and the refusal path, and the other two still have
none.
`Claim`'s `must_use` carries the most severe message in the family and
has no runtime mechanism behind it at all (CL21). Meanwhile `Claimer : Sync`,
which the entire multi-producer design rests on, is enforced only by accident:
five `thread::scope` tests would fail to compile without it, and none of them
says so (CL50).

### Reading Order

| If you want | Start at |
|-------------|----------|
| what the crate does and why the loop is shaped that way | [`algorithm/001`](algorithm/001_the_gate_inside_the_retry.md) |
| the founding argument against the obvious alternative | [`decisions/001`](decisions/001_compare_exchange_rather_than_fetch_add.md) |
| the property everything else exists to protect | [`invariant/001`](invariant/001_no_two_producers_hold_one_sequence.md) |
| the ways correct-looking code goes wrong | [`pitfall/001`](pitfall/001_dropping_a_claim.md) |
| every finding at once, ranked by severity | [`definition/readme.md`](definition/readme.md) |
