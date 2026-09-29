# lifecycle

The same story twice: once as the states one sequence passes through, once as the
four operations that move it between them. Both arrive at the same place — the
transition this crate exists to protect is the one that changes no shared state
at all, so it can only be observed by an instrument built for the purpose.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Slot From Claim to Visibility](001_a_slot_from_claim_to_visibility.md) | PB25, PB26 — the five states, the two that share a predicate, the four crates checked for per-slot state, and the `AtomicUsize` the loom model builds because none of them supplies one |
| 002 | [The Four-Operation Handshake](002_the_four_operation_handshake.md) | PB27, PB28 — the acceptance criterion's three clauses, the four annotations that pin them by number, the ten tests scored clause by clause, and the one clause actually checked under concurrency |

### Five States, Four Transitions

```
      1 ─── claim ──▶ 2 ─── write ──▶ 3 ─── publish ──▶ 4 ─── commit ──▶ 5
          claimed++       (nothing)       published++      position++
         ring_claim                      ring_publish     ring_consume
```

| # | State | Predicate | Who may touch the slot |
|--:|-------|-----------|------------------------|
| 1 | Unclaimed | `seq >= claimed` | nobody |
| 2 | Claimed, unwritten | `published <= seq < claimed` | its producer, exclusively |
| 3 | Claimed, written | `published <= seq < claimed` | its producer, exclusively |
| 4 | Published | `consumer_position <= seq < published` | consumers, read-only |
| 5 | Committed | `seq < consumer_position` | its producer again, after a lap |

States 2 and 3 share a predicate because nothing in the tiered stack distinguishes
them — `ring_store` stores no per-slot flag and `ring_slot`'s `Option` is
`&mut`-gated, so no concurrent reader can consult it. The published cursor is a
*single* boundary between "definitely written" and "possibly not", which is
exactly why the 2 → 3 transition does not need to be observable: it only needs to
happen-before 3 → 4, and that is what `Release`/`Acquire` buys.

### Four Operations, Three Clauses

| # | Operation | Crate | Moves |
|--:|-----------|-------|-------|
| 1 | `Claimer::claim( n )` | `ring_claim` | the claimed cursor |
| 2 | `Publisher::publish( start, len )` | **`ring_publish`** | **the published cursor** |
| 3 | `Consumer::available()` | `ring_consume` | nothing — a read |
| 4 | `Consumer::commit( to )` | `ring_consume` | the consumer position |

| Clause | Direction | Pinned at | Under concurrency |
|-------:|-----------|-----------|:-----------------:|
| 1 — nothing appears too early | safety | `:133` (loom), `:333` (threaded) | **exhaustively** |
| 2 — everything published does appear | liveness | `:153` (loom) | no — after both joins |
| 3 — commit never passes `available()` | safety | `:143` (loom) | yes |

Clause 1 without clause 2 is satisfied by an `available()` that always returns
nothing; clause 2 without clause 1 by one that always returns everything. The pair
is what makes either meaningful, and clause 3 closes the loop back to the producer.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# which crates can distinguish written from unwritten, per slot
grep -nE '^\s*pub (fn|struct|enum|const fn)' ring_slot/src/lib.rs
grep -nE '^\s*pub (fn|struct|enum|const fn)' ring_store/src/lib.rs
grep -rlnE 'stamps|slot_state|per-slot' ring_*/src/*.rs

# the instrument the loom model builds instead
awk '/^  \/\/\/ What the producer writes into the slot\. Any value the slot cannot hold by$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 2 { print } /^      \/\/ The slot itself, so that "did the consumer read something the producer$/{ n2 = NR } n2 && NR >= n2 && NR <= n2 + 3 { print } /^          \/\/ Between here and the publish below, the slot is claimed and the$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 3 { print } /^          assert_eq!\( run\.start\(\), Seq::ZERO \);$/{ n4 = NR } n4 && NR >= n4 + 1 && NR <= n4 + 5 { print }' ring_publish/tests/handshake_test.rs

# every crate that pins a criterion clause by number, at the assertion site
grep -rn 'Clause' --include='*.rs' */ 

# the criterion itself
command grep -m1 -F '| 170 | Claim / publish / available / commit handshake | `ring_claim` `ring_publish` `ring_consume` | S4 | A slot claimed but not published is never returned by `available()`; after publish it is; `commit()` advances the consumer cursor and never past `available()`. Asserted over every interleaving of one claim and one drain under `loom` | `ring_publish/tests/handshake_test.rs` |' bench_harness/docs/acceptance/001_feature_reached_tests.md
```

| | Value |
|--|------:|
| States a sequence passes through | 5 |
| Transitions between them | 4 |
| …that move a cursor | 3 |
| …that move nothing | **1** — and it is the one this crate exists for |
| States sharing a predicate | 2 and 3 |
| States externally indistinguishable | 3 — all answer `false` to `is_published` |
| Crates in the tiered stack with per-slot state | **0** |
| Crates in the family with per-slot stamps | 2 — both declined this crate |
| Slot types the loom model imports | **0** — it builds an `AtomicUsize` |
| The written sentinel | `0xABC`, chosen against `0` and `1` |
| Producer's publication window, in statements | 2 |
| Operations in the handshake | 4 |
| Crates owning them | 3 |
| Crates in the wiring | 6 |
| Clauses in the criterion | 3 |
| …pinned by number at an assertion site | **4 annotations, all 3 clauses** |
| Crates family-wide that pin clauses this way | 3, and only one pins all of its own |
| Tests in the file | 10 — 2 loom, 8 threaded |
| …that check the gate rather than a clause | 5 |
| …that touch no clause at all | 3 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB25 | family | n/a — observation | Of the four transitions, the one this crate exists to protect is the only one that changes no shared state; states 2 and 3 are genuinely indistinguishable within the tiered stack, and the two crates that *can* tell them apart per slot are the two that built stamps instead of using this crate |
| PB26 | `ring_publish` | n/a — observation | So the loom model constructs its own observation instrument out of a bare `AtomicUsize`, with a `0xABC` sentinel chosen against two specific failures: `0` is indistinguishable from never-written, and `1` could collide with an initial value |
| PB27 | family | n/a — observation | Three files family-wide mention a numbered clause, and this is the only crate that pins all three of its criterion's clauses to specific assertions by number, at the assertion site; nothing mechanical checks that a clause has an assertion |
| PB28 | `ring_publish` | n/a — coverage | The criterion's *"asserted over every interleaving"* reads as covering all three clauses; the model checks clause 1 exhaustively, clause 3 at every point the drain could reach it, and clause 2 once per execution at a quiescent point |
