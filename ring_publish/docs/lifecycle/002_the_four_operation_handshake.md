# Lifecycle: The Four-Operation Handshake

### Scope

- **Purpose**: Record the claim → publish → available → commit protocol as this crate hosts it: the operations, the crates that own them, the acceptance criterion's three clauses, and which assertion pins each.
- **Responsibility**: State the criterion verbatim, split it into clauses, map every clause onto the code that checks it, and record which clauses are checked under concurrency and which are not.
- **In Scope**: The protocol as `tests/handshake_test.rs` assembles it, and this crate's place in it.
- **Out of Scope**: One sequence's states — see [`lifecycle/001`](001_a_slot_from_claim_to_visibility.md).

### The Four Operations

| # | Operation | Signature | Crate | Moves |
|--:|-----------|-----------|-------|-------|
| 1 | `Claimer::claim( n )` | `-> Result< Claim, RingError >` | `ring_claim` | the claimed cursor |
| 2 | `Publisher::publish( start, len )` | `-> Seq` | **`ring_publish`** | **the published cursor** |
| 3 | `Consumer::available()` | `-> Run` | `ring_consume` | nothing — a read |
| 4 | `Consumer::commit( to )` | `-> Result< Seq, RingError >` | `ring_consume` | the consumer position |

Three crates, four operations, and a fourth crate — `ring_barrier` — supplying
the thing operation 3 reads through. `ring_gating` holds the cursor operation 4
moves and operation 1 gates against. Six crates in the wiring; three named by the
feature.

The protocol keeps two separations, each buying one guarantee: claiming is
separate from publishing so a half-written slot is never visible, and
availability is separate from commitment so the producer learns how far it may
safely advance.

Two separations, each buying one guarantee. The first is this crate's; the second
is `ring_consume`'s.

### The Criterion, and Its Three Clauses

`bench_harness/docs/acceptance/001_feature_reached_tests.md:38`, verbatim:

> A slot claimed but not published is never returned by `available()`; after
> publish it is; `commit()` advances the consumer cursor and never past
> `available()`. Asserted over every interleaving of one claim and one drain
> under `loom`

| Clause | Text | Direction |
|-------:|------|-----------|
| 1 | *a slot claimed but not published is never returned by `available()`* | safety — nothing appears too early |
| 2 | *after publish it is* | liveness — everything published does appear |
| 3 | *`commit()` advances the consumer cursor and never past `available()`* | safety — the consumer cannot over-report |

Clause 1 without clause 2 is satisfied by an `available()` that always returns
nothing; clause 2 without clause 1 is satisfied by one that always returns
everything. The pair is what makes either meaningful, and clause 3 closes the
loop back to the producer, since the consumer position is what operation 1 gates
against.

### PB27 — Every Clause Is Pinned By Number, In the Code

```sh
cd "$(git rev-parse --show-toplevel)"
# all four roots named explicitly: `ring/` holds this crate and both its
# numbered-clause siblings, and `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/` is a repo-root sibling of `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`
# rather than nested inside it, so neither is reachable from a bare `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`
grep -r 'Clause' --include='*.rs' /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | LC_ALL=C sort
```

Live output:

