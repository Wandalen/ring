# ring_core manual testing plan

`tests/core_test.rs` carries `the_same_program_behaves_identically_on_every_backend`,
one program written once and run against every backend, and the tests around
it. Line coverage is 100%, **measured with `--all-features`**. See C4, which is
about the reading itself.

The `crossbeam` feature makes this crate two programs rather than one, and that
shapes every check below. A gate that runs a single configuration is
half-blind on it. C3 and C4 check that the gates build and measure both.

The second theme is the crate's whole claim, *the backend is a build flag
rather than a rewrite*. A test that loops over backends asserts it, so the
loop's own coverage of backends matters in a way an ordinary assertion's does
not. C1 and C2 are about whether that loop, and the documentation beside it,
can fail at all.

Run from the workspace root. The code-line filter throughout is
`grep -nE <pattern> <file> | grep -vE '^[0-9]+:[[:space:]]*//'`. The number
comes from the *unfiltered* stream, because filtering first and numbering after
renumbers the surviving lines and understates every position.

## C1. The suite detects a broken composition, and the doc tests do not

**This is a mutation check: it edits the source, runs the suite, and restores.**
Take a copy first. The restore is the whole procedure, not a formality.

The mutation is not invented. It is the bug this crate shipped with for one
build. `Producer::try_push`'s MPSC arm called
`ring_mpsc::Producer::push`, which **takes the record by value and returns
`Result< Seq, RingError >`**. So on a full ring the record is consumed inside
it and there is nothing left to hand back, while this crate's signature promises
`Result< (), T >`. The only way to make that compile is to panic on refusal.
The fix claims a slot *before* consuming the record.

```bash
cp ring_core/src/lib.rs /tmp/-core_orig.rs

# Mutation — in `Producer::try_push`, replace the whole `ProducerInner::Mpsc`
# arm body (the `match producer.claim() { ... }` block) with:
#
#   Ok( producer.push( record ).map( | _ | () ).expect( "mpsc refusal carries no record" ) )

cargo nextest run -p ring_core --offline --features crossbeam 2>&1 \
  | grep -E 'FAIL|Summary'
cargo test --doc -p ring_core --offline --features crossbeam 2>&1 \
  | grep -E 'test result'

cp /tmp/-core_orig.rs ring_core/src/lib.rs
cargo nextest run -p ring_core --offline --features crossbeam
```

**Expected:**

| Suite | Result under the mutation |
|---|---|
| `core_test` | **FAIL**, including at least `a_partial_batch_push_reports_its_count_and_consumes_the_refused_record`, `drop_newest_discards_the_incoming_record_without_an_error`, `a_refused_record_comes_back_on_every_backend`, `four_threads_publishing_through_clones_lose_nothing` |
| doc tests | **none fail** |

Then all pass after the restore.

Read the FAIL list as a lower bound. Under this mutation nextest sometimes runs
the whole suite and sometimes stops scheduling tests partway while still
reporting `0 skipped`, and the failures beyond the four above vary between
runs. The list is complete only when the summary line shows the whole suite ran
(`N tests run`, not `M/N tests run`).

`four_threads_publishing_through_clones_lose_nothing` fails after its 30s
deadline rather than hanging (see C8, the concurrency test fails rather than
hangs).

**That no doc test fails is the finding.** Every doc example on this crate
could have shipped against a `try_push` that panics on any full multi-producer
ring. C2 explains why, and it is not an oversight that more examples would fix.

## C2. What the doc examples reach

Read the doc examples and classify them. The command extracts each block with
its line number, so you can check the classification against the source rather
than take it on trust:

