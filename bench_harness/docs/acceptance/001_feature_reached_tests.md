# Feature Reached-Tests

The binary condition each of the 22 graded features must satisfy to count as
delivered, the crate(s) that own it, and the test that claims it.

Authored after resolving seven contract gaps that made these unauthorable: no
acceptance criteria existed anywhere in the corpus, no feature named a crate, no
crate cited a feature, and five outright contradictions stood between the feature
text and the manifests.

### Scope

- **Purpose**: Give each of the 22 graded features one binary reached-condition, so "delivered" is something a command answers rather than something a reader judges.
- **Responsibility**: The claiming rule, the 22 conditions with their owning crates and claiming tests, and the per-crate coverage roll-up.
- **In Scope**: The 22 graded features and the tests under `ring_*/tests/` that claim them.
- **Out of Scope**: Whether a gate can distinguish pass from fail at all (→ [`invariant/001`](../invariant/001_gate_non_vacuity.md)); the resolved contract gaps that made these unauthorable before this table could be written.

### How a feature is claimed

A feature is claimed by a test file under `ring_*/tests/` that cites
`docs/feature/NNN_` in its own text. Gate `g3_features.sh` greps for exactly that
citation across all 33 crates and reports how many of the 22 are claimed. The
citation is the crate→feature edge; this table is the feature→crate edge. Nothing
else records the mapping, deliberately — a crate claims a feature by testing it,
never by asserting it in prose.

