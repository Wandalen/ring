# ring_consume — manual testing plan

`tests/consume_test.rs` covers the consumer half of
`docs/feature/170_claim_publish_available_commit_handshake.md` — 21 tests over
`available` and `commit`, including an exhaustive sweep of every
(position, candidate) pair up to the frontier.

The gap automation leaves here is that this crate's central risk is a *wiring*
mistake, not a logic one. A `Consumer` whose cursor is private compiles, runs,
passes all 21 tests, and gates nothing — the producer would be reading a cursor
nobody ever advances and would lap the consumer on the first pass. The type
system cannot express "this borrow came from the producer's gating set", so the
checks below read for the shape that makes the mistake impossible instead.

Run from the workspace root. The code-line filter is `^[[:space:]]*//`, which
drops `///`, `//!` and plain `//` alike: this crate's module documentation
describes the private-cursor mistake in detail, and a filter keeping ordinary
comments would report that description as an implementation.

## N1 — the cursor is borrowed, never owned

The single structural fact the whole crate rests on.

```bash
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs \
  | grep -nE "cursor : |PaddedCursor" 
```

**Expected:** every occurrence is behind a `&` or `&'a` — the struct field
`cursor : &'a PaddedCursor`, `new`'s parameter, and `cursor()`'s return. Not one
bare `PaddedCursor` by value.

A field declared `cursor : PaddedCursor` is the bug this crate is shaped to
prevent, and it is a *quieter* bug than it sounds: the crate still compiles, the
suite still passes, and the ring silently overwrites unread slots under load.
There is no test that can catch it, because from inside this crate the two
versions are indistinguishable — only the caller can tell, which is why
`ring_publish/tests/handshake_test.rs` asserts the wiring with `ptr::eq` rather
than trusting it.

## N2 — commit is the only thing that moves the cursor

```bash
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs \
  | grep -nE "\.store\(|fetch_add|compare_exchange"
```

**Expected:** exactly two `store` calls, one in `commit` and one in
`commit_available`, both at `COMMIT`. No `fetch_add`, no `compare_exchange`, and
nothing in `available`, `available_up_to`, `position` or `new`.

A plain `store` is correct here and would be wrong in `ring_claim`: this cursor
has exactly one writer. The moment a second consumer shares it, this check is
what says the store has to become an exchange — and `Consumer::new`'s own
documentation is what says the cursor is not reset on construction, which is
what makes it safe to build one around a position the producer is already
gating on.

## N3 — commit refuses in both directions

The two commits that must be refused, and the reason the suite tests both: a
past-available commit frees slots that were never read, and a backwards commit
re-reads slots the producer has already been cleared to reuse. Neither is caught
by a suite that only ever commits exactly what `available` returned.

```bash
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs \
  | grep -n -A 6 "pub fn commit( "
```

**Expected:** a single guard testing **both** bounds —
`through < run.start() || through > run.end()` — before the store, not after,
and not two separate `if`s with a store between them. Both comparisons must be
present; dropping either half leaves a version that passes every test which only
commits what it was offered.

## N4 — the frontier comes from the barrier, and the crate does no arithmetic of its own

```bash
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs \
  | grep -nE "ring_seqno::|barrier\.|frontier|[+-] 1|wrapping_"
```

**Expected:** two adjacent lines and nothing else — a `.frontier()` call and the
`.map_or( 0, | frontier | ring_seqno::pending( frontier, position ) )` that
consumes it, both inside `available`. No hand-rolled sequence arithmetic
anywhere: no `+ 1`, no `- 1`, no `wrapping_*`. Distances between sequences are
`ring_seqno`'s subject; a second copy here is how the two disagree at a wrap.

Note what `available` does with `None`: `map_or( 0, ... )`. A barrier over no
dependencies makes *nothing* readable, which is the opposite of what the same
`None` means to `ring_gating` — see `ring_cursor::slowest`'s own documentation
on why the fold returns `Option` rather than picking one of the two answers.

## N5 — every declared dependency is actually used

```bash
comm -23 \
  <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_consume/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output. Four dependencies, all real — unlike `ring_claim` and
`ring_publish`, this crate genuinely needs `ring_seqno`, because `available` is a
distance computation and that is exactly what `ring_seqno::pending` is for.

## N6 — the exhaustive sweep builds a fresh consumer per case

The sweep in `the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends`
is the crate's strongest test and was, in its first draft, self-defeating: it reused one
consumer across the inner loop, and since an accepted commit *moves the
position*, later iterations were silently testing a different case than the one
they asserted.

```bash
grep -n -B 6 "let accepted = consumer.commit" ring_consume/tests/consume_test.rs
```

**Expected:** a `Consumer::new(...)` inside the inner loop, immediately above the
setup commit — one per (position, candidate) pair, not one per row. And the
comment saying why, because the construction looks redundant and the next reader
will want to hoist it.

---

## Run Record

| Date | Checks | Result |
| ---- | ------ | ------ |
| 2026-08-28 | N1–N6 | 6/6 as expected — N2 measured 2 stores, N5 clean, N6 confirmed fresh-per-case after the first-draft defect was fixed |
