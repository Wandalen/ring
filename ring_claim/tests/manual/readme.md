# ring_claim manual testing plan

`tests/claim_test.rs` covers the claim half of the handshake and the
guarantee that no two producers share a sequence. Its sequential half is
exhaustive: `overlap_is_symmetric_and_detects_every_shared_sequence` checks
every range pair in its grid against a from-first-principles definition. Its
concurrent half asserts that the grants made across several threads partition
the sequences with no gap and no overlap.

The gap automation leaves here is different in kind from the other crates'.
Nothing here is unobservable. The problem is that a concurrency test which
passes tells you almost nothing on its own. A test that spawns threads and
asserts a partition passes just as readily against an implementation with a race
that did not happen to fire this run. So the first check below does not read the
source. It runs the suite against a deliberately broken implementation, to
establish that the suite can fail.

Run from the workspace root. The code-line filter is `^[[:space:]]*//`, which
drops `///`, `//!` and plain `//` alike. This crate's module documentation
discusses `fetch_add` at length in prose, and a filter keeping ordinary comments
would report that argument as an implementation.

## C1. The suite detects the bug it is written against

**This is a mutation check: it edits the source, runs the test, and restores.**
Take a copy first; the restore is the whole procedure, not a formality.

The module documentation argues that `fetch_add` claiming is wrong because the
gate check and the advance are two steps with a window between them. That
argument is worth exactly as much as the suite's ability to notice.
`fetch_add` produces no duplicate grants (the add is atomic, so every producer
still gets a distinct start), so the *only* symptom is a grant that runs past
the gate. One test asserts that, and this check confirms it fires.

```bash
cp ring_claim/src/lib.rs /tmp/-claim_orig.rs

# Replace the CAS loop in `claim` with the check-then-add form:
#   let current = self.claimed();
#   if count > self.consumers.headroom( current ) { return Err( RingError::Full ); }
#   let start = self.cursor.fetch_add( count as u64, CLAIM_SUCCESS );
#   Ok( Claim::new( start, count ) )

for i in 1 2 3 4 5; do
  cargo nextest run -p ring_claim --all-features \
    -E 'test(no_grant_ever_passes_the_limit_under_contention)' >/dev/null 2>&1
  echo "run $i exit $?"
done

cp /tmp/-claim_orig.rs ring_claim/src/lib.rs
cargo nextest run -p ring_claim --all-features
```

**Expected:** `exit 100` on **every** mutated run, then every test passes after
the restore. Anything less than every run means the test is probabilistic
enough that a real regression could ship green. The consumer thread's
`yield_now` is what makes every run fail. Without it some mutated runs pass, so
the yield is required rather than cosmetic. If a future edit drops it, this
check catches the resulting loss of sensitivity.

## C2. Only compare-exchange ever moves the cursor

The structural form of what C1 checks behaviourally, and much cheaper to run.

```bash
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -nE "fetch_add|compare_exchange|\.store\("
```

**Expected:** only `compare_exchange` hits, one in `claim` and one in
`claim_up_to`. No `fetch_add` and no `store`. A producer cursor that can be
assigned rather than exchanged can be moved backwards, and moving it backwards
hands out sequences twice.

## C3. The gate is the loop condition, not a value computed once

The subtler version of the same defect, and the one a CAS loop can still have. A
`headroom` computed once before the loop and reused across retries is stale by
definition, since the retry only happens because another producer moved the
cursor.

```bash
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -n -A 14 "let mut current = self.claimed();"
```

**Expected:** in both functions, the line immediately after `let mut current` is
a `while` whose condition calls `self.consumers.headroom( current )`. The gate
*is* the loop condition, so the loop re-evaluates it on every retry, and there is
no way to write the stale-headroom bug without deleting the `while`.

`claim` reads `while count <= self.consumers.headroom( current )`; `claim_up_to`
reads `while let granted @ 1.. = max.min( self.consumers.headroom( current ) )`,
which binds the grant and gates on it in one expression. That binding form is
the only slightly unusual line in the crate, and it earns its place. The plain
alternative computes `max.min( headroom )` once before the loop and again in the
retry arm, and two copies of a gate is how one of them stops being updated.

## C4. A claim cannot be silently dropped, and does not release on drop

Two halves of one decision. `#[must_use]` makes the common accident a compiler
error. The absence of `Drop` is the deliberate part. Releasing is not possible,
because another producer may already hold the range beyond this one, and
rewinding the cursor to "give back" a claim would grant those sequences twice.

```bash
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -nE "must_use = " 
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -nE "impl.*Drop"
```

**Expected:** one `must_use` **with a message** on `Claim` itself. The message
tells the next reader why, since a bare `#[must_use]` warning says only that a
value was unused. And **no output at all** from the second command. A `Drop`
impl here would look like careful resource handling and would be a correctness
bug.

## C5. Every declared dependency is used

```bash
comm -23 \
  <( grep -E "^ring_" ring_claim/Cargo.toml | cut -d' ' -f1 | sort ) \
  <( grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output. The crate does not depend on `ring_seqno`. Every piece
of sequence arithmetic claiming does is either `ring_types::Seq`'s own or
already inside `ring_gating`'s `headroom`.

## C6. The concurrency tests have enough contention to mean anything

A run in which the threads happened not to interleave passes every
multi-threaded test here trivially. Thread count and iteration count are the
only things making that unlikely, and they are the first numbers a future edit
reduces to make the suite faster.

```bash
grep -nE "const (PRODUCERS|PER_PRODUCER|CLAIMS_EACH|CALLS_EACH|RELEASES|WIDTH)" \
  ring_claim/tests/claim_test.rs
grep -c "thread::scope" ring_claim/tests/claim_test.rs
```

**Expected:** a `thread::scope` block in every contention test, and constants
no smaller than `PRODUCERS = 4` / `PER_PRODUCER = 2_000` / `CLAIMS_EACH = 500` /
`CALLS_EACH = 500` / `RELEASES = 4_000`.
The whole suite runs in well under a second at these numbers, so there is no
performance argument for cutting them. There will be a tidiness one, and this
check records that they were chosen rather than defaulted.

`claim_up_to_under_contention_loses_no_sequences_either` is the only test that
can take `claim_up_to`'s CAS retry arm, because every other test of it is
single-threaded. Without it the crate has two CAS loops and contention coverage
of only one, which is the shape of gap a green suite hides.

---

## Run Record

| Date | Checks | Result |
| ---- | ------ | ------ |
| 2026-08-28 | C1–C6 | 6/6 as expected. C1 measured 8/8 mutated failures, 5/5 clean after restore |
| 2026-08-28 | C1–C6 | 6/6 after C3 was rewritten for the `while`-condition gate that replaced both bare `loop`s |
