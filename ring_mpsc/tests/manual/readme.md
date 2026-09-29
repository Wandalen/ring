# ring_mpsc — manual testing plan

`tests/mpsc_test.rs` carries the reached-test for feature 172 and 30 tests
around it, plus two `loom` models under `--cfg loom`; `src/lib.rs` carries four
`compile_fail` doc tests. Line coverage is 100% over 113 coverable lines.

Automation leaves three kinds of gap here, and the stages below are organized
around them rather than around the source's structure:

1. **Absences cannot be asserted at runtime.** No lock is taken, no second
   consumer exists, no `unsafe` is public. Each is a `grep` or a
   `compile_fail`, and each has its own silent-vacuity failure mode.
2. **This crate opts out of the workspace's `unsafe-code = "deny"`**, and no
   test demonstrates the absence of undefined behaviour. A data race that does
   not fire on this run passes.
3. **Line coverage is not mutation coverage.** 113/113 says every line ran; it
   says nothing about whether any assertion would have noticed the line being
   wrong. M1, M2 and M4 are mutation checks, and M4 found a real gap that
   100% coverage had concealed.

Run from the workspace root unless a stage says otherwise.

**One trap that bit this plan on its first run, before anything else.** Every
mutation stage filters the suite down to one test. Use nextest's **positional**
filter:

```bash
cargo nextest run -p ring_mpsc four_producers_exchange       # correct
cargo nextest run -p ring_mpsc -E "test( four_... )"         # matches NOTHING
```

nextest's filterset grammar takes the spaces this project's Rust codestyle puts
inside parens as part of the pattern, so the `-E` form above selects zero tests
and exits **4**. With stderr discarded — which every mutation loop does — a
`4` is indistinguishable from a failing test, so *every* mutation reads as
caught, including the unmutated baseline. Always run the baseline first and
require it to be `0`; the runner script asserts the filter selects exactly one
passing test before it mutates anything.

---

## M1 — the parity reached-test detects a broken ring

**This is a mutation check: it edits the source, runs a test, and restores.**
Take a copy first; the restore is the whole procedure, not a formality.

Feature 172's condition is byte-parity across four concurrent producers, and
that argument is worth exactly what the test's ability to notice a break is
worth.

```bash
cp ring_mpsc/src/lib.rs /tmp/-mpsc_orig.rs

# Mutation A — off-by-one read: in `Batch::get`, change
#   self.start.advanced_by( offset as u64 )
# to
#   self.start.advanced_by( offset as u64 + 1 )

# Mutation B — premature commit: in `Batch::drop`, change
#   self.start.advanced_by( self.len as u64 )
# to
#   self.start.advanced_by( self.len as u64 + 1 )

# Mutation C — ordering instead of equality: in `contiguous_end`, change
#   if self.stamp( end ).load( OBSERVE ) != end
# to
#   if self.stamp( end ).load( OBSERVE ) < end

for i in 1 2 3; do
  timeout 180 cargo nextest run -p ring_mpsc four_producers_exchange >/dev/null 2>&1
  echo "run $i exit $?"
done

cp /tmp/-mpsc_orig.rs ring_mpsc/src/lib.rs
```

**Expected — and two of the three are not what a first reading predicts:**

| Mutation | Parity test alone | Reading |
|---|---|---|
| A — `Batch::get` off by one | **0/3 caught** | Correct, and not a defect. The parity test drains through `get_mut`, never `get`. A mutation on a method the test does not call cannot be caught by it, and treating this as a coverage hole would be misreading the mutation, not the suite (M2 catches it) |
| B — `Batch::drop` commits one too far | 3/3 caught, `exit 100` | The one straightforward result |
| C — equality → ordering | **1/3 caught** | A genuine weakness of *this test*, with a specific and interesting cause — below |

**Why C is only sometimes caught, and why that is the sentinel's doing.**
`UNSTAMPED` is `Seq( u64::MAX )`, above every real sequence; every *stale*
stamp is below the sequence being tested. So `< end` and `!= end` behave
identically on a stale slot and differ only on an **unstamped** one — which
exists only during the ring's first lap. After sequence 1024 the mutation is
undetectable in principle, and whether the first lap catches it is a race
between the consumer's first scan and the producers filling the ring.

