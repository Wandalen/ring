# pitfall

Two changes that a competent reviewer would approve, and that every automatic
check in this crate would let through.

### Overview Table

| ID | Name | The change | What survives it |
|----|------|------------|------------------|
| 001 | [The Obvious Implementation Forks the Constant](001_the_obvious_implementation_forks_the_constant.md) | `#[ repr( align( 64 ) ) ]` in place of `CacheAligned` | All five layout tests, clippy, every doc example |
| 002 | [A Reading That Consults One Cursor](002_a_reading_that_consults_one_cursor.md) | A two-cursor reading that reads only the producer | Every doc example, and any test that leaves the consumer where the constructor put it |

### What They Have in Common

**Both produce identical observable values today.** 001 yields the same layout by
a different route; 002 yields the same numbers while the consumer sits at
`Seq::ZERO`. Neither is a wrong answer — each is a right answer arrived at in a
way that stops being right later, under a change nobody will connect to it.

That is why the countermeasures are structural rather than behavioural. The
checks that see these are M1 (grep the source for a literal `64`) and M4 (count
the loads in three method bodies) — both read the *shape* of the code, because
its *output* is indistinguishable from the correct version.

Both manual checks have a recorded history of being wrong themselves: M4
miscounted with `grep -c` and manufactured a finding; M5, next door, still
carries a stale expected count. **A structural check is only as good as its
instrument**, and these instruments are shell one-liners maintained by hand.

### Evidence That These Are Not Hypothetical

001 has already happened three times in the family — with the gating ordering
rather than the cache line, in `ring_debug`, `ring_batch`, and `ring_mpsc`. Two
of the three wrote a paragraph explaining that they were matching
`ring_cursor`'s decision, and then restated the value instead of importing it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the fork the family already made --'
command grep -r 'abs_diff( consume )' ring_mpsc/src/lib.rs | sed -E 's/^/  /'
echo '  -- and the readings here, every one of which consults both cursors --'
command grep -cE 'self\.producer\.load\( GATING \), self\.consumer\.load\( GATING \)' ring_cursor/src/lib.rs
echo '  -- control: the one reading that consults a slice instead --'
command grep 'cursors.iter().map' ring_cursor/src/lib.rs
```

Live output:

```
  -- the fork the family already made --
      claim.abs_diff( consume ) >= 64
  -- and the readings here, every one of which consults both cursors --
3
  -- control: the one reading that consults a slice instead --
  cursors.iter().map( | c | c.load( GATING ) ).min()
```

**The first arm's line number is stripped on purpose.** It points into
`ring_mpsc`, a crate under active edit, and a citation by line into a moving file
goes stale on changes that have nothing to do with the finding — which is
[`../integration/002`](../integration/002_who_reads_a_cursor.md) CU20, recorded
after four of `ring_align`'s recipes broke in exactly that way.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU41 | `ring_mpsc` | n/a — duplication | The fork this pitfall predicts already exists one crate over: `claim.abs_diff( consume ) >= 64`, a second copy of the cache-line constant under a method name shared with `ring_align`'s function. The failure mode is documented from an instance rather than from imagination |
| CU42 | `cargo +nightly udeps` | n/a — unenforced | The second-order detector for the fork is `udeps` reporting `ring_align` unused after the wrapper is replaced by a literal. It runs at verification level 4, which ordinary work does not reach, and only if the author leaves the now-unused dependency declared |
| CU43 | The three readings | n/a — observation | All three consult both cursors, and the one reading that does not — `slowest` — consults a slice instead. So the negative case this pitfall describes does not occur anywhere in this crate, and the document says so from a count rather than from a survey |
| CU44 | `ring_mpsc`'s two cursors | **latent hazard** | It holds a claimer's cursor and the ring's consumer cursor — two cursors from two owners rather than a `CursorPair` — which is exactly the shape this pitfall describes. Nothing checks that a consumer holding two loose cursors is not simply the wrong shape for the pair it is emulating |
