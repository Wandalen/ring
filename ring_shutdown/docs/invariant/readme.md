# Invariant Doc Definition

### Scope

- **Purpose**: State what must hold for the crate's guarantees to mean anything, with the measurement or the structural argument for each.
- **Responsibility**: Two invariants: one flag in the family, and a drain that terminates.
- **In Scope**: Liveness-flag uniqueness across the family; drain termination.
- **Out of Scope**: Ordering invariants inside a backend (→ `ring_spsc`, `ring_mpsc`).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Exactly One Liveness Flag](001_exactly_one_liveness_flag.md) | Why `ring_core` has no `is_closed` and this crate has the only `AtomicBool` | 🔄 |
| 002 | [A Drain Terminates Because a Close Preceded It](002_drain_terminates_because_close_preceded_it.md) | The ordering the type system enforces, and the single case where enforcing it is not enough | 🔄 |

**The two invariants are the crate's two halves, and they fail in opposite
ways.** `001` is a negative about state — one flag, no copies — and its
enforcement is a search for something that must not exist. `002` is a positive
about ordering — a drain follows a close — and its enforcement is the type
system, which is supposed to make searching unnecessary.

They are separate documents because a reader needs `001` to hold before `002`
means anything: a drain ordered against a *cached* flag would satisfy the type
argument and still hang. Stating them together would let the second borrow the
first's confidence, which is exactly the mistake the four findings below record.

The findings pair off across that split, and both pairs say the same thing about
enforcement. `001`'s are about **an index and a search that are narrower than
their own prose**: a cross-reference naming two readers of a flag six functions
read, one of them with no callers at all (SD21), and a check for `AtomicBool`
guarding a sentence that forbids a copy *"cached or otherwise"*, in a family
that already carries four other atomic types and a plain `bool` field (SD22).
`002`'s are about **a guarantee asserted more strongly than the code supports**:
an Enforcement Mechanism whose "there is none" is contradicted by a test three
files away (SD23), and a premise table whose failing row has exactly one witness,
built from a method under deliberation for removal, which stops before the
failure it exists to show (SD24).