That is worth stating twice, because it is a property of the design and not of
the test: **the sentinel's value is what makes the equality-vs-ordering
distinction observable at all.** Had `UNSTAMPED` been chosen below every real
sequence — the more obvious choice — `< end` would be indistinguishable from
`!= end` in every lap, and no test could ever separate the correct predicate
from the wrong one. `Seq( u64::MAX )` was picked for uniformity
(→ `docs/lifecycle/001_ring_construction_and_teardown.md`); its testability
consequence was not anticipated and is recorded here.

---

## M2 — the suite catches what the reached-test alone does not

M1's headline number is misleading on its own. Run the same three mutations
against the whole suite:

```bash
timeout 300 cargo nextest run -p ring_mpsc 2>&1 | grep -E "^ *FAIL|tests run:"
```

**Expected:**

| Mutation | Full suite | Tests that fire |
|---|---|---|
| A — `get` off by one | caught, deterministically | `a_live_batch_still_holds_its_slots_against_reuse`, `a_claim_dropped_without_a_write_publishes_an_empty_record`, `a_bytes_payload_round_trips_its_written_length` |
| A2 — `get_mut` off by one | caught, deterministically | six tests, including `a_heap_payload_arrives_with_its_contents_rather_than_a_shallow_copy` and `a_taken_record_leaves_its_slot_empty` |
| C — equality → ordering | **caught 5/5 runs** | 5–7 tests, always including `an_unpublished_claim_blocks_every_later_sequence_while_it_is_held` and `a_claim_dropped_without_a_write_publishes_an_empty_record` |

**The conclusion this stage exists to record: the small deterministic tests are
strictly stronger than the 100 000-item stress test at catching logic errors,
and the stress test is strictly stronger at catching ordering errors.** C is a
one-line predicate change that the reached-test catches a third of the time and
that six small single-threaded tests catch every time — because they construct
the first-lap state deliberately instead of hoping to race into it. Scale is
not coverage, and the converse is also true: none of those six would ever have
found M4's `PUBLISH` finding.

A2 is worth running even though it is A's obvious twin: `get` and `get_mut`
compute the same address in two places, so a fix applied to one and not the
other is exactly the shape a reviewer waves through.

---

## M3 — the unsafe surface is where decision 123 put it, and no wider

```bash
grep -n "unsafe fn\|unsafe impl\|allow( unsafe_code )" ring_mpsc/src/lib.rs
grep -c "SAFETY:" ring_mpsc/src/lib.rs
grep -nE "pub unsafe" ring_mpsc/src/lib.rs | grep -vE "^[0-9]+:[[:space:]]*//"
```

**Expected:** one `#![ allow( unsafe_code ) ]`, exactly two `unsafe fn` —
`slot` and `slot_mut` — one `unsafe impl` for `Sync`, **seven** `SAFETY:`
comments, and **no output at all** from the third command.

Seven rather than three is the check. One sits on the `unsafe impl`, one inside
each of the two `unsafe fn`, and four at call sites: `Reserved`'s `Deref` and
`DerefMut`, and `Batch`'s `get` and `get_mut`. The workspace's
`undocumented_unsafe_blocks = "deny"` catches a *missing* comment; nothing but
reading these seven against each other catches a *stale* one.

A third `unsafe fn` means the surface grew. That is not forbidden, but it
changes what
[decision 123](../../../../docs/decision/123_ring_shared_slot_storage_unsafe_sited.md)
ruled, and the ruling must move with it.

A `pub unsafe fn` would move the precondition onto the caller — which, for a
crate reached only through `ring_handle`, means moving it somewhere nobody
reads it.

---

## M4 — the four orderings, and the one nothing can check

The crate names four orderings as constants. Confirm each is where it claims
to be — note the `grep -n | grep -v` form rather than `grep -v | grep -n`, so
the line numbers are the file's own and not the filtered stream's:

