# Decision: Should `Guarded::into_inner` Exist

- **Status:** ❓ open
- **Owner:** Deferred until `ring_factory` becomes the first crate to hand out guarded producers

### Scope

- **Purpose**: Record an open question — whether the one sanctioned escape from `Guarded` back to a raw producer should survive — together with what would settle it, rather than resolving it now on a guess.
- **Responsibility**: The question, the three reasons the method exists today, the four options with their costs, and the evidence that would settle it.
- **In Scope**: `Guarded::into_inner` and any replacement for it.
- **Out of Scope**: The trap it re-opens (→ [`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)); the wrapper's own guarantee (→ [`api/001`](../api/001_shutdown_surface.md)).

### Question

`Guarded::into_inner` returns the wrapped `ring_core::Producer`, discarding the
close check. Should it exist at all?

It is the only sanctioned route from a guaranteed surface back to an
unguaranteed one, and every use of it re-opens
[`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md) —
deliberately, which is worse than accidentally, because it looks approved.

### Why It Exists Today

Three reasons, in descending order of how good they are:

1. **A helper with a `Producer` parameter cannot take a `Guarded`.** There is
   no `Deref`, no trait, and no blanket conversion. Without `into_inner` a
   caller holding a `Guarded` cannot call such a helper at all.
2. **The wrapper is not a container the ring knows about.** It borrows a
   producer that `ring_core::Ends::split` created; giving it back is the
   symmetric operation, and its absence would read as an omission.
3. **It made the limitation testable.** `an_unguarded_producer_publishes_straight_through_a_close`
   needs a raw producer from a guarded context to assert the trap as fact.

Reason 3 is circular — the method exists so a test can demonstrate why the
method is dangerous — and should not count toward keeping it.

### Options

| Option | Effect | Cost |
|---|---|---|
| **Keep as is** | Status quo. The escape hatch is documented as a scope exit | The guarantee is opt-out, and opting out is one method call with no ceremony |
| **Remove it** | `Guarded` becomes a one-way wrapper | Reason 1 becomes a hard block: helpers must be rewritten to take `&mut Guarded`, including any outside this family |
| **Keep, but make it consume and re-check** | `into_inner( self ) -> Result< Producer, Self >`, refusing while closed | Half-measure — a caller who unwraps *before* the close still holds a raw producer afterwards, which is the actual failure path |
| **Replace with a scoped `with_raw( &mut self, f )`** | The raw producer never escapes the closure | Cannot express a helper that stores the producer, which is exactly reason 1's case |

### What Would Settle It

**Evidence about reason 1, which is currently hypothetical.** No caller exists
yet: nothing in the family holds a `Guarded`, because nothing yet hands one
out. The question is genuinely undecidable on this crate's own evidence.

The measurement to make once `ring_factory` produces guarded ends is
restricted to crates whose `src`/`tests` mention `Guarded` at all — today that
is `ring_shutdown` alone, so `ring_align`'s unrelated `CacheAligned::into_inner`
and the two `std::into_inner` uses in `ring_bench`/`ring_mpsc`/`ring_trace`
never enter the count, and the branch below can actually return zero once this
crate's own call sites do.

```sh
cd "$(git rev-parse --show-toplevel)"
GUARDED_CRATES=$( command grep -rl 'Guarded' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u )
for c in $GUARDED_CRATES; do command grep -r 'into_inner' $c/src $c/tests --include='*.rs' 2>/dev/null; done | sed 's/:/: /' | sort
```

Live output:

```
ring_shutdown/src/lib.rs:   /// [`Guarded::into_inner`] is the documented one — it consumes the guard and
ring_shutdown/src/lib.rs:   pub fn into_inner( self ) -> Producer< 'a, T >
ring_shutdown/tests/shutdown_test.rs:   let mut raw = guarded.into_inner();
ring_shutdown/tests/shutdown_test.rs: /// This asserts the crate's limitation on purpose. `into_inner` hands back a
```

- **Zero non-test call sites** → reason 1 never materialized. Remove it, and
  rewrite the one test to construct a raw producer directly from `split()`
  rather than by unwrapping.
- **Call sites that immediately re-wrap or use-and-drop** → replace with the
  scoped `with_raw` option, which expresses exactly that shape.
- **Call sites that store the producer** → keep it, and document each one,
  because each is a place the close guarantee genuinely does not hold.

### Why This Is Filed Rather Than Decided

Deciding now means guessing which row of that table the eventual measurement
will produce, and the three rows have different answers. Removing the method
on a guess costs a rewrite if the guess is wrong; keeping it on a guess costs
nothing until the measurement is actually taken — so the cheap direction is to
keep and measure, and to say so here rather than let the status quo pass for a
decision.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'lines that command returns:    %s\n' "$( command grep -rn 'into_inner' ring_*/src ring_*/tests --include='*.rs' | wc -l )"
printf 'types declaring into_inner:    %s\n' "$( command grep -rln 'fn into_inner' ring_*/src --include='*.rs' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'crates the matches come from:  %s\n' "$( command grep -rl 'into_inner' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'matches inside this crate:     %s\n' "$( command grep -rn 'into_inner' ring_shutdown/src ring_shutdown/tests --include='*.rs' | wc -l )"
printf 'of those, the declaration:     %s\n' "$( command grep -rc 'pub fn into_inner' ring_shutdown/src/lib.rs )"
printf 'one is a doc-comment mention:  %s\n' "$( command grep -rn 'into_inner' ring_shutdown/src ring_shutdown/tests --include='*.rs' | command grep -c '/// ' || true )"
printf 'so call sites of the subject:  %s\n' "$( command grep -rn 'into_inner' ring_shutdown/src ring_shutdown/tests --include='*.rs' | command grep -v 'pub fn into_inner' | command grep -cv '/// ' || true )"
printf 'the bullet that removes it:    %s\n' "$( awk '/^### Regenerate/{exit} {print}' ring_shutdown/docs/decisions/001_should_into_inner_exist.md | command grep -m1 -o 'Zero non-test call sites' )"
printf 'crates that hold a Guarded:    %s\n' "$( command grep -rl 'Guarded' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'the owner crate exists:        %s\n' "$( ls -d ring_factory 2>/dev/null | wc -l )"
printf 'and hands out a Guarded:       %s\n' "$( command grep -c 'Guarded' ring_factory/src/lib.rs 2>/dev/null || true )"
# excludes .claude/worktrees/ copies and hyphen-prefixed scratch/backup dirs
# at the repo root -- neither is a real corpus file (see ring_factory's
# docs/integration/002 for the worktree-exclusion precedent)
printf 'files referencing this file:   %s\n' "$( command grep -rl 'should_into_inner_exist' --include=*.md . | command grep -vE '\.claude/worktrees|^\./-' | wc -l )"
printf 'of those, outside this crate:  %s\n' "$( command grep -rl 'should_into_inner_exist' --include=*.md . | command grep -vE '\.claude/worktrees|^\./-' | command grep -vc ring_shutdown || true )"
printf 'options the table below lists: %s\n' "$( awk '/^\| Option \|/{f=1;next} f&&/^\|---/{next} f&&/^\| /{n++} f&&!/^\| /{exit} END{ print n+0 }' ring_shutdown/docs/decisions/001_should_into_inner_exist.md )"
printf 'the other exit from a Guarded: %s\n' "$( awk '/^  pub const fn shutdown/{ if ( ++n == 2 ) { sub( /^ */, "" ); print } }' ring_shutdown/src/lib.rs )"
printf 'options naming that exit:      %s\n' "$( awk '/^\| Option \|/{f=1} f&&/^$/{exit} f' ring_shutdown/docs/decisions/001_should_into_inner_exist.md | command grep -c 'shutdown()' || true )"
```

Live output:

```
lines that command returns:    28
types declaring into_inner:    ring_align ring_shutdown 
crates the matches come from:  ring_align ring_bench ring_mpsc ring_shutdown ring_trace 
matches inside this crate:     4
of those, the declaration:     1
one is a doc-comment mention:  2
so call sites of the subject:  1
the bullet that removes it:    Zero non-test call sites
crates that hold a Guarded:    ring_shutdown 
the owner crate exists:        1
and hands out a Guarded:       0
files referencing this file:   9
of those, outside this crate:  0
options the table below lists: 4
the other exit from a Guarded: pub const fn shutdown( &self ) -> &'a Shutdown
options naming that exit:      0
```

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_close_is_advisory_to_an_unguarded_producer.md](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md) | Mitigation 2, whose "treat it as a scope exit" advice this decision may replace with a compiler-enforced version |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_shutdown_surface.md](../api/001_shutdown_surface.md) | The guarantee column, one of whose **construction** rows this method can be used to leave |


### SD13 — The Measurement Filed to Settle This Decision Cannot Distinguish Its Own Subject

The decision defers to a command, and routes to three different outcomes on
what that command returns. The command greps the bare identifier `into_inner`
across every `ring_*` crate's `src` and `tests`.

The family declares that name on two unrelated types — `ring_align`'s
`CacheAligned::into_inner` and this crate's `Guarded::into_inner` — and *uses*
it on two more, `std::sync::Mutex::into_inner` in `ring_bench` and
`ring_mpsc`'s tests, and `std::sync::PoisonError::into_inner` in `ring_trace`.
Run today the command returns thirteen lines from five crates. Three are in this
crate: the declaration, one doc-comment mention, and **one** call site — the one
in the test the decision itself identifies as circular. The other ten are about
`CacheAligned` and two `std` types.

The first bullet is the one that matters, and it is unreachable. *"Zero
non-test call sites → reason 1 never materialized. Remove it"* — the command
cannot return zero. It could not return zero the day this decision was filed,
and it would not return zero after `Guarded::into_inner` was deleted, because
`CacheAligned` and the two `std` uses would still match. So the branch that
retires the method is guarded by a threshold its own instrument can never
report, and the two branches that keep it are reachable on evidence about
`ring_align`.

The fix is one qualifier — grep `guarded.into_inner`, or restrict the paths to
crates that mention `Guarded` — and the reason it is worth writing down is that
the decision is otherwise carefully built: it names its owner, enumerates its
options with costs, states why it is filed rather than decided, and separates
the good reasons from the circular one. All of that rests on a command that
answers a different question.

Two smaller things travel with it. `ring_factory`, named as the owner and as the
event that will produce the evidence, **already exists** as a crate and hands
out no `Guarded` — so the trigger has quietly passed without firing. And no file
outside this crate's own `docs/` references this decision at all: not
`ring_factory` itself, not any other crate. The decision is filed,
correct in shape, and connected to nothing that would surface it.

**Disposition:** applied — the measurement command now restricts its search to
crates whose `src`/`tests` mention `Guarded` at all (today, `ring_shutdown`
alone), so `ring_align`'s `CacheAligned::into_inner` and the `std::into_inner`
uses elsewhere no longer inflate the count, and the "zero non-test call sites"
branch is reachable again. This does not resolve the `into_inner` removal
question itself — that remains for the measurement above to settle, per this
document's own filing.
Now prints: `guarded crates: ring_shutdown`

```sh
cd "$(git rev-parse --show-toplevel)"
GUARDED_CRATES=$( command grep -rl 'Guarded' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u )
printf 'guarded crates: %s\n' "$GUARDED_CRATES"
for c in $GUARDED_CRATES; do command grep -rl 'into_inner' $c/src $c/tests --include='*.rs' 2>/dev/null; done | wc -l
```

Live output:

```
guarded crates: ring_shutdown
2
```

### SD14 — Four Options Constrain One Exit While a Cheaper One Goes Unlisted

Every row of the options table is about `into_inner`: keep it, remove it, make
it re-check, or replace it with a scoped closure. The table's implicit premise
is that `into_inner` is *the* route from a guaranteed surface back to an
unguaranteed one — the decision says so directly: *"It is the only sanctioned
route."*

It is not the only route, and it is the more expensive of the two.
`Guarded::shutdown( &self ) -> &'a Shutdown` is a `const fn` that consumes
nothing, and `Shutdown::close` takes `&self` — so a `Guarded` holder can reach
`close`, and through the returned `Stopped` reach `drain_all`, `discard_all` and
`reopen`, while still holding the guard (→
[`api/002`](../api/002_the_surface_the_table_does_not_grade.md)).

That is not the same escape as `into_inner` — it does not yield a raw producer,
so [`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)
is not what it re-opens — but it is a route out of the guard's constrained
position, and it is strictly cheaper: no consumption, no ceremony, no name that
warns.

The consequence is concrete: **Option 2 does not do what it says.** *"Remove it
→ `Guarded` becomes a one-way wrapper"* is false while `shutdown()` remains.
Picking the strictest row on the table would leave the guard exactly as
escapable as before, and the decision would close believing otherwise.

Adding a fifth row is not the fix here — whether the accessor should exist is a
separate question with a separate answer, since removing it would break
`wait_for_close( guarded.shutdown(), … )`, which is the composition the crate is
built around. What belongs in this decision is the sentence it is missing: that
`into_inner` is not the only exit, so removing it does not close the surface.
