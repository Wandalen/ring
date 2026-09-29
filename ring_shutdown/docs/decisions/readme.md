# Decisions Doc Definition

### Scope

- **Purpose**: Open questions this crate cannot settle on its own evidence — each with its options, their costs, and the measurement that would decide.
- **Responsibility**: The question, its options, what would settle it, and who owns it.
- **In Scope**: Whether `Guarded::into_inner` should exist; whether a `Stopped` token should be unique.
- **Out of Scope**: Settled choices, which are documented where they take effect rather than filed here; the guarantee column those choices affect (→ [`../api/001`](../api/001_shutdown_surface.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Should `into_inner` Exist](001_should_into_inner_exist.md) | The escape hatch that reopens the crate's central limitation on purpose | ❓ |
| 002 | [Should a `Stopped` Token Be Unique](002_should_a_stopped_token_be_unique.md) | `close( &self )` mints without limit; `reopen( self )` spends one | ❓ |

**Both questions are about the same thing from opposite ends: what a caller can
still reach after the type system was supposed to have stopped them.** `001`
asks about the documented exit — the method that hands a raw producer back.
`002` asks about the undocumented supply — the constructor that mints proof
tokens without limit. Neither can be answered from this crate's own callers,
because this crate has none: nothing outside it holds a `Guarded`, and only one
other crate binds a `Stopped`.

They are filed separately because they have different owners in practice —
`001` is settled by counting call sites, `002` by changing a signature that four
other declarations depend on — and together because deciding either alone
decides the wrong question. Removing `into_inner` while `Guarded::shutdown`
remains leaves the guard exactly as escapable; making tokens unique while
`into_inner` remains leaves the producer exactly as unguarded.

The four findings divide along the same seam. **Two are about a decision that
cannot execute as written**: the settling measurement counts an identifier
shared by four unrelated types and so can never report the threshold that
retires the method (SD13), and the options table controls one exit while a
cheaper unlisted one reaches past all four rows (SD14). **Two are about a
guarantee stated more strongly than it holds**: the token's doc comment promises
that a reopen ends it, falsified by the crate's own test three files away
(SD15), and the type withholds `Clone` while an accessor on that same type hands
back a second token from a shared borrow of the first (SD16).

Read together they say something uncomfortable about filing decisions: both of
these documents are well-formed — status, owner, options, costs, a stated
trigger — and both would produce the wrong answer if executed today. Form is not
the hard part.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/decisions
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each declares a status:       %s\n' "$( command grep -hc '^- \*\*Status:\*\*' [0-9][0-9][0-9]_*.md | paste -sd' ' )"
printf 'each declares an owner:       %s\n' "$( command grep -hc '^- \*\*Owner:\*\*' [0-9][0-9][0-9]_*.md | paste -sd' ' )"
printf 'each names its measurement:   %s\n' "$( command grep -lc '^### What Would Settle It' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'crates outside holding Guarded: %s\n' "$( cd ../../..; command grep -rl 'Guarded' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | command grep -vc ring_shutdown || true )"
printf 'crates outside binding Stopped: %s\n' "$( cd ../../..; command grep -rl '= .*\.close()' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | command grep -v ring_shutdown | tr '\n' ' ' )"
printf 'files outside citing either:   %s\n' "$( cd ../../../..; command grep -rl 'should_into_inner_exist\|should_a_stopped_token_be_unique' --include=*.md . | command grep -vc ring_shutdown || true )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
each declares a status:       1 1
each declares an owner:       1 1
each names its measurement:   2
crates outside holding Guarded: 0
crates outside binding Stopped: ring_testkit 
files outside citing either:   0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD13 | the measurement filed to settle a decision cannot distinguish its own subject | **misleading doc** | [`001`](001_should_into_inner_exist.md) defers to `grep -rn 'into_inner'` across every `ring_*` crate and routes to three outcomes on what it returns, but the family declares that identifier on two unrelated types (`ring_align`'s `CacheAligned` and this crate's `Guarded`) and uses it on two more (`std::sync::Mutex` in `ring_bench` and `ring_mpsc`'s tests, `PoisonError` in `ring_trace`), so the command returns thirteen lines from five crates of which exactly one is a call site of the subject; the bullet that retires the method fires on *"Zero non-test call sites"*, a threshold the instrument could not report the day it was filed and would not report after the method was deleted, while the two keep-it branches are reachable on evidence about `ring_align` — and the named trigger has quietly passed, since `ring_factory` already exists and hands out no `Guarded`, with no file outside this crate's `docs/` referencing the decision at all. |
| SD14 | four options constrain one exit while a cheaper one goes unlisted | n/a — doc gap | [`001`](001_should_into_inner_exist.md) states that `into_inner` *"is the only sanctioned route"* from a guaranteed surface back to an unguaranteed one and builds all four options on that premise, but `Guarded::shutdown( &self ) -> &'a Shutdown` is a `const fn` consuming nothing and `Shutdown::close` takes `&self`, so a guard holder reaches `close`, hence a `Stopped`, hence `drain_all`/`discard_all`/`reopen`, while still holding the guard (→ [`api/002`](../api/002_the_surface_the_table_does_not_grade.md)); the concrete consequence is that Option 2 is false as written — *"Remove it → `Guarded` becomes a one-way wrapper"* would leave the guard exactly as escapable — and the fix is not a fifth row, since removing the accessor would break `wait_for_close( guarded.shutdown(), … )`, but the missing sentence saying so. |
| SD15 | the token's doc comment is falsified by the crate's own test | **wrong doc** | `Stopped`'s doc comment claims *"`Stopped::reopen` takes `self` by value: once the ring is open again the token is gone, and a drain written against it no longer compiles"*, and the first clause is true while the second does not follow, because `close( &self )` mints without limit and `reopen` consumes only its own receiver; `tests/shutdown_test.rs`'s `close_is_idempotent_and_admit_reports_it` binds two tokens and spends one, so it ends with a live `Stopped` on a reopened ring, where `first.drain_all( … )` compiles and — per [`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)'s *"the loop never sees an empty batch"* — hangs rather than returning wrong, making the crate's worst documented outcome three lines away, two of them already in the suite. |
| SD16 | the type withholds `Clone` and an accessor on the same type reconstitutes it; the missing scarcity rule is now written down, the receiver change is not | **latent hazard** | `Stopped` derives `Debug` alone where `Wake` derives six and `Refusal` four, which reads as a deliberate refusal to let the token be duplicated, but `Stopped::shutdown( &self ) -> &'a Shutdown` gives any holder `stopped.shutdown().close()` — a second token, from a *shared* borrow that leaves the first alive; the crate itself extracted the pattern and wrote the rule this breaks, [`pattern/001`](../pattern/001_proof_token_orders_two_operations.md)'s third — *"Anything that undoes A consumes the token"*, described there as *"the one most often skipped … the failure only appears when a caller re-enters the pre-A state and then calls B with a stale proof"* — and `ring_shutdown` satisfies it to the letter while producing its failure anyway, because rule 1 (*"the token's only constructor performs A"*) holds without bounding how often the constructor may be called; the general fix belonged in the pattern, which was missing the condition that undoing A must invalidate every outstanding token rather than the one passed to it — it is now `pattern/002`'s rule 4, and `pattern/001` defers to it, while the receiver change stays open as this decision's own subject rather than being made under cover of a finding. |