```bash
grep -nE "\.store\(|\.load\(" ring_mpsc/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*//"
grep -nE "compare_exchange|fetch_add|fetch_update" ring_mpsc/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*//"
grep -nE "Mutex|RwLock|Condvar|park\(|sleep|spin" ring_mpsc/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*//"
```

**Expected:** exactly two stores — `stamp( seq ).store( seq, PUBLISH )` and
`consumer_cursor().store( .., COMMIT )` — and exactly three loads:
`consumer_cursor().load( GATING )`, `stamp( end ).load( OBSERVE )`, and
`consumer_cursor().load( OWN )`. **No output** from the second command: the
claim's compare-exchange lives in `ring_claim`, and this crate re-implementing
one would mean the seam had been bypassed. **No output** from the third.

Then mutate each ordering down to `Relaxed` in turn and record what notices:

| Mutation | Constant-reading test | 2 loom models | 60 hardware parity runs |
|---|---|---|---|
| `PUBLISH` Release → Relaxed | fails | **fails, deterministically, in ~0.01s** | **8 failures in 20** |
| `OBSERVE` Acquire → Relaxed | fails | **fails** | passes |
| `COMMIT` Release → Relaxed | fails | **passes** | **passes — 60/60, and 60/60 on the lap-reuse test too** |

**The finding: `COMMIT`'s `Release` has no behavioural check anywhere, and
100% line coverage concealed that completely.** Downgrading it to `Relaxed`
survives both loom models and 120 hardware runs on an aarch64 host — this
machine is ARM Neoverse-N1, so the usual "it only shows on a weakly-ordered
target" escape does not apply. The only thing that fails is
`the_orderings_are_the_ones_the_publication_invariant_names`, which reads the
constant and compares it to `Ordering::Release`. That is a check on the source
text, not on the machine.

**Why, and what the residual risk actually is.** `COMMIT` releases the
consumer's *reads* of the slots it is about to hand back. Loom cannot see them:
slot payloads live in plain memory behind `ring_store`'s `UnsafeCell`, and
loom instruments only its own atomics — the same limitation that made an
earlier version of the `PUBLISH` model vacuous (M9). Hardware cannot easily
show them either, because the window is one core's own load-to-store
reordering, not an inter-thread interleaving that more iterations make likelier.

So the argument for `Release` here rests on the memory model rather than on
evidence, and this plan says so rather than implying the 100% figure covers it.
Two things that *would* close it, neither taken:

- A loom model in which the payload is a loom atomic read by the consumer
  **before** its commit and overwritten by a producer **after** it. Whether
  loom explores the reordering that makes this fail is unverified — loom does
  not model every relaxed behaviour, and shipping a model that cannot fail is
  the exact anti-pattern M9 exists to catch. It must be written against the
  mutation, and kept only if the mutation reddens it.
- ThreadSanitizer or Miri under `-Zmiri-tree-borrows` with a wrapping workload.

Until one of them lands, `COMMIT` is the crate's one ordering held by argument
alone. `docs/invariant/002_publication_ordering.md` states the argument.

---

## M5 — the four `compile_fail` blocks fail for their own reason

`compile_fail` has a failure mode of its own: a block that fails to compile for
an *unrelated* reason — a typo, a missing import — passes just as green as one
that fails for the right reason.

```bash
grep -c '^//! ```compile_fail' ring_mpsc/src/lib.rs
cargo test --doc -p ring_mpsc 2>&1 | grep -c "compile fail ... ok"
```

**Expected:** four and four. The pattern is anchored to the fence rather than
to the bare word: a plain `grep -c compile_fail` returns five, because the
surrounding prose names the mechanism too, and a check that counts its own
explanation drifts the moment the prose is reworded.

Then verify each fails for its own reason by *making it compile*:

```bash
# Block 1 — delete the `assert_clone::< Consumer< .. > >()` call
# Block 2 — delete the `assert_sync::< Consumer< .. > >()` call
# Block 3 — delete the second `ends.split()`
# Block 4 — move the `Reserved` binding inside the inner scope
cargo test --doc -p ring_mpsc 2>&1 | tail -5
```

**Expected:** `test result: FAILED`, naming the block edited. Block 3 is the
one to check most carefully: in `ring_spsc` the equivalent block was *dead*,
because a second `split()` compiles fine when the first pair is never used
again — NLL ends the first borrow immediately. This crate's block uses
`_producer`/`_consumer` bindings that outlive the second call, which is what
keeps the first borrow alive; renaming them to `_` would silently kill the
check.

The blocks live in `src/lib.rs` and not in `tests/mpsc_test.rs`, and that is
load-bearing: **rustdoc collects doc tests from the library target only.** The
same four blocks in an integration test file are never compiled.

---

## M6 — every declared dependency is used, and two absences are the design

```bash
comm -23 \
  <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_mpsc/Cargo.toml \
     | grep -E "^ring_" | cut -d' ' -f1 | sort ) \
  <( grep -vE "^[[:space:]]*//" ring_mpsc/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )

grep -E "^ring_(publish|consume|spsc|overflow|barrier)" ring_mpsc/Cargo.toml
```

**Expected:** **no output from either.** Eight dependencies, all used; and the
five named absences all absent.

The two that matter are `ring_publish` and `ring_consume`. Message 960's
dependency forest gave this crate seven dependencies including both; the
implementation needed eight, without either. Publication here is a per-slot
stamp write and a scan rather than a cursor advance
([decision 124](../../../../docs/decision/124_ring_mpsc_publication_stamped_not_cursor.md)),
so neither crate's shape fits. Both reappearing in this manifest would mean the
stamp protocol had been quietly replaced by a published-cursor one, and every
argument in `docs/algorithm/002_batch_drain_by_cursor_swap.md` would be stale.

This is the fourth scaffolded manifest in the family found wrong, and the first
found to **under**-declare as well as over-declare — `ring_slot`, `ring_types`
and `ring_atomic` were all needed and none was scaffolded. The check earns its
place in both directions.

---

## M7 — the reached-test's numbers have enough room to mean anything

```bash
grep -nE "const (PRODUCERS|PER_PRODUCER|TOTAL|LAPS|CAPACITY|PATIENCE)" \
  ring_mpsc/tests/mpsc_test.rs
grep -nE "Ring::new\( capacity\( (1024|CAPACITY as usize) \) \)" \
  ring_mpsc/tests/mpsc_test.rs
grep -c "thread::scope" ring_mpsc/tests/mpsc_test.rs
```

**Expected:** `PRODUCERS = 4`, `PER_PRODUCER = 25_000` — 100 000 items, the
figure feature 172 names, not a smaller one chosen for speed — against
`capacity( 1024 )` at line 104, so the ring wraps 97 times and producers
genuinely hit `RingError::Full`. Separately `LAPS = 500` against `CAPACITY = 8`
at line 547, in
`every_slot_is_reused_across_many_laps_without_loss_or_duplication`, which is
the wrap-density test the parity test is too roomy to be.

Note the second grep matches `capacity( .. )` — the file's own two-line helper
around `Capacity::new( .. ).expect( .. )` — and not `Capacity::new( 1024 )`,
which appears nowhere in this file. A check written against the type's
constructor rather than the caller's helper finds nothing and reads as a
missing test.

`PATIENCE = 30s` is a deadline, not a timeout knob, and **it was added because
of M9 rather than by design.** The first version of the reached-test had
unbounded retry loops on both sides; M9's `PUBLISH` mutation killed the
consumer on a failed `expect`, the producers then spun on `RingError::Full`
forever, and the suite *hung* instead of failing — a test unable to report the
one defect it was written to catch. The deadline is the mutation's residue.

That is the general lesson worth carrying to the rest of the family: **a
mutation check does not only grade the assertions, it grades the test's failure
mode.** A suite that hangs under mutation is broken whether or not its
assertions are right, and nothing but running the mutation reveals it.

Both loops reset the deadline on progress, so a slow machine still passes and a
stalled ring still fails in bounded time. Do not "tidy" the deadline away, and
do not lower it to quiet a flaky run: a run needing more than 30 seconds of *no
progress at all* is a finding, not a timing problem.

---

## M8 — the doc examples do not stand in for the tests

```bash
# Per-block count of successful writes, attributed to the block's first line.
awk '
  /^[[:space:]]*(\/\/\/|\/\/!) ```/ {
    if( inb ) { printf "block @%d : %d\n", start, n } else { start = NR; n = 0 }
    inb = !inb; next
  }
  inb && /push\(|claim\(/ { n++ }
' ring_mpsc/src/lib.rs | sort -t: -k2 -rn | head -3
grep -nE "^[[:space:]]*///.*assert.*RingError::Full" ring_mpsc/src/lib.rs
grep -cE "^//!.*scope\.spawn" ring_mpsc/src/lib.rs
```

