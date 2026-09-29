# Invariant: Exactly One Liveness Flag in the Family

### Scope

- **Purpose**: State that the family holds exactly one "is this ring closed" flag, that it is this crate's, and that the absence of a second one is checkable rather than assumed.
- **Responsibility**: The invariant, why a second copy is the failure mode, the command that checks it, and what it costs.
- **In Scope**: `Shutdown::closed`, and the absence of any equivalent in the other 32 crates.
- **Out of Scope**: What the flag guarantees once read (→ [`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)).

### Invariant Statement

**There is one `AtomicBool` answering "closed?" in the ring family, and it is
`ring_shutdown::Shutdown::closed`.** No handle, producer, consumer, or ring
carries a copy, cached or otherwise.

### Enforcement Mechanism

The invariant is a negative, so the check is a search for the things that must
not exist. A second flag is not necessarily an `AtomicBool` — the sentence above
forbids a *copy*, in whatever shape — so the search covers every shape a cached
liveness bit can take, and then asks which of them is named for liveness:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'files with an AtomicBool:      %s\n' "$( command grep -rln 'AtomicBool' ring_*/src --include='*.rs' | tr '\n' ' ' )"
printf 'files declaring fn is_closed:  %s\n' "$( command grep -rc 'fn is_closed' ring_*/src --include='*.rs' | command grep -v ':0$' | tr '\n' ' ' )"
printf 'of those, loads of the flag:   %s\n' "$( command grep -rc 'closed\.load' ring_shutdown/src/lib.rs )"
printf 'and stores into it:            %s\n' "$( command grep -rc 'closed\.store' ring_shutdown/src/lib.rs )"
echo '-- the other shapes the same sentence forbids --'
printf 'Cell< bool > fields:           %s\n' "$( command grep -rl 'Cell< *bool *>' ring_*/src --include='*.rs' | tr '\n' ' ' )"
printf 'plain bool fields:             %s\n' "$( command grep -rlE '^ *[a-z_]+ *: *bool,' ring_*/src --include='*.rs' | tr '\n' ' ' )"
printf 'non-boolean atomics:           %s\n' "$( command grep -rlE 'AtomicU(size|8|16|32|64)' ring_*/src --include='*.rs' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'any of them named for liveness: %s\n' "$( command grep -rnE '(closed|shutdown|stopped|alive|running) *: *(bool|Cell< *bool *>|AtomicU)' ring_*/src --include='*.rs' | command grep -v '^ring_shutdown/' | wc -l )"
```

Live output:

```
files with an AtomicBool:      ring_shutdown/src/lib.rs 
files declaring fn is_closed:  ring_shutdown/src/lib.rs:2 
of those, loads of the flag:   1
and stores into it:            2
-- the other shapes the same sentence forbids --
Cell< bool > fields:           
plain bool fields:             ring_trace/src/lib.rs 
non-boolean atomics:           ring_atomic ring_bench ring_core ring_stats 
any of them named for liveness: 0
```

**The second command's expectation is two, not one, and the count is not the
check.** `Shutdown::is_closed` reads the flag; `Refusal::is_closed` asks which
arm a refusal is and touches no atomic at all. A reader who checks only that the
count is 1 fails a passing invariant; one who checks only that the file is
`ring_shutdown` passes a hit that could be a second flag added here later. What
must hold is that exactly one of the hits is a load of `Shutdown::closed` —
which is why the check is a reading rather than a count.

**The last four lines are the ones that make the search match the sentence.**
`ring_trace` carries a plain `bool` field and four crates carry non-boolean
atomics, so the forbidden *shape* is already present in the family — the
discriminator is not the shape but the role, which is why the final line filters
on the names a liveness bit would be given. It reads zero, and a second flag
introduced under any of those names would move it off zero whatever type it was
declared with.

Both are checked as stage D3 of `tests/manual/readme.md`, because neither is
something a unit test can assert: a crate cannot test for the absence of a
field in a crate that does not depend on it.

### Violation Consequences

A cached copy of a liveness flag is wrong the instant the original changes, and
the window is unbounded — nothing invalidates it. The failure is not a race
that a stronger ordering fixes; it is that two values exist where the domain
has one fact. A producer holding a stale `false` publishes into a closed ring
and reports success, which is the failure this crate exists to prevent and
would then itself have caused.

This is why `ring_core` has no `is_closed`, and the absence is deliberate
enough to be recorded on both sides:

- `ring_core/docs/integration/002_handle_surface_divergence.md` lists
  `is_closed` in its divergence table as `present` / `absent` /
  ***settled** — see below*, and devotes a section to why it is settled rather
  than deferred.
- `ring_core/docs/invariant/001_no_atomic_of_its_own.md` states the general
  form: that crate adds no atomic of its own, so it has no flag to cache.
- `ring_core/tests/core_test.rs`'s module docs say the same in the "what is
  deliberately not here" section.

### Cost

The invariant is what makes `close` unenforceable against a raw producer. A
flag inside `ring_core::Producer` would be checked by every push automatically
— and would be exactly the cached copy this invariant forbids. The trap in
[`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)
is the price, `Guarded` is the mitigation, and the trade is deliberate rather
than an oversight.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'crates the invariant covers:   %s\n' "$( ls -d ring_*/ | wc -l )"
# deliberately not the module-tree total, which this line used to print: that
# count moves whenever any unrelated crate is added anywhere in the workspace,
# so quoting it made this document stale by other people's work rather than by
# its own subject changing.  The count below plus the name beneath it are the
# invariant itself, and they move only when it is actually violated.
printf 'family files carrying AtomicBool: %s\n' "$( command grep -rlc 'AtomicBool' ring_*/src --include='*.rs' | wc -l )"
printf 'and the one file that has it:  %s\n' "$( command grep -rl 'AtomicBool' ring_*/src --include='*.rs' )"
echo '-- does the enforcement command cover every shape the statement forbids? --'
E=$( awk '/^### Enforcement/{f=1} /^### Violation Consequences/{f=0} f' ring_shutdown/docs/invariant/001_exactly_one_liveness_flag.md )
for shape in 'AtomicBool' 'Cell< *bool *>' 'bool,' 'AtomicU(size' ; do
  printf '  %-16s searched: %s\n' "$shape" "$( printf '%s' "$E" | command grep -cF -- "$shape" )"
done
printf 'what the invariant forbids:    %s\n' "$( command grep -o 'carries a copy, cached or otherwise' ring_shutdown/docs/invariant/001_exactly_one_liveness_flag.md | head -1 )"
printf 'functions reading the flag:    %s\n' "$( awk '/\.is_closed\(\)/&&!/^ *\/\/\//{ n++ } END{ print n+0 }' ring_shutdown/src/lib.rs )"
printf 'the two the table names:       %s\n' "$( awk -F'\\|' '/^\| .\.\.\/api\/001/{ print $3; exit }' ring_shutdown/docs/invariant/001_exactly_one_liveness_flag.md )"
printf 'admit mentions in src:         %s\n' "$( command grep -c '\.admit()' ring_shutdown/src/lib.rs || true )"
printf 'of those, doctest lines:       %s\n' "$( command grep '\.admit()' ring_shutdown/src/lib.rs | command grep -c '^ *///' || true )"
printf 'so real call sites of admit:   %s\n' "$( command grep '\.admit()' ring_shutdown/src/lib.rs | command grep -cv '^ *///' || true )"
printf 'and try_push call sites in src: %s\n' "$( command grep '\.try_push(' ring_shutdown/src/lib.rs | command grep -cv '^ *///' || true )"
```

Live output:

```
crates the invariant covers:   33
family files carrying AtomicBool: 1
and the one file that has it:  ring_shutdown/src/lib.rs
-- does the enforcement command cover every shape the statement forbids? --
  AtomicBool       searched: 3
  Cell< *bool *>   searched: 2
  bool,            searched: 1
  AtomicU(size     searched: 1
what the invariant forbids:    carries a copy, cached or otherwise
functions reading the flag:    6
the two the table names:        Six real readers, not two: `admit`, `Guarded::try_push`, `Guarded::try_push_batch`, `Guarded::is_blocked`, `wait_for_close`, `for_space_or_close` 
admit mentions in src:         2
of those, doctest lines:       2
so real call sites of admit:   0
and try_push call sites in src: 1
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_shutdown_surface.md](../api/001_shutdown_surface.md) | Six real readers, not two: `admit`, `Guarded::try_push`, `Guarded::try_push_batch`, `Guarded::is_blocked`, `wait_for_close`, `for_space_or_close` |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_close_is_advisory_to_an_unguarded_producer.md](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md) | The cost this invariant imposes, stated as a caller-facing trap |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/docs/invariant/001`](../../../ring_core/docs/invariant/001_no_atomic_of_its_own.md) | The general form one layer down: no atomic of its own, hence no flag to cache |
| [`ring_core/docs/integration/002`](../../../ring_core/docs/integration/002_handle_surface_divergence.md) | Records `is_closed` as ***settled** — see below* in its divergence table, and argues the settlement |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | D3 — the two greps above, which no unit test can replace |
| `tests/shutdown_test.rs` | `close_is_idempotent_and_admit_reports_it` — the one flag, read through the one API |

### SD21 — The Document's Own Index of Who Reads the Flag Names a Dead Function and Omits the Live One

The APIs cross-reference row above reads *"`is_closed` and `admit`, the only
readers of the one flag"*. Six functions in `src/lib.rs` call `is_closed()`
outside a doctest: `admit`, `Guarded::try_push`, `Guarded::try_push_batch`,
`Guarded::is_blocked`, `wait_for_close`, and `for_space_or_close`. So "the only
readers" names two of six.

The selection is worse than arbitrary. `admit` has **zero** real call sites —
its two mentions in `src/lib.rs` are both doctest lines — and is used by nothing
in the crate, the family, or any test outside its own documentation. It is named.
`Guarded::try_push` is the operation the entire crate exists to make safe, the
push that [`../api/001`](../api/001_shutdown_surface.md) grades **construction**
on the strength of its flag read, and the only reason the invariant's cost
section has anything to trade against. It is not named.

The direct load is genuinely singular — `closed.load` appears once, which is the
invariant's real content and holds. What does not hold is the sentence a reader
arrives at first. A reader following this row to understand who depends on the
one flag learns about a function with no callers and never reaches the four that
carry the guarantee, including the two `Guarded` pushes and the two free
functions that spin on it.

Two stores also exist, not one: `close` and `reopen`, the second reaching
through `self.shutdown.closed` into a private field from a different type.
Neither the invariant statement nor the enforcement section mentions that the
flag has a second writer, and `reopen` is the operation
[`decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md) is open
about.

**Disposition:** applied — the `### APIs` table row now names all six real
readers instead of misnaming `admit` (zero real call sites) as one of "the
only readers" while omitting `Guarded::try_push` and the other four. Now
prints: `Six real readers, not two:`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
command grep -n '\.is_closed()' src/lib.rs | command grep -cv '^ *[0-9]*: *///' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
# The file under test is the one this block lives in, so an unbounded literal
# match finds the disposition, this command line, and every earlier copy of its
# own output -- which is how the block below reached eight stacked passes. Two
# guards: address the row structurally and stop at it, and spell the literal
# tw[o] so this line is not itself a match.
awk -F'|' '/^### APIs/{ f = 1 } f && /api\/001_shutdown_surface\.md/ { gsub( /^ +| +$/, "", $3 ); print $3; exit }' docs/invariant/001_exactly_one_liveness_flag.md
```

Live output:

```
6
Six real readers, not two: `admit`, `Guarded::try_push`, `Guarded::try_push_batch`, `Guarded::is_blocked`, `wait_for_close`, `for_space_or_close`
```

### SD22 — The Enforcement Mechanism Searched for One Spelling of the Thing It Forbids

The invariant forbids a second flag in the strongest available terms — *"No
handle, producer, consumer, or ring carries a copy, cached or otherwise"* — and
the enforcement command searched for the literal string `AtomicBool`.

Those are not the same set. A cached liveness bit can be a plain `bool` field, a
`Cell< bool >`, an `AtomicU8` used as a tri-state, or a sentinel value in an
existing counter, and every one of them passed both greps while being exactly
the thing the sentence prohibits. This was not hypothetical shape-hunting: four
crates in the family already carry non-boolean atomics (`ring_atomic`,
`ring_bench`, `ring_core`, `ring_stats`) and one already carries a plain `bool`
field (`ring_trace/src/lib.rs`'s `enabled : bool`). The forbidden *shape* is
present in the family today, in a role that happens to be innocent, and the
check could not tell the difference because it never looked.

The document was unusually careful about the *second* command — it argues at
length that a count is the wrong instrument there, that the expectation is two
rather than one, and that "the check is a reading rather than a count".
`tests/manual/readme.md`'s D3 goes further, naming the trap by number and
observing the prediction only works *"because the crate was read first"*. All of
that scrutiny was spent on the second command's arithmetic. None was spent on
whether the first command's search term matched the invariant's own words.

The asymmetry is the finding. The place a doc argues hardest was not where the
gap was — one paragraph proving a two-hit expectation correct, one line above a
search that would have missed the violation it was written to catch.

The repair could not be "add `bool` to the grep", because the family is full of
innocent bools; what distinguishes a forbidden copy from an ordinary field is
the *role*, not the type. So the enforcement block now enumerates every shape
and then filters the union by the names a liveness bit would be given, which is
the discriminator the invariant's own sentence actually uses. The Regenerate
block checks that enumeration against the statement rather than restating the
answer, replacing a line that printed the string `AtomicBool` from a hardcoded
`print` and would have gone on printing it whatever the command searched for.

**Disposition:** applied — the enforcement command now searches `AtomicBool`,
`Cell< bool >`, plain `bool` fields and `AtomicU*` across `ring_*/src`, then
reports how many of the hits are named for liveness outside `ring_shutdown`;
`Regenerate` verifies the four shapes are covered instead of asserting a
constant. Now prints: `any of them named for liveness: 0`