```
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path/tests/judge_test.rs:/// Clause 1: the driven arm must deliver exactly the staged batch, in order. Dropping one record
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path/tests/judge_test.rs:/// Clause 2: exactly one flush. Two entries would mean the batch moved in pieces — the records
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path/tests/judge_test.rs:/// Clause 3: the driven flush must leave nothing behind. `judge_flush` already read the count the
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path/tests/judge_test.rs:/// Clause 4 is T75 itself: a control arm that never called `drive` must see nothing. A record
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path/tests/judge_test.rs:/// Clause 5: the records the control arm did not deliver must still be staged. Without it a
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path/tests/judge_test.rs:/// Clause 6, and the field that replaced `ArmReport::drive_called`: an arm that never drove cannot
ring_publish/tests/handshake_test.rs:          // Clause 1, made observable: `available` offered this slot, so the
ring_publish/tests/handshake_test.rs:          // Clause 3: commit takes exactly what was offered, and lands there.
ring_publish/tests/handshake_test.rs:      // Clause 2, deterministically now that the producer is done: what was
ring_publish/tests/handshake_test.rs:    // Clause 1, single-threaded and exact: the claim exists, the slot may even
ring_registry/tests/registry_test.rs://! | Clause | Test |
ring_testkit/tests/testkit_test.rs:/// **Clause 1 — a scripted sequence reproduces identical outcomes every run.**
ring_testkit/tests/testkit_test.rs:/// **Clause 1, the part a single re-run cannot show.** Ten runs, all equal.
ring_testkit/tests/testkit_test.rs:/// **Clause 2 — the fixture decides something a count cannot.**
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_asteroids/tests/judge_test.rs:/// Clause order: all three real-arm clauses before any control-arm clause, and the split before
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_orbit_ship/tests/judge_test.rs:/// Clause order: envelope, then spread, then control arm. All three wrong reports the envelope —
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_spatial_wrap/src/lane.rs:/// Clauses 1 and 2 alone were the whole reached-test until a probe ran them against deliberately
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_spatial_wrap/src/lane.rs:/// So the pair could not distinguish a correct wrap from returning nothing at all. Clause 3 grades
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_spatial_wrap/tests/lane_test.rs:/// and one folding the x and z axes into each other. Clause 3 — agreement with an independently
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_c.rs:  // Clause 1 — the crate's own module doc still says it is unimplemented.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_c.rs:  // Clause 2 — the crate publishes at least one item, by band E's own producer.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_c.rs:  // Clause 3 — every populated mapped definition has a matching test surface.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_c.rs:  // Clause 4a — no spec still marks itself pending.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_c.rs:  // Clause 4b — every declared case id is cited by a test.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_d.rs:    /// Clause key, as the lane prints it.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_d.rs:    /// Clause key, as the lane prints it.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_d.rs:    /// Clause key, as the lane prints it.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_d.rs://! | Lane says | Clauses satisfy the cell | Band D reports | Why |
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_d.rs:fn nonempty( found : &[ Clause ], key : &str ) -> Option< String >
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_d.rs:fn unmet( requirement : &Requirement, found : &[ Clause ], tag : &str ) -> Option< String >
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/band_d.rs:use crate::lane_line::{ Clause, LaneLine, clause_number, clause_value, clauses, parse_lane_line };
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/lane_line.rs:    found.push( Clause { key : key.to_string(), value : value.to_string() } );
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/lane_line.rs:pub fn clause_number( found : &[ Clause ], key : &str ) -> Option< u64 >
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/lane_line.rs:pub fn clause_value< 'a >( found : &'a [ Clause ], key : &str ) -> Option< &'a str >
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/lane_line.rs:pub fn clauses( line : &str ) -> Vec< Clause >
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/src/lane_line.rs:pub struct Clause
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 1: while the crate's own module doc says it is unimplemented, that sentence is the
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 2's other half: no `src/` at all is a different condition from a `src/` that publishes
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 2, and the reason clause 1 is not enough on its own: deleting the skeleton sentence is a
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 3's exemptions, in the corpus's own practice: a pitfall is a warning and a feature is a
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 3's other boundary: a definition directory holding only its own index is scaffolding, not
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 3's surface reader must not count a hyphen-prefixed scratch file as a real mirror. Such
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 3: a populated doc definition with no test surface is the corpus describing behaviour
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 4a: a spec marked pending is the corpus itself saying the case is unimplemented. Nothing
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 4b's prefix guard, live rather than in the unit self-test. Without it a crate declaring
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 4b's prefix guard, the boundary its sibling above does not cover: a shorter id must not
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 4b's source reader must not count a hyphen-prefixed scratch probe as a real citation. A
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/band_c_test.rs:/// Clause 4b, the half a status glyph cannot cover: a case can be declared, marked done, and cited
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/lane_line_test.rs:  assert_eq!( found, vec![ Clause { key : "detail".to_string(), value : "a=b".to_string() } ] );
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/lane_line_test.rs:/// Clauses are read out of a real lane line, and the prose around them is not mistaken for one.
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ws001_gate/tests/lane_line_test.rs:use ws001_gate::lane_line::{ self, Clause, clause_number, clause_value, clauses, parse_lane_line };
```

`ws001_gate` (added to the corpus after this doc was written) has grown since: the `Clause` type
moved out of `band_b.rs`/`band_b_test.rs` into its own `lane_line.rs`/`lane_line_test.rs`, and two
new bands, `band_c.rs` and `band_d.rs`, joined them — 32 lines here now, not 8. Of the two new
bands, one — `band_c.rs`, with `band_c_test.rs` — does carry a genuinely numbered clause: the four
checks band C grades a crate's "skeleton" status against. Those number the gate's own invented
grading criteria for that band, never a feature's `docs/acceptance/`-sourced reached-test
criterion — what "numbered clause" means in the paragraph below — so that count is still
unaffected.