**Expected:** a maximum of **3** writes per block (at lines 701 and 1036), one
doc example *asserting* `RingError::Full` — line 712 only — and one spawning
threads.

The `assert` in that second pattern is load-bearing: without it the grep
returns four hits, three of which are prose in doc comments that merely name
the error. Third instance in this plan of the same failure — a check counting
its own documentation (M5, M9). Anchor every prose-adjacent grep to the syntax
it is really looking for, not to the identifier.

**The reading is the opposite of what this stage was first written to say, and
the correction is the point.** The plan initially claimed the doc examples were
toy-sized and reached none of the interesting states. Running the checks
refuted that on three counts:

| State | Reached by a doc example? | Where |
|---|---|---|
| Contention — several producers | **yes** | module-level example: 4 threads into 8 slots |
| Saturation — `RingError::Full` | **yes** | `claim`: 3 claims into 2 slots, third fails |
| A push after a drain | **yes** | `Batch::start`: push, drain, push, drain |
| **Genuine wrap — a slot index reused** | **no** | no block's successful writes ever exceed its own capacity |
| **Scale** | **no** | 4 items, against the reached-test's 100 000 |

So the doc examples are *not* trivial, and a claim that they are would be
wrong. The gap is narrower and much harder to see:

**The multi-threaded example sorts before it asserts.** `seen.sort_unstable()`
then `assert_eq!( seen, vec![ 0, 1, 2, 3 ] )` — it discards order and so
**cannot fail on any ordering defect at all**, which is the entire class of bug
this crate exists to get right. It is the crate's most impressive-looking
example and its least discriminating one. That is the correct choice for
documentation, where four threads make the point and a total-order assertion
would be flaky prose; it is a trap only if the example is mistaken for a test.

`cargo tarpaulin` does not count doc tests, which is why the 113/113 figure is
attributable entirely to `tests/mpsc_test.rs`. Had doc tests counted, this
example would have contributed coverage of the concurrent path while asserting
nothing about it — the exact shape of a number that overstates what is checked.

Also worth not being misled by: the capacity histogram
(`grep -oE "Capacity::new\( [0-9_]+ \)"`) shows one 64-slot and one 16-slot
ring, the two widest in the crate. Both push nothing whatsoever; they only
assert what `capacity()` returns. The widest examples are the emptiest, so
sizing tells you nothing about reach here.

`cargo tarpaulin` does not count doc tests, which is why the 113/113 figure is
attributable entirely to `tests/mpsc_test.rs`. The four methods that were
uncovered before the last three tests were written all *had* doc tests already;
that is what surfaced the distinction.

---

## M9 — the loom models can actually fail

**Why by hand.** This is the check that a check works, and it cannot be
automated by the thing it checks. `loom::model` returning `ok` proves nothing
on its own: a model that never observes the state it asserts about passes
identically to one that observes it and finds it correct.

**The finding this stage inherits.** `ring_spsc`'s equivalent model pushed a
byte into a `TypedSlot` and asserted the byte arrived. Under a `Relaxed`
publish it **passed** — loom instruments only its own atomics, and a slot
payload lives in plain memory behind an `UnsafeCell`. This crate's model was
written against that finding from the start: its payload is a
`loom::sync::atomic::AtomicUsize` stored before the publish and loaded after
the drain, so the thing whose visibility is asserted is something loom can see.
M4 shows the same limitation is still live for `COMMIT`.

### Step 1 — the model fails when the publish stops releasing

