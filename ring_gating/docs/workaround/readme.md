# workaround

External constraints `ring_gating` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

### Overview Table

| ID | Name | Works around | Deletable when |
|----|------|--------------|----------------|
| 001 | [The Manual Check That Named a Foreign Method](001_the_manual_check_that_names_a_foreign_method.md) | No language or lint expresses "this crate must not reimplement its dependency's fold" | Not while `ring_gating` and `ring_barrier` share a cursor slice — make it loud in CI instead |
| 002 | [The Multi-Consumer Path No Ring Uses](002_the_multi_consumer_path_no_ring_uses.md) | A specified plurality no ring constructs, paid for on every gating read until `b7e075ca` and held for the set's lifetime since | A multi-consumer ring lands, or the family rules one out and `ring_mpsc` moves to `CursorPair` |

### Both Are Costs of Being General Early

001 absorbs a property the compiler cannot check, and pays in manual greps whose
success is indistinguishable from a broken pattern. 002 absorbs a shape nothing
constructs, and paid in an allocation per read until `b7e075ca` removed it two
crates down — leaving the shape, and its own record that removing the cost would
not resolve the question, intact.

Neither is a defect. In both cases the crate is doing the specified thing
correctly, and the cost lands on a caller that does not yet need the generality —
which is exactly the kind of thing that stops looking like a choice once the
reason for it is forgotten.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT55 | The manual checks | n/a — coverage | Three of the seven expect no output at all, and a fourth expects none from its second command — an absence check passes identically whether the code is clean or the pattern is wrong |
| GT56 | M1's note | n/a — doc gap | It justified a deliberate gap in its pattern by naming `frontier`, which is `ring_barrier`'s method and appears zero times in this crate, and `headroom`, whose `map_or` the pattern would not have matched — the one method it protects, `limit`, went unnamed |
| GT57 | The foreign method name | n/a — drift | The check names a method on a type this crate does not own, so it goes stale on a rename in a crate it cannot see. A rename leaves the check reading plausibly and matching nothing — the same failure shape as a corpus recipe quoting line numbers into a file someone else is editing |
| GT58 | The family's two rings | n/a — observation | Each has exactly one consumer and they gate through different mechanisms: `ring_spsc` reads a `CursorPair` in two loads, `ring_mpsc` folds a one-element `Vec` it holds for the set's whole life — a per-read allocation until `b7e075ca`, a pointer chase since |
| GT59 | `Claimer` plus a one-consumer set | n/a — observation | Together they hold exactly `CursorPair`'s three fields, one heap allocation and one lifetime parameter apart — so the multi-consumer machinery, at the only width anything ships with, is a more expensive spelling of a type the family already has |
| GT60 | The multi-consumer path | n/a — coverage | No shipping ring uses it. `ring_spsc` constructs no `GatingSet` at all and `ring_mpsc` constructs one with `consumers = 1`, so the fold, the allocation and the minimum-across-the-set rule are exercised only by tests |
| GT61 | The one production cursor read | n/a — observation | `ring_mpsc::consumer_cursor` calls `.cursor( 0 ).expect( "a gating set built with one consumer has cursor 0" )`. The single-consumer assumption is recorded in an `expect` message rather than in a type, and this is the only place in shipping code where a cursor is taken out of a set |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the manual checks, and how many expect nothing --'
command grep -cE '^## M[0-9]' ring_gating/tests/manual/readme.md || true
command grep -cE '^\*\*Expected:\*\* (\*\*)?no output' ring_gating/tests/manual/readme.md || true
echo '  -- every production construction of a GatingSet, and the one cursor read --'
command grep -r 'GatingSet::new' --include=*.rs */src/ | command grep -v '//' | sed 's|ring/||'
command grep -r '\.cursor( 0 )' --include=*.rs */src/ | command grep -v '///' | sed 's|ring/||'
echo '  -- control: the same constructor in tests --'
command grep -rl 'GatingSet::new' --include=*.rs */tests/ | sed 's|ring/||' | LC_ALL=C sort | tr '\n' ' '
echo
```

Live output:

```
  -- the manual checks, and how many expect nothing --
7
3
  -- every production construction of a GatingSet, and the one cursor read --
ring_mpsc/src/lib.rs:      consumers : GatingSet::new( capacity, 1 ),
ring_mpsc/src/lib.rs:      .cursor( 0 )
  -- control: the same constructor in tests --
ring_barrier/tests/barrier_test.rs ring_claim/tests/allocation_test.rs ring_claim/tests/claim_test.rs ring_gating/tests/gating_test.rs ring_publish/tests/handshake_test.rs 
```

**Seven manual checks, three of which pass on an empty result; one production
construction; one production cursor read.** The control is what makes the middle
arms a finding rather than a grep that missed — the constructor is exercised
widely in tests and exactly once by a shipping ring, with `consumers = 1`, the
count the multi-consumer machinery is not needed for.
