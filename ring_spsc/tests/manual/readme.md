# ring_spsc manual testing plan

`tests/spsc_test.rs` carries the reached-test for
`docs/feature/171_spsc_ring_api.md` and 27 tests around it, and the crate's
module documentation carries five `compile_fail` doc tests. Line coverage was
100% over 82 coverable lines as of 2026-08-28; re-measure rather than trust
that figure, since the source has grown since (`tests/manual/readme.md` S1's
Run Record row).

The gap automation leaves here is the largest of any crate in the family, and
it has one cause. **This is the first crate that opts out of the workspace's
`unsafe-code = "deny"`, and no test can demonstrate the absence of undefined
behaviour.** A data race that does not fire on this run passes. So the checks
below are about the *shape* the soundness argument rests on: that the unsafe
code is where it was ruled to be, that the negatives are checked, and that the
orderings are the ones the design claims. Two mutation checks then establish
that the suite can fail at all.

Run from the workspace root. The code-line filter is `^[[:space:]]*//`, which
drops `///`, `//!` and plain `//` alike. This crate's module documentation
discusses `Mutex`, `compare_exchange` and `Relaxed` at length in prose, and a
filter keeping ordinary comments would report that discussion as an
implementation.

## S1. The reached-test actually detects a broken ring

**This is a mutation check: it edits the source, runs the test, and restores.**
Take a copy first; the restore is the whole procedure, not a formality.

Feature 171's condition has three behavioural clauses: byte-parity, order, and
zero loss. The argument for asserting all three of one run is that each is
independently violable. That argument is worth exactly as much as the suite's
ability to notice each one, so this check breaks the ring three different ways
and confirms a different clause catches each.

```bash
cp ring_spsc/src/lib.rs /tmp/-spsc_orig.rs

# Mutation A — off-by-one fold: in `Batch::get`, change
#   self.start.advanced_by( offset as u64 )
# to
#   self.start.advanced_by( offset as u64 + 1 )
# Expect: byte-parity fails.

# Mutation B — premature commit: in `Batch::drop`, change
#   self.start.advanced_by( self.len as u64 )
# to
#   self.start.advanced_by( self.len as u64 + 1 )
# Expect: loss — the consumer skips a record it never read.

# Mutation C — the gate removed: in `Producer::claim`, delete the
#   if self.is_full() { return Err( RingError::Full ); }
# Expect: the producer laps the consumer; parity fails on overwritten slots.

for m in A B C; do
  cargo nextest run -p ring_spsc --all-features \
    -E 'test(one_producer_and_one_consumer_exchange_one_hundred_thousand_items)' \
    >/dev/null 2>&1
  echo "mutation $m exit $?"
done

cp /tmp/-spsc_orig.rs ring_spsc/src/lib.rs
cargo nextest run -p ring_spsc --all-features
```

**Expected:** `exit 100` for each of the three mutations, then a clean 28/28
after the restore. It was 25/25 when this step was first written, and the total
grew by three as S9 and S10 added tests; re-run rather than trust either
figure. A mutation that passes means the reached-test is asserting less than
feature 171 requires, and the gate would report REACHED against a ring that
loses records.

**`exit 124` is a failure of this check, not a pass.** That is `timeout`'s own
code, and it means the mutated run *hung* instead of failing. For that reason
each invocation is wrapped in `timeout 120` rather than run bare. Mutation B
produced exactly that on its first run, and the defect was in the test, not in
the mutation. The consumer loop said `while received.len() < ITEMS`, so a ring
that skips records waits forever for records that will never arrive. In CI
that reads as a slow machine. The loop now samples the producer's
`is_finished()` *before* each drain and stops when a finished producer leaves an
empty ring, so a lost record fails in about 30ms with a readable count mismatch.

The sampling order is the correctness of that exit, so do not "tidy" it.
Checking `is_finished()` after the drain instead of before would let the
producer publish between the two. The loop would then break having missed a
record it was about to receive, turning a correct ring into a flaky failure.

Run mutation C more than once. It is a race, not a deterministic error. With
capacity 1 024 and 100 000 items the producer laps the consumer readily, but a
scheduler that happens to run the consumer hot could hide it. It failed 5/5
times when measured. Anything less means the reached-test's capacity is too
large relative to the item count to force the wrap it depends on.

## S2. No lock in the path, and no read-modify-write either