**A feature claimed by more than one file has all of them named in the Claiming
test column — the count that governs is files, not owning crates.** An earlier
form of this rule keyed it to the crate count ("a feature owned by more than one
crate is claimed by more than one file"), which is true and too narrow to catch
its own next instance: rows 179 and 188 each have exactly one owning crate and two
claiming files, and each named one (→ below). A rule stated about crates and
enforced against files is the same shape as the criteria this table grades — the
thing asserted and the thing checked are one step apart, and the gap is where the
omission lives. `g3_features.sh` cannot enforce either form: it
greps `docs/feature/0*NNN_` across all 33 crates and reports whether *any* file
cites the feature, so one citation satisfies it however many crates the row lists.
That makes the column the only record of the full set, and an incomplete cell is
invisible to the gate that exists to check the mapping. Rows 173 and 174 were both
incomplete for this reason — each names two owning crates, each is claimed by a
test in both, and each named only the handler-side file while its own Reached-when
text leads with the `ring_types` discriminant clause. The two crates' test files
say so themselves: `ring_types/tests/types_test.rs` calls the handlers
"`ring_wait`'s half" and "`ring_overflow`'s half", and
`ring_overflow/tests/overflow_test.rs` calls itself "the handler half" and points
at `ring_types` for the enum half.

**The two rows were incomplete the same way, and 173's own crates had already
written down why the single name was wrong.** For 174 the halves partition: each
file names its own and disclaims the other's, so the table had recorded one of two
files that both know the split. For 173 they overlap, deliberately.
`ring_types/tests/types_test.rs:9-10` claims "the four `WaitKind` discriminants"
and assigns "the handlers" to `ring_wait`; `ring_wait/tests/wait_test.rs:3-9` then
restates all three clauses as its own reached-test — and `:11-19` explains, under
its own heading, that this is not duplication: `ring_types` asserts the enum's
shape "as a fact about that crate", this file asserts it "as a *precondition of the
second clause*", because one-handler-per-discriminant is meaningless without
knowing how many there are. "The two assertions would survive each other's deletion
and mean different things."

So 173's clause 1 is asserted in two crates by design, with the reason recorded in
prose — and this table, the one artifact whose job is the feature→crate edge, named
one of them. The rationale survived in the crate that wrote it; only the mapping
was lost, in the document that exists to hold the mapping.

**This screen first flagged eight of the 22 rows as listing more owning crates than
claiming tests** — 167, 169, 170, 172, 180, 182, 183, 185. Reading them one at a
time corrected four (167, 180, 182 and 185, → below) and cleared three (169, 170,
172). The eighth, 183, keeps the mismatch on purpose: its co-owner has no test
citing it, for a reason that crate wrote down. **Four rows therefore still list more
owners than named files — 169, 170, 172 and 183 — and all four are correct as they
stand.** A count alone could not have told which, because the reverse count is far
more common — 17 of 22 rows have more files citing them than the row names — and a
file can cite a feature it does not claim: a header naming a neighbouring feature
for contrast cites it exactly as a claim does. Row 170 is cited by six files and
names one; 178 by six and names two. Telling a claim from a mention needs each file
read, not counted, which is the same distinction between mentioning and doing that
this corpus keeps re-finding.

Row 183 looks like the worst case by the counts and is the best: two owning
crates, one named test, and exactly **one** file citing it in the whole family.
`ring_handle` has no test citing 183 at all — deliberately, and its co-owner says
why. `ring_poll/tests/poll_test.rs:15-17`: "The feature also constrains
`ring_handle` without being claimed by it — nothing reachable from a handle may
park. That constraint is enforced here, by name, because `ring_handle`'s own suite
would stay green if it broke." A count cannot tell that apart from an omission,
which is the whole reason each of the eight had to be read rather than counted.

**But the same row's Reached-when text was wrong, and the count that looked wrong
is what led to reading it.** It required "a `trybuild` compile-fail case proving
the blocking variants are not in scope". No such fixture exists — a
`grep -rlniE "park|block|sleep|wait" ring_*/tests/ui/` over every trybuild
fixture in the family returns nothing — and `poll_test.rs:21-25` explains that one
cannot: a parking call "would not fail to compile; it would fail to *resolve*,
because the crate is not a dependency — and a test cannot name a crate it cannot
see." The criterion had been written in the shape of row 179's, which does use
`trybuild` and has the seven fixtures to back it. Corrected above to the three
assertions that actually carry the clause: the `PARKING_CRATES` manifest scan, the
dependency-closure walk that pins what the scan is blind to, and the bounded-time
spin test. **`g3_features.sh` reported this row REACHED throughout** — it greps for
the citation and never reads the Reached-when text, so a criterion naming a
mechanism that cannot exist is invisible to it.

**Rows 179 and 188 are 183's mirror image: their criteria name mechanisms that do
exist, in files this table did not name.** Both have exactly one owning crate, so
the crate-keyed form of the claiming rule could not see them, and both split their
criterion's two clauses across two files in that one crate — the split being the
point in each case, not an accident of layout.

For 179, the `trybuild` clause lives entirely in `ring_handle/tests/ui_test.rs`,
which opens "Feature 179's compile-fail suite" (`:1`) and explains at `:12-15` why
it is a separate binary at all: "`trybuild` spawns a nested `cargo` build. Keeping
it out of `tests/handle_test.rs` means the fast suite stays fast." `handle_test.rs`
carries the other clause, `Send` and the two-thread move, and contains no
`trybuild` at all. The file even knows about this table — `:41-42` calls its first
two cases "the two the acceptance table names", against five it added itself. The
table named the criterion's cases and not the file that runs them.

For 188 the omission is sharper, because the named file is the one that cannot
carry the named clause. `testkit_test.rs` is `#![ cfg( not( loom ) ) ]` (`:27`);
the loom half is `exhaustive_test.rs`, which is `#![ cfg( loom ) ]` (`:30`), opens
by naming feature 188 as "the exhaustive half" (`:1`), and carries a section headed
**`# The declared bound`** (`:22`) — the exact phrase the criterion grades against
— declaring it at `:24-25` as one producer, one consumer, two slots, one push.
Under `RUSTFLAGS="--cfg loom"`, the run the first clause describes, the file this
table named compiles to nothing (`exhaustive_test.rs:18-19`). The two are not
redundant and say so: "Neither subsumes the other — one has scale without coverage,
the other coverage without scale" (`:7-9`).

Both omissions were invisible to `g3_features.sh` for the ordinary reason — each
pair's other file cites the feature too, so the grep was satisfied either way. What
makes them worth recording next to 183 is the direction: 183 graded a clause against
a mechanism no file has, 179 and 188 graded clauses against mechanisms whose only
file the table omitted. A criterion and its evidence can drift apart from either
side, and the gate reads neither.

**Running the corrected rule against the whole table then found four more: 167,
180, 182 and 185.** All four were reachable by the *narrower* crate-keyed form as
well — each lists two or three owning crates and named one file — which is the
uncomfortable part of the finding. The rule was not too weak to catch them; it had
simply never been run against the table it was written for. Stating a rule and
applying it are two acts, and only the first had happened.

Owner-count exceeding named-file-count is a screen, not a verdict: besides 183,
already adjudicated above, it flags seven rows — and three of them are correct as
they stand. What separates the two groups is
what the unnamed sibling says about itself. **A correct single-claimer's siblings
point at the named file; an omission's siblings claim a clause of their own.**

Row 170 is the clean case, twice over. `ring_claim/tests/claim_test.rs` says its
feature-170 content is the claim half and that "its full reached-test lives in
`ring_publish/tests/handshake_test.rs`, because the handshake is only observable
once publishing exists"; `ring_consume/tests/consume_test.rs` says the same in its
own words. Two siblings, both deferring to the named file, for a stated structural
reason. Row 169 clears on the same test — `ring_align/tests/align_test.rs` asserts
`CacheAligned<T>`, not the `PaddedCursor` the criterion names, and says so: "this
crate owns only the constant and the wrapper". Row 172 clears differently: the
named `mpsc_test.rs` does assert the exclusivity clause itself, at `:219`, so the
sibling's direct assertion is reinforcement rather than the only copy.

The four that fail say the opposite. `ring_seqno/tests/seq_test.rs` opens "Claims the
never-wraps half" of 167; `ring_types/tests/types_test.rs` claims "the types and the
capacity constraint", assigning the mapping to `ring_index` — three clauses, three
crates, one named. `ring_config/tests/config_test.rs` restates 180's field list as
its own acceptance criterion and hands the factory half to `ring_factory` — though
its restatement swaps this table's "is the only constructor input" for "each field
is *observable* one at a time", so even the two copies of one criterion have drifted
apart, and the only-constructor-input clause is asserted by neither wording. For
185 the criterion
already names its subjects out loud: `ring_debug`'s check and `ring_trace`'s entry
count are clauses two and three, `ring_debug/tests/debug_test.rs` calls its clause
"one of the three that feature carries", and `ring_trace/tests/trace_test.rs` cites
*this document by path* while claiming a clause this document did not credit it
with.

Row 182 is the sharpest of the eight omissions this document has now recorded —
173 and 174, then 179 and 188, then these four — sharper even than 188.
`ring_event/tests/event_test.rs` does not merely claim a clause — it states that the
named file cannot: "`ring_slot/tests/slot_test.rs` already claims the byte-identity
halves against each shape directly. What it cannot claim, because it holds no such
thing, is *identical path* — two shapes each round-tripping through their own code
is the reading that criterion is written to exclude." The criterion's central word
was chosen to rule out exactly what the named test proves, and the file that proves
the other reading went uncredited.

None of this is visible to `g3_features.sh`, for the same reason as before: every
sibling cites its feature, so one `grep` hit satisfies the gate however many files
the clause actually needs.

**Row 187 is 183's shape a second time — and here the dead clause was covering a
live one.** Its criterion ended "asserted by running one suite under both feature
configurations", a run nothing in this repo performs. `g1_coverage.sh:62` and every
`will .test` level pass `--all-features`; `g2_docs.sh:75` reaches the other
configuration with `cargo check --all-targets --no-default-features`, which
compiles those tests and never runs them. Nor is that an oversight:
`ring_core/docs/pitfall/002_feature_gated_code_reads_as_uncovered.md` records
compile-not-run as the deliberate mitigation and names `tests/manual/readme.md`
C3 as the standing check. The criterion graded the crate against a mechanism the
crate had already decided against.

Running both configurations by hand — the first time that had been done — turned
the dead clause into a live finding. The default build ran 19 tests and
`--all-features` 22, and one of the three extras was
`every_backend_reports_the_capacity_it_was_configured_with`, whose body loops over
`[ Spsc, Mpsc, Crossbeam ]` and whose own doc comment calls it "what holds the
three backends to one answer rather than a comment claiming they agree". Its
`#[ cfg( feature = "crossbeam" ) ]` sat on the *function*, sized to its widest
match arm rather than to the property it asserts — and `ring_core` declares
`default = []`, so in the build every consumer gets by default, `Ring::capacity()`
was held to that answer for no backend at all. The gate now sits on the crossbeam
arm alone and the loop runs over `every_backend()`; the default build went 19 → 20
and both configurations stay green under `-D warnings`.

The row's own correction is small. The shape is not: a criterion naming a
mechanism nobody runs is not merely inaccurate, it is the kind of inaccuracy that
deters anyone from performing the one check that would expose what sits under it.
183's dead clause was dead the whole way down. 187's was load-bearing.

**Row 184 is the shape a third time, and this time the mechanism does not exist
anywhere in the family.** Its criterion promised that after `close()` "every
parked waiter is woken". Nothing parks and nothing wakes.
`ring_shutdown::wait_for_close` delegates to `ring_wait::wait_until`, a bounded
`for` loop that re-reads a predicate; `ring_wait::pause`'s `Park` arm is a 50 µs
`thread::sleep` whose own in-source comment states that the
publisher-holds-the-waiter's-handle registration a real park would need is a
relationship the crate deliberately does not have. Scanning every `ring_*/src/`
for `Condvar`, `futex`, `unpark`, `thread::park`, `notify_one`, `notify_all` and
`Waker` returns exactly three lines, and all three are that comment. So the
behaviour a waiter actually gets is not a wake: it looks again on its next poll
and sees the flag, and one whose spin budget expires an iteration early gets
`Err( RingError::Empty )` — the case "woken" hides, and the case
`wait_for_close`'s own doctest asserts on its first line.

Unlike 183 and 187, this one had already been found — in another crate's corpus.
`ring_wait`'s WT30 records the same false claim at its real source, the public
doc on `ring_types::WaitKind::Park`, and was dispositioned **declined**. Reading
the disposition rather than the verdict word shows it declined only the *venue*:
its text names `ring_types/src/policy.rs:30` as the owner and gives
not-this-crate's-line-to-change as the reason. A finding filed correctly,
diagnosed correctly and routed correctly still sits open until someone works the
crate it was routed to — which is a third failure mode for this table, distinct
from both 183's and 187's: not a criterion nobody checked, but a defect somebody
had already checked, written up, and correctly declined to fix from where they
stood. That line now reads *"Idle between reads; no publisher wakes it.
Cheapest, highest latency."*, and the row above says what a waiter observes.

Reached is binary. Where a criterion below names a number, the number is the test's
assertion, not a target to approach.

### The 22

| # | Feature | Owning crate(s) | Stage | Reached when | Claiming test |
|---|---------|-----------------|-------|--------------|---------------|
| 167 | Sequence, slot index, power-of-two capacity | `ring_seqno` `ring_index` `ring_types` | S1 | `Seq` is a `u64` newtype that never wraps in-range; `Index::of(seq, cap)` equals `seq % cap` for every `seq` in `0..4*cap` and every power-of-two `cap` in `2..=1024`, and is computed by mask not division; a non-power-of-two capacity is rejected at construction | `ring_seqno/tests/seq_test.rs`, `ring_index/tests/index_test.rs`, `ring_types/tests/types_test.rs` |
| 168 | Ring buffer storage | `ring_store` | S2 | A `Buffer<T>` of capacity `N` allocates exactly `N` slots once, exposes indexed get/set, holds no cursor and no ordering state; two distinct slot indices never alias | `ring_store/tests/buffer_test.rs` |
| 169 | Padded cursor | `ring_cursor` `ring_align` | S3 | `align_of::<PaddedCursor>() == 64` and `size_of::<PaddedCursor>() == 64`; two `PaddedCursor` values in one struct have addresses at least 64 bytes apart | `ring_cursor/tests/cursor_test.rs` |
| 170 | Claim / publish / available / commit handshake | `ring_claim` `ring_publish` `ring_consume` | S4 | A slot claimed but not published is never returned by `available()`; after publish it is; `commit()` advances the consumer cursor and never past `available()`. Asserted over every interleaving of one claim and one drain under `loom` | `ring_publish/tests/handshake_test.rs` |
| 171 | SPSC ring API | `ring_spsc` | S5 | One producer and one consumer exchange 100_000 items with byte-parity between what was written and what was read, in order, with zero loss and no lock in the path | `ring_spsc/tests/spsc_test.rs` |
| 172 | Multi-producer claim | `ring_mpsc` `ring_claim` | S5 | 4 producers × 25_000 items each arrive with byte-parity as a multiset of 100_000; no two producers are ever granted the same sequence; each producer's own items stay in its issue order | `ring_mpsc/tests/mpsc_test.rs` |
| 173 | Wait kind and strategies | `ring_types` `ring_wait` | S1/S4 | `WaitKind` has exactly the four discriminants `Spin`, `Yield`, `Park`, `None` in `ring_types`; `ring_wait` supplies one handler per discriminant; `WaitKind::None` returns without blocking when the ring is empty, asserted by a bounded-time test | `ring_types/tests/types_test.rs`, `ring_wait/tests/wait_test.rs` |
| 174 | Overflow policy enum and handlers | `ring_types` `ring_overflow` | S1 | `OverflowPolicy` has exactly the three discriminants `DropNewest`, `DropOldest`, `Fail` in `ring_types` — no overwrite-unread variant exists, asserted by exhaustive match; each handler on a full ring produces exactly its named outcome and increments the matching `ring_stats` counter | `ring_types/tests/types_test.rs`, `ring_overflow/tests/overflow_test.rs` |
| 175 | Thread-local buffer and flush-into | `ring_tls` | S2 | A `TlsBuffer` accumulates `N` items with zero atomic operations (asserted by a counting allocator/atomic shim), and one `flush_into` moves all `N` into the ring as a single contiguous claim | `ring_tls/tests/tls_test.rs` |
| 176 | Flush policy | `ring_flush` | S6 | Each of the three policies — `OnFull`, `OnBarrier`, `OnBatch(n)` — fires at exactly its stated trigger and at no other point, asserted by a scripted sequence with a recorded flush log | `ring_flush/tests/flush_test.rs` |
| 177 | Batch claim and batch drain | `ring_batch` | S2 | A claim of 64 slots issues one fence, not 64 (asserted against a counting ordering shim); the 64 sequences returned are contiguous; a batch drain reads them in issue order | `ring_batch/tests/batch_test.rs` |
| 178 | Sequence barrier and gating set | `ring_barrier` `ring_gating` | S4 | `ring_barrier` returns the minimum across a gating set of 1, 2 and 3 cursors; `ring_gating` refuses a claim that would advance past that minimum. A producer never overwrites an uncommitted slot, asserted over a full lap with a deliberately stalled consumer | `ring_barrier/tests/barrier_test.rs`, `ring_gating/tests/gating_test.rs` |
| 179 | Producer and consumer handles | `ring_handle` | S6 | `Producer` exposes no drain method and `Consumer` no publish method — asserted by a `trybuild` compile-fail case for each; both are `Send`, and the pair can be moved to two threads without a shared mutable reference | `ring_handle/tests/ui_test.rs`, `ring_handle/tests/handle_test.rs` |
| 180 | Ring config and factory | `ring_config` `ring_factory` | S1/S7 | `RingConfig` carries capacity, wait kind, overflow policy, producer count and batch size, and is the only constructor input; `Factory::build(cfg)` returns a handle pair whose observable behaviour matches every field, asserted one field at a time | `ring_config/tests/config_test.rs`, `ring_factory/tests/factory_test.rs` |
| 181 | Named ring registry | `ring_registry` | S7 | A ring registered under a name is retrievable by that name and by no other; a second registration under a live name is refused; dropping the registry drops every ring it owns, asserted by a drop counter | `ring_registry/tests/registry_test.rs` |
| 182 | Typed slot and bytes slot | `ring_slot` `ring_event` | S1/S2 | `TypedSlot<T>` and `BytesSlot` both round-trip through the identical claim/publish/drain path; a `T` written and read back is byte-identical, and a byte payload of arbitrary length up to slot size is byte-identical | `ring_slot/tests/slot_test.rs`, `ring_event/tests/event_test.rs` |
| 183 | Try-only operations on the tick path | `ring_poll` `ring_handle` | S6 | Every method reachable from the system-facing handle returns a `Result`/`Option` and none can park — asserted structurally, since there is nothing to call: a manifest scan pins the set of crates reaching `ring_wait` to the declared `PARKING_CRATES` roster and excludes every tick-path crate from it, a dependency-closure walk pins the two known transitive reaches the scan is blind to, and a bounded-time test shows the reachable path spins rather than sleeps | `ring_poll/tests/poll_test.rs` |
| 184 | Close, reset and drain-all | `ring_shutdown` | S6 | After `close()` a publish is refused and a waiter observes the close on its next look — `wait_for_close` polls through `ring_wait::wait_until`, so nothing parks and nothing signals, and a waiter whose spin budget runs out first gets `Err( RingError::Empty )` rather than a wake; `drain_all()` returns exactly the items outstanding at close; the close/`drain_all()`/`reopen()` trio hands back a ring observationally indistinguishable from a freshly constructed one, asserted against a live reference ring rather than byte-compared, with `reset()`'s own discard-and-stay-open path asserted separately in the same file | `ring_shutdown/tests/shutdown_test.rs` |
| 185 | Ring stats | `ring_stats` `ring_debug` `ring_trace` | S1/S2 | Counters for claimed, published, dropped-per-policy and wait-nanos each equal the operation count a scripted run performed; `ring_debug`'s invariant check catches a deliberately corrupted cursor; `ring_trace` records one entry per sequence operation when enabled and zero when not | `ring_stats/tests/stats_test.rs`, `ring_debug/tests/debug_test.rs`, `ring_trace/tests/trace_test.rs` |
| 186 | Ring benchmark harness | `ring_bench` | S8 | Every `Candidate::ALL` variant, under both accumulator semantics, produces a report from the same workload at the same producer counts, batch sizes and payloads; one written verdict per candidate; a missing configuration fails the test rather than being omitted from the table — asserted as `outcomes() + refusals() == Candidate::ALL.len()` rather than against a written-down product, because the roster is build-dependent: `OffTheShelf` is `crossbeam`-gated, so six candidates under `--all-features` and five without. An established list of four names the write paths in scope, not this roster | `ring_bench/tests/bench_test.rs` |
| 187 | Optional crossbeam queue backend | `ring_core` | S5 | With `--features crossbeam` the identical test suite passes against the crossbeam-backed core; the surface the suite drives is unchanged between the two builds, asserted by one suite parameterized over `every_backend()` — so each build runs it against exactly the backends it offers, and a roster test pins that count per configuration — with G2 compile-checking the other configuration's tests under `--no-default-features --all-targets` | `ring_core/tests/core_test.rs` |
| 188 | Loom and testkit helpers | `ring_testkit` | S7 | `loom` explores the claim/publish/commit interleavings to its declared bound and reports zero violations; the scripted-sequence fixture drives a ring through a recorded claim/drain script and reproduces the identical outcome on every run | `ring_testkit/tests/exhaustive_test.rs`, `ring_testkit/tests/testkit_test.rs` |

### Crate coverage

All 33 crates appear above. The four that no feature named — `ring_core`, `ring_debug`,
`ring_event`, `ring_trace` — are assigned by an established ruling on crate coverage; the two features
that named no crate — 183 and 187 — are assigned there too.

`ring_align` and `ring_atomic` are claimed jointly with 169 and 170 respectively, as
the padding and ordering primitives those features are stated in terms of.

### Related

- [invariant/001](../invariant/001_gate_non_vacuity.md) — why a gate reporting *reached* on an empty crate is the failure mode this table guards against
