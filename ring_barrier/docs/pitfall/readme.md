# pitfall

Two mistakes, both in `wait_for`'s ending, both of which type-check and neither
of which a green suite would report. Each is guarded by a *structural* check —
a grep over the source — rather than by the behavioural test, and each records
why the behavioural test is not enough.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Two Empty Answers Look Like a Bug](001_the_two_empty_answers_look_like_a_bug.md) | Why an empty barrier and an empty gating set answer oppositely, the three meanings of *empty* here, and BR6 |
| 002 | [Returning the Request Instead of the Frontier](002_returning_the_request_instead_of_the_frontier.md) | The `from + count` mutation, and the one call site in six that catches it |

### Both Mutations, Side by Side

| | 001 — the empty default | 002 — the return value |
|--|-------------------------|------------------------|
| The edit | `map_or( 0, … )` → a non-zero default, or `unwrap_or( Seq::ZERO )` | `self.frontier().ok_or( … )` → `Ok( Seq( from.0 + count ) )` |
| Type-checks | ✔ | ✔ |
| Looks like | aligning two crates that disagree | removing a redundant second read |
| Caught behaviourally by | `an_empty_barrier_and_an_empty_gating_set_answer_oppositely` | 1 of 6 `wait_for` call sites |
| Guarded structurally by | B3's `map_or\|ok_or\|unwrap_or` grep — expect exactly 2 hits | B5's `-A 5` window on `pub fn wait_for` |
| What a green suite would show | a pass | a pass, 512 iterations slower |

### Run Both Checks

```sh
cd "$(git rev-parse --show-toplevel)"

# B3 — exactly two hits, and no unwrap_or
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -nE "map_or|ok_or|unwrap_or"

# B5 — the body must return self.frontier(), with no arithmetic on from/count
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -n -A 5 "pub fn wait_for"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR6 | `ring_barrier` | n/a — inconsistency | `Barrier::over( &[] ).admits( ZERO, 0 )` is `true` while `wait_for( ZERO, 0, … )` is `Err( Empty )` — the one input class where the two phases disagree; untested, undocumented, unguarded |
| BR46 | `ring_barrier` | n/a — unenforced | The assertion that makes the empty-set asymmetry deliberate rather than accidental is only writable because `ring_gating` is a dev-dependency; remove the entry and the library is unchanged while the one statement that the asymmetry is intentional stops being checkable |
| BR47 | `ring_barrier` | n/a — coverage | One test stands between the crate and a change that would compile, pass everything else, and silently cap every batch at the size its consumer asked for — both candidates are a `Seq`, and the correct one is a single expression a refactor toward *return what was asked for* would replace without a second thought |