```bash
cd ring_mpsc
sed -i 's|^pub const PUBLISH : Ordering = Ordering::Release;|pub const PUBLISH : Ordering = Ordering::Relaxed;|' src/lib.rs
RUSTFLAGS="--cfg loom" timeout 300 cargo test -p ring_mpsc --test mpsc_test 2>&1 | tail -12
```

**Expected:** `a_published_record_is_never_observed_before_the_write_that_preceded_it`
FAILS with *"a drained record did not carry the write that preceded its
publish"*, `left: 0`, `right: 2748`. It takes about 19 s, essentially all of it
compilation — the model itself is deterministic and takes milliseconds.

**Also expected, and the reason loom is here at all:** the hardware suite
catches the same mutation only **8 times in 20**. 40 % is a real check and not
a reliable one; loom enumerates where hardware samples.

### Step 2 — restore, and confirm both halves are green

```bash
cp /tmp/-mpsc_orig.rs src/lib.rs
RUSTFLAGS="--cfg loom" timeout 300 cargo test -p ring_mpsc --test mpsc_test 2>&1 | grep "test result"
cargo nextest run -p ring_mpsc 2>&1 | tail -3
```

**Expected:** loom 2/2, ordinary 31/31.

### Step 3 — the two halves are never compiled together

```bash
grep -nE "^#\[ cfg\( (not\( )?loom" tests/mpsc_test.rs
```

**Expected:** exactly two hits — `#[ cfg( not( loom ) ) ]` on `mod threaded`
at line 48 and `#[ cfg( loom ) ]` on `mod exhaustive` at line 900.

The pattern is anchored to the attribute at column zero for the same reason
M5's is anchored to its fence: the unanchored form `grep -n 'cfg( loom )'`
returns **three** hits, because the module doc comment explains the gate in
prose. A check that counts its own documentation drifts the moment the prose is
reworded, and drifts *upward*, which reads as extra safety rather than as
breakage. This crate has now been bitten by that shape twice.

The gate is not stylistic: `ring_atomic`
swaps `AtomicSeq` for loom's instrumented atomic under the same cfg, and loom's
atomics panic when touched outside a `loom::model`. Every test in `threaded`
constructs a `Ring` whose stamps are those atomics. The gate is what makes each
half runnable at all, and also why neither can quietly stand in for the other.

---

## Run Record

| Date | Checks | Result |
| ---- | ------ | ------ |
| 2026-08-28 | M1–M9 | 9/9 run. **M1's first execution was entirely invalid and reported a perfect score**: the mutation loop filtered with `-E "test( name )"`, whose inner spaces make nextest's filterset match nothing and exit 4, and the loop discarded stderr — so the unmutated baseline "failed" too and every mutation looked caught. Fixed to a positional filter plus a baseline assertion, then re-run. Corrected results: B caught 3/3; **A caught 0/3 by the parity test** (it drains through `get_mut`, so the mutation is unreachable from it — M2 catches it via three other tests); **C caught only 1/3**, traced to `UNSTAMPED = Seq( u64::MAX )` making the equality-vs-ordering distinction observable on the first lap only. M4 mutated all three orderings and found **`COMMIT`'s `Release` has no behavioural check at all** — `Relaxed` survives both loom models and 120 aarch64 hardware runs, caught only by the constant-reading test; recorded as an open gap rather than closed with a model that might not be able to fail. M9's `PUBLISH` mutation reddened loom deterministically and hardware 8/20. M3 and M6 passed as first written; M6's expectation was set from real output rather than from message 960's predicted dependency list, which was wrong in both directions. **M7, M8 and M9-step-3 did not, and were corrected against measurement rather than shipped** — M7 grepped `Capacity::new( 1024 )`, which appears nowhere in the test file (it uses a local `capacity( .. )` helper), so the check found nothing and read as a missing test; M9's cfg grep returned 3 rather than 2 by counting the prose that explains the gate; and **M8's premise was simply false** — it claimed no doc example reached wrap, saturation or contention, whereas the examples reach saturation (line 712), contention (4 threads, module level) and a post-drain push, and the real gap is that the concurrent example calls `seen.sort_unstable()` before asserting and so cannot fail on any ordering defect. Three of nine stages wrong on first writing, all three found by executing the plan's own commands instead of trusting them |