```bash
awk '/^[[:space:]]*(\/\/\/|\/\/!) ```/{ if(!inb){inb=1; start=NR; body=""} else {inb=0; print "block @" start ":"; print body} ; next } inb{ sub(/^[[:space:]]*(\/\/\/|\/\/!) ?/,""); body = body "    " $0 "\n" }' ring_core/src/lib.rs
```

**Expected:** the classification below.

| Reached | Verdict | Evidence |
|---|---|---|
| SPSC push/pop | **yes** | every behavioural block |
| Saturation and refusal | **yes** | `Producer::try_push`'s example fills 2 slots and asserts `Err( 3 )`; `Producer::try_push_batch`'s is the batch form |
| Construction-time policy refusal | **yes** | `Ring::new`'s example |
| Backend selection | **yes** | `Backend`'s example |
| **MPSC push path** | **no** | `Backend`'s example builds an MPSC ring but only reads `.backend()` and never pushes |
| **Crossbeam, at all** | **no** | no block names it |
| Wrap | no | no block drains and refills |
| Contention | no | no block spawns a thread |

The MPSC row explains why no doc test fails under C1's mutation.
**`Producer::try_push`'s own doc example
exercises exactly one of its three arms.** That is not a gap to close by adding
examples. A doc test is prose that has to stay readable, and the example on
`try_push` is there to show the caller what a refusal looks like, which one
backend demonstrates as well as three. Backend coverage is
`the_same_program_behaves_identically_on_every_backend`'s job, and the division
is deliberate. But it means **the doc tests must never be cited as evidence
that this crate's backends work.** They are evidence that its API reads
well.

## C3. Both feature configurations are built by something

No test can check this. It is a property of which commands the gates run.

```bash
grep -nE '^[^#]*cargo [a-z]+ .*(--all-features|--no-default-features)' \
  bench_harness/gate/*.sh
```

**Expected:** among the hits, `g1_coverage.sh` running tarpaulin with
`--all-features`. `g2_docs.sh` must also run `check` once with
`--all-features` and once with `--all-targets --no-default-features`. It passes
the subcommand and flags to `cargo_over_workspaces` on a continuation line,
which this grep does not match, so read those calls with
`grep -n -A1 'cargo_over_workspaces' bench_harness/gate/g2_docs.sh`.

Anchored to `cargo <subcommand>` and to a non-`#` line start, not to the flag
name. The obvious form, grepping for `all-features` alone, also matches the
comments that explain the flags. A check that counts its own prose drifts
upward, and upward reads as extra safety.

Each configuration needs a gate:

- **G1** (coverage) must run tarpaulin with `--all-features`. See C4 for what a
  run with no feature flags produces.
- **G2** (docs / `cargo check`) must also build the default configuration.
  `--all-features` compiles the `#[ cfg( feature = ... ) ]` arms and skips
  every `#[ cfg( not( ... ) ) ]` one, so with `--all-features` alone the
  default build could stop compiling entirely and every gate would still pass.

Confirm the negative directly. This must fail if the default build is broken,
and it is the check G2 runs:

```bash
RUSTFLAGS="-D warnings" cargo check -p ring_core --all-targets --no-default-features --offline
```

**Expected:** clean. The default build's unique code is currently test-only
(`every_backend` has a `#[ cfg( not( feature = "crossbeam" ) ) ]` twin), which
is why `--all-targets` is not optional here. Without it the check
compiles the library, finds no `cfg(not(...))` code in it, and passes while
never looking at the half that has some.

## C4. The coverage figure depends on the feature set, and one reading is a miscount

```bash
cargo tarpaulin -p ring_core --offline --skip-clean --out Stdout --engine llvm 2>&1 \
  | grep -E 'ring_core/src'
cargo tarpaulin -p ring_core --offline --features crossbeam --skip-clean --out Stdout --engine llvm 2>&1 \
  | grep -E 'ring_core/src'
```

**Expected:** below 100% for the first and 100% for the second, **for the same
test suite**, with nothing tested differently.

The difference is not untested code. Print the lines the first run names and
every one of them is a `Storage::Crossbeam` / `ProducerInner::Crossbeam`
/ `ConsumerInner::Crossbeam` arm, i.e. code the default build's compiler
removed. **Tarpaulin counts `cfg`-eliminated lines as uncovered rather than
omitting them from the denominator.** Verified stable: repeated runs give the
same figure, and dropping `--skip-clean` does not change it, so it is a real
property of the tool rather than a stale-artifact effect.

So `--all-features` is the honest reading, the one where every counted
line is also a line the binary contains, and G1 runs with it. A gate that
reports less than 100% for a crate with nothing untested makes the family's
coverage figure unreadable.

**A second, smaller artifact of the same kind can pass for a gap.** Under the
family's brace style a bare `else` sits on its own line, and `llvm-cov` opens a
region there that nothing can execute. A crate written that way reads one line
short of 100% with both arms of the `if` tested. The family writes a `match`
instead:

```bash
grep -cE '^[[:space:]]*else[[:space:]]*$' ring_*/src/lib.rs | grep -v ':0'
```

**Expected:** no output. Any crate this prints will read one line short of 100%
for a reason that has nothing to do with its tests.

## C5. This crate adds no atomic and no unsafe

A correctness requirement, not hygiene. `ring_spsc` asserts **zero read-modify-writes across
a run**, and every publish through this crate reaches it. A counter added here
would break that assertion with nothing in `ring_spsc`'s own dependency tree to
blame for it.

```bash
grep -nE '\b(Atomic[A-Za-z]*|unsafe|Ordering::|fetch_[a-z]+|compare_exchange)\b' \
  ring_core/src/lib.rs | grep -vE '^[0-9]+:[[:space:]]*//'