This is feature 171's fourth clause, and the only one no test run can
demonstrate. An execution that did not block proves nothing about one that might.

```bash
grep -nE "Mutex|RwLock|Condvar|park|sleep|spin|fetch_add|compare_exchange|fetch_update" \
  ring_spsc/src/lib.rs | grep -vE "^[0-9]+:[[:space:]]*//"
```

Number **first**, then drop comments. The reverse order,
`grep -v ... | grep -n ...`, numbers the *filtered* stream, so every line
number it prints is the position in a file that does not exist on disk. Here
that understated the real positions by roughly 500 lines.

**Expected:** no output at all, not one hit. The producer path is a load, a
compare, a write and a release store; the consumer path is a load, a subtract,
N reads and a release store. A `compare_exchange` appearing here would not be a
lock, but it would mean the single-writer property had been given up, and with
it every argument in the module documentation's "Orderings" section.

The complement checks that the two paths *do* end in a release store, one each:

```bash
grep -nE "\.store\(" ring_spsc/src/lib.rs | grep -vE "^[0-9]+:[[:space:]]*//"
```

**Expected:** exactly two hits, both `HANDOFF`, at lines **794**
(`Reservation::drop`) and **1137** (`Batch::drop`). Two is the number. A third store means something publishes
or commits outside a guard's drop, which is the abandonment case the guard shape
exists to make unreachable.

## S3. The unsafe is exactly where decision 123 put it, and no wider

```bash
grep -n "unsafe fn\|unsafe impl\|allow( unsafe_code )" ring_spsc/src/lib.rs
grep -c "SAFETY:" ring_spsc/src/lib.rs
```

**Expected:** one `#![ allow( unsafe_code ) ]`; exactly two `unsafe fn`,
`slot` and `slot_mut`; and exactly one `unsafe impl`, for `Sync`. Seven
`SAFETY:` comments: one on the `unsafe impl`, one inside each of the two
`unsafe fn`, and one at each of the four call sites. Those sites are
`Reservation`'s `Deref` and `DerefMut`, and `Batch::get` and `Batch::get_mut`.

The count is the check, so note that it is seven, not four. The
two `unsafe fn` carry `# Safety` *sections* stating a precondition; the four
call sites carry `SAFETY:` *comments* discharging it. The workspace's
`undocumented_unsafe_blocks = "deny"` would catch a call site that acquired a
fifth caller without a comment. It would not catch one that acquired a comment
saying the wrong thing; only reading these seven against each other catches
it.

A third `unsafe fn` means the unsafe code grew. That is not forbidden, but it
changes what decision 123 ruled and what `docs/workaround/readme.md` justifies,
and both must move with it. The workspace's `undocumented_unsafe_blocks = "deny"`
catches a missing `SAFETY` comment on a block; nothing but this check catches a
*stale* one.

Every path into an `unsafe` operation must also stay private:

```bash
grep -nE "pub unsafe" ring_spsc/src/lib.rs | grep -vE "^[0-9]+:[[:space:]]*//"
```

**Expected:** no output. A `pub unsafe fn` would move the precondition onto
the caller, which for a crate reached only through `ring_handle` means moving it
somewhere no one will read it.

## S4. The negatives are checked rather than assumed

The soundness argument rests on four properties that are all absences, and an
absence cannot be asserted at runtime. They are `compile_fail` doc tests, and
`compile_fail` has a failure mode of its own. A block that fails to compile for
an unrelated reason, such as a typo or a missing import, passes just as green
as one that fails for the right reason.

