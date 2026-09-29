# docs

Design documentation for `ring_publish`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | One exchange with two contracts, and the only unbounded loop in the family |
| `api/` | Six methods, no caller, and a `Result` whose error is not an error |
| `data_structure/` | Eight live bytes in sixty-four, and the four cursors around them |
| `decisions/` | Choices with live alternatives, recorded with their arguments |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Seven crates name it, none depends on it, and two changed shape for it |
| `invariant/` | Properties that must hold for every input, one of them an absence |
| `item/` | Per-item contracts and coverage, method by method |
| `lifecycle/` | Five states, four transitions, and the one that changes nothing |
| `non_functional_requirement/` | What a publication costs, and the three terms the spin's cost is built from |
| `pattern/` | Two conventions this crate is the reference instance of |
| `pitfall/` | Two ways to corrupt the ring with well-typed code |
| `type/` | One cast, two derives, and an overflow sentence that is wrong twice |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: making a claimed range visible to consumers, and not one
sequence earlier.

Start at [`definition/readme.md`](definition/readme.md) — it carries the full
instance table and the forty-six findings this corpus recorded.

### What a Crate With One Struct and One Loop Is Documented For

`ring_publish` is 222 lines. It declares one `struct` with one field, six public
methods, no `unsafe`, no `std::` path, and exactly one cast — `len as u64` at
`:163`, in the widening direction, against eleven `u64 as usize` casts elsewhere
in the family that would truncate on a 32-bit target
([`type/001`](type/001_a_seq_a_usize_and_the_one_cast.md) § PB39). Its executable
content is a compare-exchange at `:164` and a loop around it at `:200-209`.

What is left is what the crate is actually for: **choosing the one moment a
consumer is allowed to see a slot.** That moment is a boundary rather than a
computation, which is why a crate this thin carries a corpus this size. The
exchange's two contracts are the subject of
[`algorithm/001`](algorithm/001_the_compare_exchange_that_refuses.md); the loop
that turns *not yet* into *eventually* is
[`algorithm/002`](algorithm/002_a_loop_with_no_budget.md), the family's only bare
unbounded spin in any library; and the boundary itself — exclusive, so that
`is_published( published() )` is `false` — is asserted 320 times in
[`invariant/002`](invariant/002_is_published_is_exclusive_of_the_frontier.md).

### The Three Threads Running Through the Corpus

**A finished crate with nothing linked against it.** Seven crates name
`ring_publish` in prose and not one manifest depends on it
([`integration/001`](integration/001_ten_crates_name_it_and_none_depends_on_it.md) § PB2);
every call site of every method is one of its own tests
([`api/001`](api/001_six_methods_and_no_caller.md) § PB10). The two crates that
would have been its consumers each declined on opposite grounds — `ring_spsc`
because a single producer collapses the problem, `ring_mpsc` because per-slot
stamps answer it differently — and this crate's own module documentation named
`ring_mpsc` as the reason before that crate existed
([`integration/002`](integration/002_the_two_crates_that_declined.md) § PB4,
[`pitfall/002`](pitfall/002_conflating_the_two_cursors.md) § PB38). An open
question set the escalation condition explicitly: `ring_core` would be the next possible
adopter. `ring_core` has since been implemented, does not adopt, and the follow-up
decision that question required is unwritten
([`integration/002`](integration/002_the_two_crates_that_declined.md) § PB5). Two
neighbours nonetheless changed shape *for* it — `ring_barrier` opened a signature,
`ring_atomic` built a `loom` seam — for a test living in a crate neither depends
on ([`integration/001`](integration/001_ten_crates_name_it_and_none_depends_on_it.md) § PB2,
[`workaround/001`](workaround/001_the_loom_seam_and_its_only_user.md)).

**Correctness held by what is absent.** The frontier moves at one site, from its
exact current value, forward only — and nothing type-level enforces any of the
three. `SeqCell` offers four methods, two of which would move the cursor anywhere,
and the field is in scope; the invariant holds because the crate does not call
them, checked by a grep whose comment-stripping filter is load-bearing
([`invariant/001`](invariant/001_the_frontier_moves_only_by_compare_exchange.md) § PB19,
[`decisions/001`](decisions/001_refused_rather_than_reordered.md) § PB17). A
`store`-based `publish` would pass every behavioural test in the suite under one
producer and most of them under several. The same shape recurs outward:
`cursor()` hands out a `&PaddedCursor` that still exposes `store` and `fetch_add`,
there is no read-only cursor type anywhere in the family, and six accessors across
five crates have the same hole
([`item/001`](item/001_the_three_readings_of_the_cursor.md) § PB22); `publish`
takes a `start` and a `len` where `ring_claim::Claim` is exactly those two values,
carrying a `must_use` that names this crate's failure from the other side, and is
not on the signature
([`pitfall/001`](pitfall/001_publishing_a_range_you_never_claimed.md) § PB35).

**A transition that leaves no trace.** Of the handshake's four operations, the one
this crate owns is the only one whose transition changes no shared state — states
2 and 3, *claimed-unwritten* and *claimed-written*, are indistinguishable
everywhere in the tiered stack
([`lifecycle/001`](lifecycle/001_a_slot_from_claim_to_visibility.md) § PB25). So
the ordering it establishes cannot be checked by looking at cursors, and the loom
model builds its own instrument out of a bare `AtomicUsize` with a `0xABC`
sentinel chosen against two specific failures
([`lifecycle/001`](lifecycle/001_a_slot_from_claim_to_visibility.md) § PB26). The
same invisibility runs through the costs: the family's benchmark crate has five
candidates and nine path dependencies and reaches none of this code, and there is
no `benches/` directory in any of the 33
([`non_functional_requirement/001`](non_functional_requirement/001_what_a_publication_costs.md) § PB29),
so every figure in this corpus is a count read off the source. The one cost that
is not a count is the spin's, and it is a function of three terms — a peer's
payload write, CAS latency under contention, and how many predecessors are ahead —
none of which is a parameter, a field, or an argument
([`non_functional_requirement/002`](non_functional_requirement/002_what_the_spin_costs.md) § PB31).
What *is* pinned is the criterion: three clauses, four annotations naming them by
number at the assertion site, and this is the only crate in the family that pins
all three of its own
([`lifecycle/002`](lifecycle/002_the_four_operation_handshake.md) § PB27).