grep -c 'ring_stats' ring_core/Cargo.toml
```

**Expected:** no output from the first, and `0` from the second. The module
documentation mentions `AtomicUsize` in prose, and the filter correctly drops
that line. `ring_stats` is deliberately not a dependency, because
instrumentation is where the counter would come from.

The filter's anchoring matters here more than usual: this crate's module
documentation argues about atomics at length, and an unanchored grep reports
that argument as an implementation.

## C6. The API matches `ring_handle`'s

`ring_handle`'s producer forwards to this crate's, so the two must agree on
receivers and on the refusal type. This check exists so a drift between them is
found by running a command rather than by someone noticing during review.

```bash
grep -nE 'pub fn (try_push|try_push_batch|free_capacity|is_closed)' ring_handle/src/lib.rs
grep -nE 'pub fn (try_push|try_push_batch|free_capacity|is_closed)' ring_core/src/lib.rs
```

**Expected:** three hits in each file, with the same signatures. `is_closed`
appears in neither.

| Point | `ring_handle` | `ring_core` |
|---|---|---|
| `try_push` receiver | `&mut self` | `&mut self` |
| `try_push_batch` receiver | `&mut self` | `&mut self` |
| `free_capacity` receiver | `&self` | `&self` |
| Refusal type | `T` | `T` |
| `is_closed` | absent | absent |

The two methods that take `&mut self` are the two that reach a backend
producer, and `ring_spsc`'s requires exclusive access. The one that takes
`&self` only reads. A `&self` push would need interior mutability or a `Copy`
producer at every backend.

`is_closed` is absent on purpose. Liveness is `ring_shutdown`'s, and a
handle-local copy of that flag is the failure `ring_shutdown` exists to
prevent, so this crate has no flag to read.
[`ring_handle`'s ADR 002](../../../ring_handle/docs/decisions/002_handles_have_no_is_closed.md)
records the same choice for the handles.

## C7. `free_capacity`'s binding contract is not tested, and cannot be here

The module documentation's table says `free_capacity` is **binding at SPSC,
advisory at MPSC and crossbeam**, and calls this the crate's sharpest hazard,
one signature over two contracts with no compiler error between them.

`free_capacity_never_overstates_the_room_available` tests the direction that is
testable from one thread, that a reported `n` never exceeds the room actually
available. It does not test the binding claim, because the binding claim is
*"and at SPSC, `n` pushes will therefore succeed"*, whose only failure mode
requires a second thread taking the room in between. A single-threaded test
cannot distinguish binding from advisory, because both satisfy it.

```bash
grep -nE '^fn .*free_capacity' ring_core/tests/core_test.rs
```

**Expected:** exactly one hit, `free_capacity_never_overstates_the_room_available`.
No test is named for the binding contract, because there is not one.

Anchored to `^fn `, not to the identifier. The bare form also returns the
comments arguing about the hazard. A stage whose expected count includes its
own explanatory prose cannot distinguish a test being added from a paragraph
being added.

What stands in for it is structural rather than behavioural. SPSC is the backend
where `try_clone` returns `None`, so no second producer can exist to invalidate
the reading. `try_clone_refuses_at_spsc_and_permits_elsewhere` is what
guards the contract, and it guards it by construction rather than by
observation. This is recorded here because a reader looking for a `free_capacity`
binding test will not find one and should not conclude it was forgotten.

## C8. The concurrency test fails rather than hangs

A retry-on-refusal loop with no bound is an unbounded spin the moment the ring
stops releasing slots, and a hung test in CI reports nothing at all.

Both loops in `four_threads_publishing_through_clones_lose_nothing` are bounded
by a 30s deadline. Their bounds differ, and the difference is the check:

- The **consumer** loop refreshes its deadline **on progress**, whenever a
  drain moved at least one record. That is correct, because it is waiting on
  work that arrives incrementally.
- The **producer** loop computes its deadline **once, outside the loop, and
  never refreshes it**. Refreshing it per iteration would push the deadline
  forward faster than time passes. The assertion could never fire, and the
  guard would silently become the unbounded spin it exists to prevent.

```bash
grep -n 'deadline' ring_core/tests/core_test.rs
awk '/while let Err\(returned\)/,/let mut deadline/ { if (/deadline[[:space:]]*=[^=]/ && !/let mut deadline/) print NR": "$0 }' \
  ring_core/tests/core_test.rs