Family-wide, three files mention a numbered clause: `ring_testkit`
(clauses 1 and 2 of its own criterion), `ring_registry` (a `Clause | Test`
table), and this one. **`ring_publish` is the only crate that pins all three
clauses of its criterion to specific assertions, by number, at the assertion
site.**

| Comment | Line | Harness | Clause | What it guards |
|---------|-----:|---------|-------:|----------------|
| *"Clause 1, made observable"* | `133` | loom | 1 | `available` offered it ⇒ the write already happened |
| *"Clause 3: commit takes exactly what was offered, and lands there"* | `143` | loom | 3 | `commit` returns and reaches `run.end()` |
| *"Clause 2, deterministically now that the producer is done"* | `153` | loom | 2 | one claim published ⇒ one slot available |
| *"Clause 1, single-threaded and exact"* | `333` | threaded | 1 | two claims, neither published, nothing visible |

Clause 1 is pinned twice — once in each harness — and the two annotations
describe different jobs. The loom one says *"made observable"*, because the
sequence numbers alone cannot show it and the model builds an instrument
([`lifecycle/001`](001_a_slot_from_claim_to_visibility.md) § PB26). The threaded
one says *"single-threaded and exact"*, because with no concurrency the claim,
the write and the absence of visibility can simply be asserted in sequence.

The annotations are worth recording because they are the only thing tying the
criterion's prose to the code. Nothing mechanical checks that a clause has an
assertion; the numbering is a convention this file follows and `ring_testkit`
partially follows, and the other twenty features do not use at all.

### PB28 — Only One of the Three Clauses Is Actually Checked Under Concurrency

The criterion's last sentence — *"asserted over every interleaving of one claim
and one drain under `loom`"* — reads as covering all three clauses. The model
does not do that, and says so.

| Clause | Loom assertion | Where it runs | Interleaving-sensitive |
|-------:|----------------|---------------|:----------------------:|
| 1 | `slot.load() == WRITTEN` (`:137-141`) | inside the drain thread, after `available()` returned a run | **yes** |
| 3 | `reached == run.end()`, `position() == run.end()` (`:144-146`) | inside the drain thread | yes, but its failure mode is not ordering |
| 2 | `published() == Seq( 1 )`, then a fresh consumer sees 1 (`:155-158`) | **after both `join()` calls** | **no** |

`tests/handshake_test.rs:153-154` states the placement outright:

> Clause 2, deterministically now that the producer is done: what was published
> is available. A consumer starting fresh sees exactly one slot.

That is the correct design and not a gap. Clause 2 is a liveness property, and
liveness cannot be asserted at an arbitrary interleaving point — a drain that
looks before the producer publishes legitimately sees nothing, which is why the
drain's own path has an early return at `:128-131`:

```rust
if run.is_empty()
{
  return;
}
```

So the model's honest coverage is: clause 1 exhaustively, clause 3 at every point
the drain could reach it, clause 2 once per execution at a quiescent point. The
drain's comment at `:122-124` gives the matching argument for why one look
beats a loop:

> One look, not a loop. Every point at which the look could land is a separate
> execution loom already runs, so spinning here would only add unbounded
> executions without adding a single new observation.

The clause that most needs exhaustive treatment gets it. Recording the
distribution matters because the criterion's wording does not distinguish the
three, and a reader could reasonably assume all three carry the same weight of
evidence.

### The Ten Tests, By Clause

| Test | Line | Harness | 1 | 2 | 3 | Gate |
|------|-----:|---------|:-:|:-:|:-:|:----:|
| `a_claimed_slot_is_invisible_until_published_over_every_interleaving` | `78` | loom | ✔ | ✔ | ✔ | — |
| `a_full_ring_stops_the_producer_over_every_interleaving` | `168` | loom | — | — | ✔ | ✔ |
| `the_four_operations_carry_every_item_across_in_order` | `274` | threaded | ✔ | ✔ | ✔ | ✔ |
| `a_claim_that_is_never_published_stops_the_consumer_at_it` | `331` | threaded | ✔ | ✔ | — | — |
| `publication_out_of_order_is_refused_rather_than_advancing_past_a_gap` | `357` | threaded | ✔ | ✔ | — | — |
| `the_consumer_position_the_producer_gates_on_is_the_one_commit_moves` | `384` | threaded | — | — | ✔ | ✔ |
| `a_stalled_consumer_stops_the_producer_after_exactly_one_lap` | `410` | threaded | — | — | — | ✔ |
| `a_slow_consumer_and_a_fast_producer_never_lose_or_duplicate_an_item` | `435` | threaded | ✔ | ✔ | ✔ | ✔ |
| `several_producers_and_one_drain_agree_on_every_sequence` | `498` | threaded | ✔ | ✔ | ✔ | ✔ |
| `a_consumer_that_never_commits_leaves_the_producer_exactly_one_lap_ahead` | `564` | threaded | — | — | — | ✔ |

