# algorithm

What each method of `RingStats` executes, as distinct from what it is documented to
deliver. The crate holds eighteen atomic operations and sixteen public methods — eleven
and fourteen when these findings were written — and the interesting number is the
difference: ten methods are exactly one operation, and four assemble several into a
result the counter set was never simultaneously in.

The two instances here take those two halves. The first is a census — every
`fetch_add`, `load` and `store` in the file, and the observation that a crate whose
production path has no branch, no loop and no retry has no algorithm left to be wrong
about. The second follows the one composition whose seam changes the returned value
rather than merely the moment it describes, and finds that the error is
one-directional, that `saturating_sub` removes the only value a reader could have
found suspicious, and that the family's one recorded bug in this method was caught
because it failed in the opposite direction.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_eleven_operations_and_three_compositions.md) | Eleven Operations, and the Three That Are Not One | Every atomic site in the crate, and the methods that issue more than one — three when written, four now |
| [002](002_in_flight_subtracts_two_moments.md) | `in_flight` Subtracts One Moment From Another | The direction the seam fails in, measured, and why that direction has no tell |

## A Crate With Nothing to Get Wrong

Five recorders, five readers, one reset. Each recorder is a `fetch_add` on one
counter; each reader is a `load` of one counter; the two methods carrying a `match`
are selecting a field from a three-variant enum, not deciding anything about the
ring. There is no compare-exchange, no retry loop, no arithmetic on a value read
back, and no `unsafe`. A value handed to a recorder is the value the counter holds
when the instruction retires.

That is the crate's whole claim to being cheap enough to leave on permanently, and
it holds per call. What it does not cover is the layout those operations land
in — seven contended atomics packed into 56 bytes, measured at roughly 2.9× the cost
of the same counters on their own cache lines
([`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md)).

## The Compositions, and Which One Bites

`dropped_total` is three loads folded over `OverflowPolicy::ALL`; `in_flight` is two
loads and a subtraction; `reset` is seven stores from one `for` statement; `snapshot`
is seven loads into seven locals and two fields derived from them. Each is a reading
or an action over the whole set, and each is a sequence over its members.

They do not fail equally. `dropped_total` cannot be caught out by magnitude, because
monotone counters put any interleaved sum between its first and last load — the tear
shows up in the per-policy breakdown instead. `reset` leaves an externally observable
window in which some counters are cleared and others are not, rare but reproducible.
`in_flight` is the one where the seam lands directly in the answer, always in the
same direction, and where the floor at zero makes the wrong answer identical to the
healthy one. `snapshot` is the newest and the only one whose own doc opens by saying
which of the two it is — not atomic, internally consistent — which is the sentence the
other three were each found to be missing.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- eighteen atomic operations, sixteen public methods --'
printf '    fetch_add %-4s load %-4s store %-4s pub fn %s\n' \
  "$( command grep -c 'fetch_add(' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '\.load(' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '\.store(' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '^  pub \(const \)\?fn ' ring_stats/src/lib.rs || true )"
echo '  -- the four that compose --'
command grep '^  pub fn dropped_total\|^  pub fn in_flight\|^  pub fn snapshot\|^  pub fn reset' ring_stats/src/lib.rs
echo '  -- and who calls the one that subtracts, doc comments excluded --'
command grep -rn '[^_]in_flight()' --include=*.rs */src/ */tests/ | command grep -v '///' | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST1 | `ring_stats` | n/a — observation | Ten public methods compile to one atomic operation and a move — ten of fourteen when this was written, ten of sixteen since `snapshot` and `checked_in_flight` joined the methods that are not one operation — with no loop, no retry and no arithmetic on a value read back, and the two `match` arms select a field rather than decide anything, so the production path has no algorithm of its own and every question worth asking is about the compositions and the layout beneath them |
| ST2 | `ring_stats` | n/a — doc gap | `dropped_total`, `in_flight` and `reset` issue three, two and seven atomic operations, and each is documented as a single reading or action over the whole set — while the module comment's ordering rationale ("a stats read is a diagnostic, never a synchronisation point") is true of the ten single-operation methods and silent about these three; `snapshot`, added later, issues seven more and is the one composition whose own doc opens by naming the count and denying atomicity |
| ST3 | `ring_stats` | **latent hazard** | `in_flight` loads `claimed` before `published` and both climb, so the result is understated by exactly the traffic passing through the window and can never be overstated — measured, a fixed leak of 8 read as fewer than 8 in 13,099 of 2,000,000 samples and as *zero* in 27, while `saturating_sub` floors the wrong answer at the value a healthy ring reports |
| ST4 | `ring_stats` | **misleading doc** | The one `in_flight` bug the family caught — 240 leaked slots reported on a 16-slot ring, written up in `ring_bench`'s test file — announced itself by exceeding a known bound, which the under-reporting direction cannot do; all 16 call sites are tests, none production, and the only assertion made under contention still reads after every producer has joined |