```

**Expected:** the first lists, in order, the producer's deadline computed
before its loop, the comment recording why, the producer's assertion, the
consumer's init, the consumer's refresh inside `if drained > 0`, and the
consumer's assertion.

The second command reads from the producer's `while let` to the consumer's
`let mut deadline`. **It must print nothing.** Any *assignment* to `deadline`
inside the producer's `while let` body is the defect above, restored. A count
alone cannot catch it. The broken version had as many hits as the fixed one,
just one of them in the wrong place. That is why the range check is the real
test and the count is only orientation for it. An empty range also prints
nothing, so confirm the producer's loop still reads
`while let Err(returned) = mine.try_push(record)`.

Under C1's mutation this test fails after its 30s deadline with its own
message, rather than hanging, which is the evidence that the surviving guard
fires. It is the only observation of the deadline actually expiring, and it
comes free with C1, which is why the two stages are worth running in that
order.

## C9. The dependency edges are the ones the crate claims

Three claims about this crate's dependency list live only in prose.
`ring_stats`, `ring_shutdown` and `ring_handle` are *pointedly absent*. The
default build does not pull `crossbeam-queue`. The crossbeam build pulls it
with `default-features = false` (see
[ADR 002](../../docs/decisions/002_crossbeam_queue_is_an_interim_backend_inside_ring_core.md)).
Prose about a dependency list goes stale the moment someone adds a line.

```bash
cd ring_core
grep -cE '^(ring_stats|ring_shutdown|ring_handle) ' Cargo.toml
cargo tree -p ring_core -e normal --depth 1
cargo tree -p ring_core -e normal --depth 1 --features crossbeam
cargo tree -p ring_core -e features --features crossbeam
cargo tree -p ring_core -e normal --invert
```

**Expected**, one item per command after the `cd`:

1. **0**, the three absent crates. The grep anchors to column 0 so the prose in
   this very file cannot inflate it.
2. The crate and its in-house dependencies, and **no `crossbeam-queue`**. The
   default build must not pull the optional edge.
3. The same plus `crossbeam-queue`.
4. `crossbeam-queue` with only its `alloc` feature, and beneath it
   `crossbeam-utils` with **no** features. `crossbeam-queue` depends on
   `crossbeam-utils` unconditionally for `CachePadded` and `Backoff`, which
   `ArrayQueue` needs in every configuration, so `default-features = false`
   removes a *feature*, not a crate.
5. The crates built on `ring_core`. The *direction* is what matters. Edges
   point into this crate, never out of it.

To see what the flag removes, compare against a scratch crate that declares
plain `crossbeam-queue = "0.3"`:

```bash
cargo tree -p ring_core -e features --features crossbeam    # here
cargo tree -e features                                      # a scratch crate, plain `crossbeam-queue = "0.3"`
```

| | `crossbeam-queue` features | `crossbeam-utils` features | packages |
|---|---|---|---|
| default | `default`, `std`, `alloc` | **`std`** | 2 |
| ours | `alloc` | **none** | 2 |

The benefit is real, since `crossbeam-utils/std` is the thread-parking
machinery a spin-only queue never calls.

Item 4 uses `-e features` because `--depth 1` shows one level and the claim
about `crossbeam-utils` is about level two. A depth-limited check would print
the expected line count whether or not `crossbeam-utils` was pulled. A
prediction the command cannot reach is worse than a wrong one, because a wrong
one fails.

## Run Record

| Date | Stages | Outcome |
|---|---|---|
| 2026-08-28 | C1–C9 | 9/9 run. Six stages were wrong as first written and were corrected against real output rather than the output being explained away; two of those corrections changed machinery (`g1_coverage.sh`, `g2_docs.sh`) and one changed a test (the producer's deadline guard, which could never fire). C1 measured 4 integration failures against **0 of 7 doc-test failures** under the shipped MPSC bug, which is the crate's sharpest finding: the doc examples reach one of `try_push`'s three arms. C4 found the same crate reading 78.2% and 100% for one suite, 26 lines apart, and the low reading counting code the compiler had deleted. The crate's own citation of feature 187 named a file that does not exist and was corrected to `187_optional_crossbeam_queue_backend.md` |

### Per-stage detail

| Stage | Run | Correct as first written | Result |
|---|---|---|---|
| C1 | yes | yes | 4 integration FAIL, **0 of 7 doc tests fail**; 21/21 after restore |
| C2 | yes | yes | 7 blocks at 52/79/142/339/406/443/548; MPSC push path and crossbeam reached by none |
| C3 | yes | **no** | grep counted its own comments, giving 5 hits, not 3. Re-anchored to `cargo <sub>` |
| C4 | yes | **no** | expected one figure, got two: 93/119 vs 120/120, 26 lines `cfg`-removed |
| C5 | yes | yes | zero atomics, zero unsafe, `ring_stats` absent |
| C6 | yes | **no** | `free_capacity` was predicted to diverge; it agrees. Divergence is the two mutating methods only |
| C7 | yes | **no** | "hits in that test only" was false, with 6 hits, half of them prose. Re-anchored to `^fn ` |
| C8 | yes | **no** | predicted five hits, found six. A count cannot catch this defect at all, so it needed a range check |
| C9 | yes | **no** | 3 of 4 predictions held; `crossbeam-utils` is pulled regardless of `default-features`, and `--depth 1` could not have seen it either way |