The **Gate** column is the protocol's unstated fourth requirement — the producer
must not lap the consumer — which belongs to `ring_barrier`/`ring_gating`'s own
gating mechanism and is checked here anyway because the four operations cannot be
exercised end to end without it. Five of the ten tests are gate tests; three
touch no clause at all and exist purely to prove the wiring is load-bearing.

`a_claim_that_is_never_published_stops_the_consumer_at_it` (`:331-354`) is the
cleanest statement of clause 1 in the file, because it needs no threads:

```rust
let first = claimer.claim( 3 ).unwrap();
assert!( consumer.available().is_empty(), "a claim alone published nothing" );

let second = claimer.claim( 2 ).unwrap();
assert!( consumer.available().is_empty(), "two claims alone still published nothing" );

publisher.publish( first.start(), first.len() );
assert_eq!( consumer.available().len(), 3, "only the published claim" );
```

Five claimed sequences, three published, three available. Clause 1 and clause 2
in six lines.

### Why the Test Lives Here

`tests/handshake_test.rs:11-15` argues the placement, and it is the only reason
this crate hosts a test naming four dependencies it does not depend on:

> It lives in `ring_publish` rather than in any of the other three crates because
> publication is the moment the other three become observable together: before
> it, a claim is invisible; after it, the consumer's whole contract is decided.

The argument holds. A claim, on its own, changes nothing any other crate can see
— `ring_claim`'s cursor is read by nobody
([`data_structure/002`](../data_structure/002_the_four_cursors_of_the_handshake.md)).
`available` and `commit`, on their own, have nothing to report. Publication is
the only one of the four operations whose effect is visible to a crate that did
not perform it.

The consequence is recorded in
[`integration/001`](../integration/001_ten_crates_name_it_and_none_depends_on_it.md)
§ PB3: the crate with no dependents at all carries the reached-test for the
family's central protocol, through four dev-dependencies that exist for this
file alone.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_four_cursors_of_the_handshake.md](../data_structure/002_the_four_cursors_of_the_handshake.md) | The cursors these four operations move |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_ten_crates_name_it_and_none_depends_on_it.md](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) | The four dev-dependencies this file alone needs |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_is_published_is_exclusive_of_the_frontier.md](../invariant/002_is_published_is_exclusive_of_the_frontier.md) | Clause 1 and clause 2, as one comparison |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_a_slot_from_claim_to_visibility.md](001_a_slot_from_claim_to_visibility.md) | The same protocol from one sequence's point of view |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_the_spin_costs.md](../non_functional_requirement/002_what_the_spin_costs.md) | What the largest of these tests measures, and what it does not |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | The two harnesses, and why both |
| [../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md](../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md) | How six crates wire together without a cycle |

### Sources

| File | Relationship |
|------|--------------|
| `bench_harness/docs/acceptance/001_feature_reached_tests.md:38` | The criterion, and the file named as its reached-test |
| `ring_publish/src/lib.rs:18-27` | Operation 2's reason for being distinct from operation 1 |
| `ring_claim/src/lib.rs:336-341` | Operation 1, and the same requirement from its side |
| `ring_consume/src/lib.rs:12-19` | Why operations 3 and 4 are separate calls |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handshake_test.rs:1-50` | The placement argument, the topology, and the two-harness rationale |
| `tests/handshake_test.rs:133,143,153,333` | The four clause annotations |
| `tests/handshake_test.rs:331-354` | Clauses 1 and 2 with no threads at all |
| `tests/handshake_test.rs:274-328` | All three clauses plus the gate, at 20 000 items through a 16-slot ring |
| `tests/handshake_test.rs:434-495` | 8 000 items through 4 slots — 2 000 laps, gate-bound throughout |
| `tests/manual/readme.md § P6` | The two harnesses wired and mutually exclusive |
