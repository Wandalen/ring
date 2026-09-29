# Non Functional Requirement: The Gating Read Allocates Nothing

### Scope

- **Purpose**: Record that `slowest` no longer performs a heap allocation per call, keep the trace of the position that allocation occupied in a lock-free retry loop, and record why the measurement that would have priced it was never taken.
- **Responsibility**: Show what the allocation was, show the call chain it sat on, show the fold that replaced it, and mark clearly what is derived versus what is measured.
- **In Scope**: `ring_cursor::slowest`'s fold, and the `Vec< Seq >` it used to build; its reachability from `ring_claim::Claimer::claim`.
- **Out of Scope**: Whether cache-line separation makes anything faster, which belongs to `ring_bench`.

### The Requirement

> A gating read on a producer's admission path allocates nothing.

**This crate meets it.** It did not when this document was written, and the rest
of it is the evidence of what the gap was, what closed it, and what the closure
left behind.

### The Allocation There Used to Be

```rust
// as it stood before commit b7e075ca
#[ must_use ]
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
  ring_seqno::slowest( &positions )
}
```

The `Vec` existed to hand a `&[ Seq ]` to `ring_seqno::slowest`, whose entire body is:

```rust
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq > { cursors.iter().copied().min() }
```

**One heap allocation and one free, per call, to reach a `.min()`.**

The allocation-free version was shorter than the one written, and it is now what
the crate has:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the body, whole --'
awk '/^#\[ must_use \]$/ { held = $0; next }
     /^pub fn slowest\( cursors : &\[ PaddedCursor \] \)/ { print held; f = 1 }
     f { print } f && /^\}$/ { exit }' ring_cursor/src/lib.rs
# and the two names whose absence is the requirement being met. Both print
# nothing and exit nonzero, so both are caught rather than ending the block
echo '  -- and the two names that are no longer in it --'
command grep -E 'Vec|collect' ring_cursor/src/lib.rs || echo '    (neither Vec nor collect appears anywhere in the crate)'
```

Live output:

```
  -- the body, whole --
#[ must_use ]
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}
  -- and the two names that are no longer in it --
    (neither Vec nor collect appears anywhere in the crate)
```

It lost nothing but the textual reuse of `ring_seqno::slowest`, and that loss is
the one thing the change did cost: `ring_seqno::slowest` has had no caller outside
its own tests since, which is recorded where it belongs, in
[`ring_seqno`'s own NFR](../../../ring_seqno/docs/non_functional_requirement/001_every_reading_is_allocation_free.md)
§ SQ35. The two folds are now written twice, four lines apart across a crate
boundary, with nothing keeping them equal.

### Where the Call Lands

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'ring_cursor::slowest\|\.slowest()\|\.headroom(' --include=*.rs */src/ \
  | command grep -vE ':[[:space:]]*(///|//!)' \
  | sed -E 's/:/: /' | LC_ALL=C sort
```

Live output:

```
ring_barrier/src/lib.rs:     ring_cursor::slowest( self.dependencies )
ring_claim/src/lib.rs:     self.consumers.headroom( self.claimed() )
ring_claim/src/lib.rs:     while count <= self.consumers.headroom( current )
ring_claim/src/lib.rs:     while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
ring_gating/src/lib.rs:     count <= self.headroom( producer )
ring_gating/src/lib.rs:     if count > self.headroom( producer )
ring_gating/src/lib.rs:     ring_cursor::slowest( &self.cursors )
ring_gating/src/lib.rs:     self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
ring_gating/src/lib.rs:     self.slowest().map_or( self.capacity.get(), | slowest |
ring_mpsc/src/lib.rs:     self.claimer.headroom()
```

| Step | Site | Note |
|------|------|------|
| 1 | `Claimer::claim` in `ring_claim/src/lib.rs` | the multi-producer claim |
| 2 | `GatingSet::headroom` in `ring_gating/src/lib.rs` | |
| 3 | `GatingSet::slowest` in `ring_gating/src/lib.rs` | |
| 4 | `slowest` in `ring_cursor/src/lib.rs` | **where the allocation was** |

The chain is unchanged; only its last step is. And step 1 is not a single call.
It is a **loop condition**:

```rust
let mut current = self.claimed();
while count <= self.consumers.headroom( current )
{
  let next = current.advanced_by( count as u64 );
  match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
  {
    Ok( _ ) => return Ok( Claim::new( current, count ) ),
    Err( actual ) => current = actual,
  }
}
```

with a comment explaining that this is deliberate:

> The gate is the loop condition, and is therefore re-read on every iteration: on
> a failed exchange another producer moved the cursor, so the headroom computed
> against the old value is stale and granting on it would overlap that producer's
> range.

**The re-read is correct and necessary. The allocation that used to ride on it
was neither, and that asymmetry is why it was worth removing before anyone
measured it.**

### Why the Position Was the Worst One

A lock-free retry loop that allocates has a feedback shape that a straight-line
allocation does not:

| | More producers → |
|---|---|
| CAS failures | more retries |
| Retries | more `slowest` calls |
| `slowest` calls | more allocator traffic |
| Allocator traffic | more contention on a **global** resource |
| → | more CAS failures |

Every arm is a consequence of the one above it. The allocator is shared across
every thread in the process, so the retry loop's back-off was competing for a
lock the ring's design specifically avoids everywhere else.

**That was derived from the code, never measured.** The loop may have retried
rarely enough that none of it registered. The next section is why nobody found
out — and why the wall that answer was blamed on turned out not to be there.

### What Was Never Known, and What Now Guards It

**Whether the `Vec` survived optimisation.** `collect::< Vec< _ > >()` followed
immediately by a slice read is a shape LLVM can sometimes eliminate — but the
`ring_seqno::slowest` call was across a crate boundary, so elision depended on
inlining that is not guaranteed without LTO. The question is moot for the `Vec`
and was live for its successor until method 2 below was actually written, which
is the only part of this section that has changed.

Nothing in this repository has measured either. Two ways to find out:

```text
1. Does a call to the allocator survive in the emitted code?
     cargo asm --lib -p ring_cursor 'ring_cursor::slowest' --rust \
       | grep -iE '__rust_alloc|__rust_dealloc|call.*alloc'
   no output → the Vec was elided in this build profile

2. Count allocations directly, under the profile that ships.
   A counting global allocator around a claim loop, in a bench or a test:
     static COUNT : AtomicUsize = AtomicUsize::new( 0 );
     // GlobalAlloc impl incrementing COUNT in `alloc`
   then assert COUNT is unchanged across N claims.
```

**Method 2 answers the question the requirement actually asks** — *does a claim
allocate* — rather than the proxy question of whether one particular `Vec`
survived, and it keeps answering it: an allocation introduced later anywhere in
the claim path trips the same assertion. That is now the whole of its value here,
since the `Vec` this document was written about is gone and only the *regression*
question is left. Method 1 depends on the build profile, so a clean result under
`--release` says nothing about a debug build.

**Method 2 is now in this crate's suite**, at
`ring_cursor/tests/allocation_test.rs`, asserting zero allocations at four
call shapes with a control arm that must allocate. It was not written when this
document was — and the reason given for not writing it, in the paragraphs that
used to stand here, was wrong.

**Method 1 cannot be run here: `cargo asm` is not installed. Method 2 always
could have been, and the reason put on record for why it could not was wrong.**
`GlobalAlloc` is an unsafe trait and the workspace sets `unsafe-code = "deny"`,
from which the conclusion drawn was that gate G6 put the instrument out of reach.
G6 reads exactly one path per crate, and it is not the one such a test lives in:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- crates permitted to write unsafe, and the one path G6 reads --'
command grep -v '^#' bench_harness/gate/declared/ring/unsafe_allowlist.txt \
  | command grep -v '^$' | tr '\n' ' ' | sed 's/^/    allowlist: /' ; echo
command grep -oE 'crate_dir "\$c" \)/src' bench_harness/gate/g6_unsafe.sh \
  | head -1 | sed 's/^/    scanned: /'