Read together: every one of the four is a place where the *strength of the
statement* outran the *reach of its check*. Neither invariant is false. Both are
enforced by something narrower than what they say.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/invariant
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each states its invariant:    %s\n' "$( command grep -lc '^### Invariant Statement' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each names its enforcement:   %s\n' "$( command grep -lc '^### Enforcement Mechanism' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'enforced by a manual probe:   %s\n' "$( command grep -lc 'tests/manual' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'enforced by the type system:  %s\n' "$( command grep -lc 'takes .self. by value' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'test fns in the suite:        %s\n' "$( cd ../../..; command grep -c '^fn [a-z_]*()' ring_shutdown/tests/shutdown_test.rs )"
printf 'manual probes standing in:    %s\n' "$( cd ../../..; command grep -c '^## D[0-9]' ring_shutdown/tests/manual/readme.md )"
printf 'the flags the family carries: %s\n' "$( cd ../../..; command grep -rlc 'AtomicBool' ring_*/src --include='*.rs' | wc -l )"
printf 'functions reading that flag:  %s\n' "$( cd ../../..; awk '/\.is_closed\(\)/&&!/^ *\/\/\//{ n++ } END{ print n+0 }' ring_shutdown/src/lib.rs )"
printf 'ways to mint a drain token:   %s\n' "$( cd ../../..; command grep -c 'pub fn close(' ring_shutdown/src/lib.rs )"
printf 'ways to spend one:            %s\n' "$( cd ../../..; command grep -c 'pub fn reopen(' ring_shutdown/src/lib.rs )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
each states its invariant:    2
each names its enforcement:   2
enforced by a manual probe:   1
enforced by the type system:  1
test fns in the suite:        20
manual probes standing in:    4
the flags the family carries: 1
functions reading that flag:  6
ways to mint a drain token:   1
ways to spend one:            1
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD21 | the document's own index of who reads the flag names a dead function and omits the live one | **misleading doc** | [`001`](001_exactly_one_liveness_flag.md)'s APIs row reads *"`is_closed` and `admit`, the only readers of the one flag"*, while six functions in `src/lib.rs` call `is_closed()` outside a doctest — `admit` (130), `Guarded::try_push` (434), `Guarded::try_push_batch` (448), `Guarded::is_blocked` (476), `wait_for_close` (575), `for_space_or_close` (603) — and the selection is worse than arbitrary: `admit` has zero real call sites (both its mentions in `src/lib.rs` are doctest lines) while `Guarded::try_push`, the operation the crate exists to make safe and the one [`api/001`](../api/001_shutdown_surface.md) grades **construction** on the strength of its flag read, goes unnamed; the singular *load* is real (`closed.load` appears once, line 82) but there are two *stores* — `close` (103) and `reopen` (317), the second reaching through `self.shutdown.closed` into a private field from another type — and neither the invariant statement nor the enforcement section mentions the flag has a second writer. |
| SD22 | the enforcement mechanism searched for one spelling of the thing it forbids and now searches all four | **latent hazard** | [`001`](001_exactly_one_liveness_flag.md) forbids a second flag in the strongest available terms — *"No handle, producer, consumer, or ring carries a copy, cached or otherwise"* — and enforces it by grepping the literal string `AtomicBool`, which a plain `bool` field, a `Cell< bool >`, an `AtomicU8` tri-state or a sentinel in an existing counter all pass while being exactly what the sentence prohibits; this is not hypothetical, since four family crates already carry non-boolean atomics (`ring_atomic`, `ring_bench`, `ring_core`, `ring_stats`) and `ring_trace/src/lib.rs` already carries a plain `bool` field, so the forbidden shape is present today in an innocent role the check cannot distinguish — and the asymmetry is the finding: the document argues at length that the *second* command's two-hit expectation is a reading rather than a count, and `tests/manual/readme.md`'s D3 names the trap by number, while nothing anywhere asked whether the first command's search term matched the invariant's own words; the enforcement command now searches `AtomicBool`, `Cell< bool >`, plain `bool` fields and `AtomicU*` across `ring_*/src` and reports how many hits are named for liveness outside `ring_shutdown` (0), and `Regenerate` verifies the four shapes are covered instead of asserting a constant. |
| SD23 | the enforcement claim is an absolute and the crate's own suite builds the counterexample | **wrong doc** | [`002`](002_drain_terminates_because_close_preceded_it.md)'s Enforcement Mechanism states *"`Stopped::reopen` takes `self` by value, so there is none that reaches a drain **after** a reopen either"*, where "none" means no expression anywhere; the first clause holds and the conclusion does not follow, because `close( &self )` mints without limit and `reopen` consumes only its receiver, so `close_is_idempotent_and_admit_reports_it` binds two tokens, spends one, and runs its last three lines with `first` live on a reopened ring where `first.drain_all( … )` compiles and — by this document's own premise table, *"the loop never sees an empty batch"* — hangs; the appearance *here* is the most serious of the three sightings (cf. [`decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md), [`api/002`](../api/002_the_surface_the_table_does_not_grade.md)) because an invariant's Enforcement Mechanism is what other documents cite when they stop reasoning, and three do: [`algorithm/001`](../algorithm/001_drain_to_empty.md), [`type/001`](../type/001_stopped_proof_token.md), [`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md). |
| SD24 | the failing row's only witness is built from a method under deliberation for removal, and stops one step short | n/a — coverage | Row 3 of [`002`](002_drain_terminates_because_close_preceded_it.md)'s premise table — *"A raw `Producer` exists, and is publishing"* — is what the invariant is about, and exactly one test reaches it: `an_unguarded_producer_publishes_straight_through_a_close`, whose raw producer comes from `guarded.into_inner()`, the subject of an open decision naming it 42 times across four option rows that cost out reasons, guarantees and ergonomics and mention a test zero times, one of which proposes removing it outright; separately the test stops before the point — it asserts the cause (`try_push` returns `Ok` after a close, the record arrives, `try_recv` reads it back) and makes zero calls to `drain_all` or `discard_all`, so the documented consequence is left entirely to prose on the stated grounds that *"a test that hangs does not report a failure, it reports nothing"*, which is true of the naive test and not of the claim: draining a fixed number of batches against a live producer and asserting the ring is still non-empty fails loudly in milliseconds and never hangs. |