```bash
grep -c '^//! ```compile_fail' ring_spsc/src/lib.rs
cargo test --doc -p ring_spsc --all-features 2>&1 | grep -c "compile fail ... ok"
```

**Expected:** five `compile_fail` blocks and five passing. The pattern is
anchored to the fence rather than to the bare word. A plain
`grep -c compile_fail` returns six, because the surrounding prose names the
mechanism too, and a check that counts its own explanation drifts the moment
the prose is reworded.

Then verify each
fails for its own reason, by making it compile and confirming the doc test
starts failing:

```bash
# In each block, delete the offending line — the `.clone()`, the second
# `split()`, the `assert_sync` call, the escaping `get`. The block should then
# compile and the doc test should FAIL.
cargo test --doc -p ring_spsc --all-features 2>&1 | tail -5
```

**Expected:** `test result: FAILED` naming the block you edited. This procedure
found two dead checks on first writing: `let _ = shared;` inside a spawned
closure captures nothing at all under edition-2021 precise capture, and a second
`split()` compiles fine when the first pair is never used again, because NLL
ends its borrow immediately. Both blocks were green and both tested nothing.

The blocks must live in `src/lib.rs`, not in `tests/spsc_test.rs`, because
**rustdoc collects doc tests from the library target only.** The same five
blocks written in an integration test file are never compiled. Confirm the
count moves when they do:

```bash
cargo test --doc -p ring_spsc --all-features 2>&1 | grep "^running"
```

**Expected:** `running 26 tests` and `running 5 tests`, two suites. The second
is the `compile_fail` set. If it says `running 0 tests`, the blocks have been
moved somewhere rustdoc does not look.

## S5. The orderings are asymmetric, deliberately

The crate's thesis is that single-writer cursors permit a weaker load than a
multi-producer ring can use. If every load ended up `Acquire`, the crate would
still be correct and the thesis would be untested.

```bash
grep -nE "load\( OWN \)|load\( GATING \)" ring_spsc/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*//"
```

**Expected:** both appear, and each is paired with the right cursor. Read the
hits: every `producer().load( OWN )` and `consumer().load( GATING )` is in a
`Producer` method; every `consumer().load( OWN )` and `producer().load( GATING )`
is in a `Consumer` method. A `load( OWN )` on the *peer's* cursor is a missing
acquire, and the slot writes it was supposed to make visible are then a data race.

**Why this is a source reading and not a test, stated carefully.** The usual
justification is that a missing acquire passes anyway on strongly-ordered
hardware, so no test could catch it. That justification does not apply here.
This dev host is aarch64 (Neoverse-N1), which is weakly ordered, so such a
mutation *might* be observable. The real reason this stays a source reading is
narrower. The window is small, the failure is probabilistic, and S9 measured
what that looks like in practice: the analogous `HANDOFF` mutation survives
100 000 items. A check that fails sometimes is worth having, but it does not
replace reading which cursor each load names, which is deterministic.

`Ring::fmt` is the one deliberate exception. It reads both cursors at `GATING`,
because it is called from neither end in particular.

## S6. Every declared dependency is used, and the absences are the design

```bash
comm -23 \
  <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_spsc/Cargo.toml \
     | grep -E "^ring_" | cut -d' ' -f1 | sort ) \
  <( grep -vE "^[[:space:]]*//" ring_spsc/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output. Five dependencies, all used.

The scaffolded manifest declared six: `ring_store`, `ring_cursor`,
`ring_claim`, `ring_publish`, `ring_consume`, `ring_config`. The
implementation needed a different five. `ring_claim`, `ring_publish` and
`ring_consume` all went, because each answers a question that only arises when
producers can overtake one another; `ring_slot` and `ring_types` arrived,
because `ring_store` re-exports neither the `Slot` trait it is generic over nor
the `Seq` its API speaks in. This is the third scaffolded manifest in the family
found to over-declare, so the check earns its place.

The absences are part of the design and should stay absent:

```bash
grep -E "^ring_(gating|claim|publish|consume|seq|index|barrier)" ring_spsc/Cargo.toml
```

**Expected:** no output. Any of these appearing means the SPSC path has
grown machinery the multi-producer ring needs and this one does not. The
crate's whole thesis, stated in the module documentation's first section,
would then no longer be true of its own manifest.

## S7. The reached-test's numbers have enough room to mean anything

```bash
grep -nE "const (ITEMS|CAPACITY|LAPS)" ring_spsc/tests/spsc_test.rs
grep -c "thread::scope" ring_spsc/tests/spsc_test.rs
```

**Expected:** `ITEMS = 100_000` in the reached-test, the figure feature 171
names and not a smaller one chosen for speed. `CAPACITY = 1_024`, so the ring
wraps 97 times and the producer must wait on the consumer. Four
`thread::scope` blocks: the reached-test, the capacity-2 stress test, and the
two departure tests.

The capacity-2 test is the one that would be quietly deleted as redundant. It is
not redundant. At capacity 1 024 the producer rarely blocks, so the reached-test
barely exercises a bug in the full-ring path, such as in the gate in `claim` or
the free-capacity arithmetic. At capacity 2 against 20 000 items that path is
hit on nearly every push.

## S8. The doc examples do not stand in for the tests

Twenty-six doc tests is a lot, and a reader may reasonably assume they cover
the public API. They do not, by construction. Each shows one operation's *shape*.

```bash
grep -oE "Capacity::new\( [0-9_]+ \)" ring_spsc/src/lib.rs | sort | uniq -c | sort -rn
```

**Expected:** capacities of 1, 2, 4, 8, 16 and 64, nothing larger. The 64
appears once, in `Ring::capacity`'s example, where no record is ever pushed. No
doc example wraps the ring even once or fills one beyond a handful of slots, so
none of them reaches the wrap, saturation and contention cases. That is the
right division, since a doc example that stress-tested would be unreadable. But
it means a coverage figure driven by doc tests would be measuring the wrong
thing. `cargo tarpaulin` does not count doc tests, which is why the coverage
figure in this page's opening paragraph is attributable to `tests/spsc_test.rs`
alone, not to the doc examples above.

---

## S9. The loom model can actually fail

**Why by hand.** This is the check that a check works, and it cannot be
automated by the thing it checks. `loom::model` returning `ok` proves nothing
on its own. A model that never observes the state it asserts about passes
identically to one that observes it and finds it correct. The only way to tell
them apart is to break the code deliberately and confirm the model notices.

**The finding this stage exists to record.** The first version of
`exhaustive::a_published_record_is_never_observed_before_its_payload_write`
pushed a byte into a `TypedSlot` and asserted the byte arrived. Under
`HANDOFF` mutated to `Relaxed` it **passed**, because loom instruments only
its own atomics, and a slot payload lives in plain memory behind the ring's
`UnsafeCell`. Loom never modelled the write, so it could not report it
unobserved. That is the third silently-vacuous check found in this crate, after
the `compile_fail` blocks in the wrong target and the two that compiled. The
common shape is a green result that was never capable of being red.

### Step 1. The model fails when the publish stops releasing

```bash
cd ring_spsc
sed -i 's|^pub const HANDOFF : Ordering = Ordering::Release;|pub const HANDOFF : Ordering = Ordering::Relaxed;|' src/lib.rs
RUSTFLAGS="--cfg loom" cargo test -p ring_spsc --test spsc_test 2>&1 | tail -20
```

**Expected:** `a_published_record_is_never_observed_before_the_write_that_preceded_it`
FAILS with "a drained record did not carry the write that preceded its
publish". Loom finds an interleaving in which the drain is offered the record
while the earlier `payload.store` is still unobserved.

**Also expected, and the reason loom is here at all:** the ordinary suite does
*not* catch this behaviourally. Run it under the same mutation:

```bash
cargo nextest run -p ring_spsc 2>&1 | tail -5
```

`one_producer_and_one_consumer_exchange_one_hundred_thousand_items` then
PASSES; only `the_two_orderings_are_the_ones_the_design_names` fails, and it
fails by reading the constant, not by observing a wrong value. 100 000 items do
not reorder those two stores. Scale is not coverage.

**And that holds on weakly-ordered hardware, which makes it stronger than it
first reads.** This dev host is aarch64 (Neoverse-N1), not x86-64, so the usual
"it would only show on a weaker target" escape is unavailable. The mutation
survives 100 000 items on precisely the kind of machine that is supposed to
expose it. An earlier revision of this section attributed the result to x86-64
and thereby explained away the one finding worth keeping.

### Step 2. Restore, and confirm both halves are green

```bash
cp /tmp/-spsc_orig.rs ring_spsc/src/lib.rs   # the copy taken in S1
RUSTFLAGS="--cfg loom" cargo test -p ring_spsc --test spsc_test 2>&1 | grep "test result"
cargo nextest run -p ring_spsc --all-features 2>&1 | tail -3
```

**Expected:** loom 2/2, ordinary 27/27.

### Step 3. The two halves are never compiled together

```bash
grep -nE "^#\[ cfg\( (not\( )?loom" tests/spsc_test.rs
```

**Expected:** exactly two hits: `#[ cfg( not( loom ) ) ]` on `mod threaded` at
line 62 and `#[ cfg( loom ) ]` on `mod exhaustive` at line 924.

The pattern is anchored to the attribute at column zero deliberately. The
unanchored form `grep -n 'cfg( loom )'` returns **three**, because the module
documentation explains the gate in prose. So the check as previously written
contradicted its own stated expectation of two, and drifted *upward*, which
reads as extra safety rather than as breakage.

The gate is not stylistic. Loom's atomics panic when touched outside a
`loom::model`, and every test in `threaded` constructs a `Ring` whose cursors
are those atomics under `--cfg loom`. The gate is what makes each half
runnable at all, and it is also why neither can quietly stand in for the other.

---

## S10. What the composition point found that this suite could not

`ring_core` (feature 187) is the first crate to use this one through a uniform
API shared with `ring_mpsc`. Composing two siblings side by side is a
different check from testing either. It compares them, and a suite that only
tests *this* ring cannot.

Three divergences showed up. **Only one was a gap in this crate**; the other
two are real and were deliberately absorbed rather than fixed.

### 1. `Batch::get_mut` was missing, and was added here

`get` yields `&S`, which is enough to read a record but not to move one out.
`TypedSlot::take` needs `&mut`. `ring_core`'s surface hands the caller a `T`
rather than a `&T`, because `ArrayQueue::pop` returns an owned value and no
borrow can outlive the pop. So the uniform drain cannot be built on `get` alone.

`ring_mpsc`'s batch always had its counterpart. This one did not.

```bash
cd "$(git rev-parse --show-toplevel)"
git show HEAD:ring_spsc/src/lib.rs | grep -c 'pub fn get_mut'    # 0 — absent before
grep -c 'pub fn get_mut' ring_spsc/src/lib.rs                    # 1 — present now
git show HEAD:ring_spsc/src/lib.rs | grep -c 'pub fn drain_up_to' # 1 — this one pre-existed
```

The third line is the control. `ring_core` needs `drain_up_to` just as much,
because a full `drain()` would commit the whole batch on `Drop` while
returning one record. But it was already here, so the composition point found
one gap rather than two.

### 2. This crate has no `Ends` type; `ring_mpsc` does

```bash
grep -c 'pub struct Ends' ring_spsc/src/lib.rs   # 0
grep -c 'pub struct Ends' ring_mpsc/src/lib.rs   # 1
```

Two sibling crates, two different split shapes. `ring_core` absorbs the
difference. Its `EndsInner::Spsc` holds `&mut ring_spsc::Ring` directly while
`EndsInner::Mpsc` holds a `ring_mpsc::Ends`. **Absorbed, not fixed**, because
adding an `Ends` here would be API churn to serve one consumer's tidiness rather
than any caller's need.

### 3. This crate's `Producer` is neither `Copy` nor `Clone`; `ring_mpsc`'s is `Copy`

```bash
grep -c 'impl.*Copy.*for Producer' ring_spsc/src/lib.rs   # 0
grep -c 'impl.*Copy.*for Producer' ring_mpsc/src/lib.rs   # 1
```

This is the sharpest of the three, and it is **not** a defect in this crate.
Exclusive access is the whole point of a single-producer ring. It matters
because it makes this crate the single blocker for a family-wide API
question. `ring_handle` specifies `try_push( &self, … )`, `ring_core` implements
`&mut self`, and the divergence reduces to this one `impl`. Recorded at
[`ring_core/docs/integration/002`](../../../ring_core/docs/integration/002_handle_surface_divergence.md),
owned by S6.

### Two unrun predictions

Written before running, unlike the three checks above, whose outputs were
already known when this stage was written:

```bash
grep -rn 'get_mut' ring_spsc/tests/                       # A
grep -n 'fn is_full' -A 14 ring_core/src/lib.rs           # B
```

**A. Expect at least one hit.** G1 requires 100% line coverage of this crate,
so a method added for an external consumer must be reached by something here. If
the only reach is the doc example, that is a finding: a method covered by
documentation rather than by a test.

**B. Expect `free_capacity() == 0` on at least the MPSC arm.** `ring_spsc` has
`is_full`; `ring_mpsc` does not. So `ring_core` must synthesize it for that arm,
which is a fourth absorbed asymmetry not yet recorded anywhere.

**Result (2026-08-28): both predictions held, and both taught something the
prediction did not anticipate.**

**A held, and the grep was written wrong.** `get_mut` is properly tested.
`a_record_taken_through_get_mut_leaves_its_slot_empty_across_a_wrap` at
`tests/spsc_test.rs:561` takes through it across a wrap and checks the slot is
empty afterwards, so the method is not doc-covered-only. But a large share of
the unanchored hits are **this file**, which lives under `tests/`. The grep
counts its own prose, including, unavoidably, this very sentence and every
other one on this page that names the method. That is defect shape 1 from
`ring_core`'s own plan summary, and this stage was written while naming that
shape, with `ring_core` C9's grep anchored against it two paragraphs earlier.
The anchored form:

```bash
grep -rn 'get_mut' ring_spsc/tests/ --include='*.rs'   # 7 hits, all real
```

**The unanchored count and "this file"'s share of it are deliberately not
pinned to a number here.** Both are self-referential in a way the anchored
form is not. This paragraph's own prose contains the string `get_mut`, so
every edit to this page, including the one that first wrote this caveat,
changes the very count it would be citing. `2026-09-11`'s bug hunt found this
mid-drift (11 unanchored / 4-in-this-file / 5 anchored, originally, had become
17 / 10 / 7 by the time it was checked) and corrects only the anchored number
above, which counts `.rs` files this page cannot itself perturb. Pin an exact
unanchored figure here and the next paragraph added anywhere on this page
quietly makes it wrong again.

**Knowing a defect shape does not prevent it.** That is the argument for
running the commands rather than reviewing them. Review is what produced the
broken version, twice, in the same sitting.

**B held, and is more uniform than predicted.** `ring_core::Producer::is_full`
is `self.free_capacity() == 0` for *every* backend. There is no `match`, no
per-arm dispatch, and `ring_core` therefore never calls `ring_spsc::is_full`
at all. So `ring_core` does not absorb the asymmetry by synthesizing the MPSC
arm; it sidesteps it by not using the SPSC one.

That has a consequence the prediction did not reach. **`is_full` inherits
`free_capacity`'s split contract exactly.** `free_capacity` is binding at SPSC
and advisory elsewhere, so `is_full() == false` does not mean a push will be
accepted at MPSC or crossbeam. Recorded at
[`ring_core/docs/pitfall/001`](../../../ring_core/docs/pitfall/001_free_capacity_carries_two_contracts.md),
where it belongs, rather than left implicit in a table cell reading "agrees with
`free_capacity() == 0`".

---

## Run Record

| Date | Checks | Result |
| ---- | ------ | ------ |
| 2026-08-28 | S1–S9 | 9/9 as expected. S1 measured A 3/3, B 3/3, C 5/5 mutated failures and 25/25 clean after restore, but only after B's first run returned `exit 124`, exposing a count-based consumer loop that hung instead of failing. S3, S4 and S8 each had a stated expectation corrected against real output rather than the code adjusted to match. S4 found two dead `compile_fail` blocks; both now fail for their stated reason. S9 found a third vacuous check, a loom model that passed under a `Relaxed` publish because loom does not instrument plain slot memory, and the model was restructured around a loom atomic until the mutation failed it |
| 2026-08-28 | S10 | Added after `ring_core` was implemented, to resolve a forward reference `Batch::get_mut`'s doc comment already carried. Three divergences from `ring_mpsc` confirmed: `get_mut` absent at HEAD and added here, no `Ends` type, `Producer` not `Copy`. Only the first was a gap in this crate; the second is absorbed by `ring_core`, and the third is correct behaviour that nonetheless makes this crate the single blocker for the family's `&self`/`&mut self` question. Both unrun predictions held. Neither was interesting for holding. A's grep counted its own prose, which is defect shape 1, committed in a stage that names that shape. B revealed that `is_full` inherits `free_capacity`'s split contract, which no document had said |
| 2026-09-11 | S2, S3, S4, S9 | Re-run against current `src/lib.rs`/`tests/spsc_test.rs` as part of a full-crate bug hunt. Four stale absolute citations found, all dating to the same original cause. S10's `get_mut` addition shifted line numbers, added a fourth unsafe call site, and added a doc test, without S2/S3/S4/S9 being updated to match. S2: `.store(` hits, originally at 725/1008, had moved to 771/1099 by the time this row was first drafted, then to **794/1137** after this same bug hunt's own `free_capacity` and `Batch::get_mut` doc fixes added lines above both sites. They are corrected to the post-fix figures directly, the only version worth recording. S3: the `SAFETY:` count had grown from six to **seven**. `Batch::get_mut`'s call site (paired with `slot_mut` at line 1067) is a fourth call site alongside `Deref`, `DerefMut` and `Batch::get`, and was never added to the enumerated list or the count. S4: the non-`compile_fail` doc-test count had grown from 25 to **26**, from `get_mut`'s own doc example. S9 Step 3: the `cfg( loom )` gate lines had moved from 58/870 to **62/924** (count still 2, corrected; `tests/spsc_test.rs` carried no further edits after that point). All four corrected in place against the final post-fix source; regenerating each recipe now matches its `Expected:` line exactly. **Lesson:** an absolute line-number citation can go stale even within the single sitting that fixes it, if that sitting also edits the file it cites. Regenerate citations *last*, after all other edits to the cited file are done, not as each one is discovered |
| 2026-09-11 | New: D2 reachability | `ring_debug/docs/invariant/002` DB35 declined to harden `ring_spsc::free_capacity`'s `capacity - occupancy`, reasoning D2 (`occupancy() > capacity`) is unreachable from any live `ring_core::Ring` a test can build. That claim was scoped to `ring_core`'s composition and never tested against `ring_spsc`'s own direct API. An `std::thread` stress test tried that directly first (2 to 128 racing bit-copies of one `Producer`, ~11.6M combined attempts against a starved consumer) and found nothing, which is inconclusive, not a proof. A new loom test, `exhaustive::free_capacity_degrades_safely_even_when_a_precondition_violation_reaches_d2`, searches the same question exhaustively instead of by luck and **does** find D2: `claim`'s `is_full` check and its own subsequent re-read of the producer cursor are two independent unfenced loads, and one racer's re-read can observe a second racer's already-landed publish, driving the cursor to 2 against a capacity of 1. Confirmed both directions with real output: reverting `free_capacity` to plain `-` and re-running reproduces `attempt to subtract with overflow` panicking at the exact call site; restoring `saturating_sub` passes cleanly across every interleaving loom explores, including the D2 one. `docs/invariant/001`'s Violation Consequences table and its "no consequence is a panic in this crate" claim are corrected to match. The claim is true now because of this fix, not because the violation became unreachable |
| 2026-09-11 | S10's "Two unrun predictions" | A second, independent instance of the same drift class as the S2/S3/S4/S9 row above, found by actually running the recipe rather than reviewing it. The anchored `grep -rn 'get_mut' ring_spsc/tests/ --include='*.rs'` recipe had drifted from its documented **5 hits** to **7**. Two more doc-comment mentions and call sites accrued in `tests/spsc_test.rs` since this section was written, none spurious, so "all real" still holds and only the count needed correcting. The unanchored sibling form (no `--include`) is a different problem in kind, not just degree. It counts this very page's own prose, so any edit to this page, including the one narrating the count, changes the number being cited. Rather than re-pin a second number certain to go stale on the next edit, the fix records the drift as a dated observation (11/4/5 originally, 17/10/7 when checked) and leaves only the non-self-referential anchored figure as a live, re-checkable assertion. **Lesson:** a recipe that counts a file's own prose cannot have a stable "expected" number for that half of the count. Anchor the assertion to what the recipe can hold still (here, `.rs`-only), and stop asserting the half that cannot |
| 2026-09-11 | Full-page sweep | Closing pass over every remaining recipe in this page (S1, S5–S8, and the opening paragraph) rather than stopping at the instances already found. S5–S7's exact assertions (both dependency greps' "no output", `ITEMS`/`CAPACITY` constants, the four `thread::scope` blocks, the six doc-example capacities) all still matched current source, so not everything in this page had drifted. Three more instances of the same class had: S1's restore-step total, stale at **25/25** (now **28/28**, because S9 and S10 added three tests since S1 was written); the opening paragraph's "24 tests around it" (now **27**, one below the current 28-test total since the reached-test itself is the 28th); and the "82/82 coverable lines" figure repeated in both the opening paragraph and S8, which a re-run via `cargo tarpaulin -p ring_spsc --all-features` could not cleanly reproduce. The invocation swept unrelated workspace crates into its denominator and reported `ring_spsc/src/lib.rs: 86/86` from what looks like only the library's own test binary, not the same measurement `tests/spsc_test.rs`'s coverage was originally attributed to. Rather than publish a number reached by an uncertain method, both mentions now read as a dated 2026-08-28 figure with an explicit re-measure pointer instead of a live assertion. **Lesson:** a full sweep of every recipe in a page, not just the ones a specific finding pointed at, is what closes out a documentation-drift check. The S10 row above and this row each found instances the other's scope would have missed |