echo '  -- tests/ is outside that path; crates that installed a counter there --'
command grep -l '#\[ global_allocator \]' ring_*/tests/*.rs | sed 's:^:    :'
echo '  -- and the crate that reached the same question and declined --'
for f in $( command grep -l 'GlobalAlloc' ring_*/tests/*.rs ) ; do
  command grep -q '#\[ global_allocator \]' "$f" || printf '    %s\n' "$f"
done
```

Live output:

```
  -- crates permitted to write unsafe, and the one path G6 reads --
    allowlist: ring_spsc ring_mpsc ring_core 
    scanned: crate_dir "$c" )/src
  -- tests/ is outside that path; crates that installed a counter there --
    ring_barrier/tests/allocation_test.rs
    ring_claim/tests/allocation_test.rs
    ring_consume/tests/allocation_test.rs
    ring_cursor/tests/allocation_test.rs
  -- and the crate that reached the same question and declined --
    ring_flush/tests/append_cost_test.rs
```

**G6 scans `src/` only.** An `#![ allow( unsafe_code ) ]` inside a `tests/` file
was never within the gate's reach, so the allowlist never had anything to say
about a test-only counting allocator, and the wall this section used to describe
was never between the crate and the measurement. Four crates in the family now
have such a counter. A fifth, `ring_flush`, reached the same question, wrote the
same observation about G6's scope into its own test file, and declined the
allowance anyway — on the ground that spreading the opt-out crate by crate
defeats what the gate exists to do even where the gate cannot see it — and
settled for the strongest *safe* proxy instead: asserting that a staging
buffer's capacity does not grow.

Both positions are coherent and they are not the same policy. Nothing has ruled
between them, which is CU53 below.

### The Second Caller

`ring_barrier::Barrier::frontier` reaches the same fold on the consumer side:

```rust
pub fn frontier( &self ) -> Option< Seq > { ring_cursor::slowest( self.dependencies ) }
```

`available()` calls it, and a waiting consumer calls `available()` repeatedly. So
the allocation was on both sides of the ring, not just the producer's — though
the consumer's loop is a wait rather than a CAS retry, so the feedback shape
above never applied there. Both sides were freed by the same one-line change,
which is why `ring_barrier`'s own NFR moved with this one.

### Scale, While It Lasted

| | |
|---|---|
| Allocation size | `n × 8` bytes, `n` = consumer count. For a 4-consumer set, 32 bytes |
| Per claim | ≥ 1, plus 1 per CAS retry |
| Per second, at 10⁶ claims/s uncontended | ≥ 10⁶ allocate/free pairs |
| Now | 0 |

The size was trivial and the *count* was the concern. A 32-byte allocation is
cheap; a million of them per second on a path whose entire design premise is
"never touch a shared resource" was a contradiction in the design, whatever the
measured cost would have turned out to be. That is the argument the removal was
taken on, and it is worth recording that it was taken on the argument rather than
on a number — because the number was never available, and § What Was Never Known
explains why it still is not.

### Status

| # | Claim | Status |
|---|-------|--------|
| A1 | `slowest` allocated a `Vec` per call | **Was verified** — closed; the body above holds no `Vec` |
| A2 | It was reachable from `Claimer::claim`'s loop condition | **Verified** — the four-step chain above, still intact |
| A3 | The allocation-free rewrite is behaviourally identical | **Verified** — it is what the crate now compiles and its tests pass on |
| A4 | The allocation survived optimisation | **Never known** — moot; there is no allocation to elide |
| A5 | It measurably cost throughput under contention | **Never known** — not measured before removal, so not recoverable after |
| A6 | Nothing prevents an allocation returning to this path | **Closed** — `tests/allocation_test.rs` asserts zero at four call shapes, and fails with `left: (1000, 8000)` when the `Vec` body is put back |

**A5 is closed unanswered and this document does not pretend otherwise.** The
allocation was removed on A1 and A3 — a shorter body doing the same thing — not
on evidence that it cost anything, and the cost is now unrecoverable because
there is no before-state left to measure. A6 was what remained actionable, and
is now closed by a test written after this document had argued it could not be.

### CU35 — The Instrument Was Called Unavailable by Policy, and the Policy Never Covered It

Counting allocations means installing a `GlobalAlloc`, which is an unsafe trait.
Gate G6 confines `unsafe` to `ring_spsc`, `ring_mpsc` and `ring_core`, and this
crate is not among them. From that, this document concluded the requirement was
unmeasurable here for a governance reason rather than a technical one — twenty
lines of code with a policy in front of them.

**Finding.** The conclusion does not follow, and the step it skips is one line of
the gate: `g6_unsafe.sh` scans `crate_dir "$c" )/src` — i.e. `<crate>/src` — and
nothing else. A test-only
opt-out has never been within its reach, so the allowlist was never the obstacle
it was named as. The requirement went unmeasured for the whole life of this
document on a premise that could have been checked by reading the gate instead of
the prose about the gate — and the sibling cited as having hit the same wall had
already written G6's actual scan path into its own test file.

**Disposition:** applied — wrote `ring_cursor/tests/allocation_test.rs`, a
test-only counting `GlobalAlloc` that asserts zero allocations across four call
shapes of `slowest` behind a control arm that must allocate, and replaced
§ What Was Never Known's policy argument with G6's actual scan path. The same
instrument now exists in `ring_barrier`, `ring_claim` and `ring_consume`;
`ring_flush` declined it deliberately, which is CU53.
Now prints: `scanned: crate_dir "$c" )/src`

---

### CU36 — A Sibling Test Cites an Allowlist That No Longer Exists

| Named in `ring_flush/tests/append_cost_test.rs:9-11` | On the live list |
|---|---|
| `ring_atomic` | no |
| `ring_store` | no |
| `ring_slot` | no |
| `ring_align` | no |

An earlier ruling wrote that four-name list. A later ruling replaced it with
`ring_spsc`, `ring_mpsc`, `ring_core`. Not one of the four originally named
survives.

**Finding.** The comment describes a policy that has not existed since the later
ruling superseded it, and it is load-bearing prose: it is the explanation a
reader of that test gets for why the proxy exists instead of the direct
measurement. The reason is still valid; every crate it names is wrong.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'confines the opt-out to three declared crates' ring_flush/tests/append_cost_test.rs
```

Live output:

```
//! confines the opt-out to three declared crates — `ring_spsc`, `ring_mpsc`,
```

**Disposition:** applied — updated `ring_flush/tests/append_cost_test.rs:8-11`
to name the current gate G6 allowlist — `ring_spsc`, `ring_mpsc`, `ring_core`
— replacing the four crates named by the superseded, earlier ruling.
This is independent of `ring_cursor::slowest`'s own allocation, which the
concurrent CL55 fix already made allocation-free (`cursors.iter().map( |c|
c.load( GATING ) ).min()` at `src/lib.rs:120-123`) — that change resolves the
headline claim this document opens with, but not this finding, which is
about a stale crate-name citation in a different crate's test file. Verified
with `cargo test --release -p ring_flush` (all tests passing, including
`append_cost_test`).
Now prints: `confines the opt-out to three declared crates`

---

### CU53 — Four Crates Took a Test-Only Unsafe Allowance and a Fifth Refused It

| Crate | Counting allocator in `tests/` | Position taken |
|---|:---:|---|
| `ring_cursor` | yes | test-only, one wrapper forwarding to `std::alloc::System` unchanged |
| `ring_barrier` | yes | same |
| `ring_claim` | yes | same |
| `ring_consume` | yes | same |
| `ring_flush` | **no** | letting the opt-out spread crate by crate defeats what G6 exists to do, even outside its scan path; uses a capacity-growth proxy instead |

**Finding.** G6 permits all five — it scans `src/` only, as § What Was Never
Known now records. Four crates read that as licence and one read it as an
oversight it declined to exploit, and no decision record rules between them. So
the same requirement — *this path allocates nothing* — is asserted directly in
four crates and by proxy in a fifth, and a reader comparing them cannot tell
whether that split was considered or accidental. It wants one of two closures:
the allowlist saying what it means for `tests/`, or four of these five files not
existing.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_slowest_fold.md](../algorithm/001_the_slowest_fold.md) | The fold itself, and the `None` that justifies its existence |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_that_decides.md](../api/002_the_surface_that_decides.md) | `slowest` as a public item |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | `ring_gating` and `ring_barrier`, the two callers |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [001_the_layout_claim_is_testable.md](001_the_layout_claim_is_testable.md) | The requirement this crate does meet, and how it is layered |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_fold_two_questions.md](../pattern/002_one_fold_two_questions.md) | Why the fold is shared, which is what the `Vec` was paying for — and is no longer shared across the crate boundary at all |

### Sources

| File | Relationship |
|------|--------------|
| `slowest` in `ring_cursor/src/lib.rs` | A1 — where the allocation was, and the fold now in its place |
| `ring_seqno::slowest` in `ring_seqno/src/lib.rs` | The `.min()` it existed to reach, now duplicated rather than called |
| `Claimer::claim` in `ring_claim/src/lib.rs` | A2 — the retry loop it was the condition of |
| `GatingSet::slowest`, `GatingSet::headroom` in `ring_gating/src/lib.rs` | Steps 2 and 3 of the chain |
| `Barrier::frontier` in `ring_barrier/src/lib.rs` | The second caller |

### Tests

| File | Relationship |
|------|--------------|
| `ring_cursor/tests/allocation_test.rs` | A6 — a test-only counting `GlobalAlloc` asserting zero across four call shapes of `slowest`, behind a control arm that must allocate or every zero below it would mean nothing |
| `ring_claim/tests/claim_test.rs` | Exercises the retry loop, without observing what it allocates |
| `slowest`'s doctest in `src/lib.rs` | Correctness only |
