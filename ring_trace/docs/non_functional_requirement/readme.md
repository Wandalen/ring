# non_functional_requirement

This crate carries two non-functional requirements and was handed neither. Its
acceptance criterion counts entries; the crate read the second half as a cost
requirement, set itself the harder standard, and then measured nothing against
it. Its `Mutex` decision fixed the family's only structural `std` dependency; the
crate argues the lock at length and never mentions the portability floor that
came with it.

Both are the same shape as the rest of this corpus: the reasoning is present and
better than average, and it stops one step short of the artefact. A requirement
stated in a paragraph and checked by nothing is a requirement in the same sense
that an unmeasured claim is a measurement.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_zero_when_not_is_a_count_not_a_cost.md) | "Zero When Not" Is a Count, Not a Cost | The self-imposed timing requirement and everything that does not check it |
| [002](002_the_one_crate_that_genuinely_needs_std.md) | The One Crate That Genuinely Needs `std` | Three `std::sync` items with no `alloc` path, and 25 crates that need none |

## A Requirement Nothing Times

The criterion asks that the trace record "one entry per sequence operation when
enabled and zero when not" — a counting statement, met three ways under 50 calls,
five operation kinds and 8,000 calls on four threads. The module doc reads the
second half as cost, calling it "the harder one" because "a trace that costs
something when disabled is a trace nobody leaves compiled in".

That is a better requirement than the one imposed, and nothing checks it. No
`Instant`, no `Duration` and no timing call anywhere in the crate — the single
`nanos` match is the word "nanoseconds" in a module-doc sentence arguing that the
ordering, not the nanoseconds, is what should be trusted. And no manifest in the
family depends on `ring_trace`, `ring_bench`'s included — the crate that owns the
harness whose distortion the argument worries about names `ring_trace` only in two
comments, both citing `Trace::entries_guard` as precedent for a poison-recovery
idiom, one of them saying outright that it reproduces the shape because it cannot
call the function. So the distortion cannot occur and the freedom cannot be
confirmed.

**Correction (2026-09-20):** this paragraph read "No `Instant`, no `Duration`, no
`nanos` in any file of the crate" and "has no file anywhere that mentions
`ring_trace`, not the manifest, not the source, not the tests", while the
Regenerate block below counted 1 and 3 respectively. Both findings survive intact
— nothing times anything, and no crate exercises the trace — but the literal
wording did not: the one `nanos` hit is prose, and the three `ring_bench` hits are
comment citations plus a generated dependent-list, none of them a dependency edge.
The recipe below now shows the matches rather than counting them, so a mention can
no longer be mistaken for a use in either direction.

## A Portability Floor Set by a Lock

Eight of 33 crates touch `std::` at all. Four take threads, one takes `HashMap`,
one is the benchmark harness, and `ring_tls` takes a single item that turns out to
be `alloc::vec::Drain` re-exported — proved by compiling a conversion between the
two paths that needs no conversion. That leaves `ring_trace`, whose three items
are `Mutex`, `MutexGuard` and `PoisonError`, none of which exists in `core` or
`alloc`.

The other 25 crates reference nothing from `std`, `Vec` included, since `Vec`
comes from `alloc`. Three of those 25 declare `#![ no_std ]` — `ring_overflow`,
`ring_stats` and `ring_types` — and no manifest carries a `std` feature, so the
family's portability posture is stated on three crates and undeclared on the
other 30: the remaining 22 would compile under `no_std` today with nothing
holding them there, and the eight above are held there by nothing either.

**Correction (2026-09-20):** this paragraph read "No crate declares
`#![ no_std ]`" while the TR36 row below — in this same file — already said
"only three crates declare `#![ no_std ]`, three of 33 in source". The
Regenerate block below is why the rollup never caught up: its two `for c in
ring_*/` loops have matched nothing since the crates moved to `ring/`,
so the census printed `crates touching std: 0` and `crates declaring no_std: 0
of 33` — a dead glob confirming the stale sentence instead of contradicting it.
Both loops are retargeted below, and the counts they now print are the ones
this section states.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the self-imposed requirement, and what checks it --'
# shown rather than counted: the one surviving match is the word "nanoseconds" in
# a sentence arguing the ordering is what to trust, and a bare count of 1 reads as
# a timing call that is not there
echo '    every Instant, Duration or nanos in the crate:'
command grep -rn 'Instant\|Duration\|nanos' --include=*.rs ring_trace/ 2>/dev/null \
  | sed 's|^ring_trace/|      |' || echo '      (none)'
# a mention in a comment is not a use — the dependency edge is the manifest, and
# ring_bench cites Trace::entries_guard as precedent for an idiom it reimplements
printf '    manifests in the family depending on ring_trace: %s\n' \
  "$( command grep -l 'ring_trace' ring_*/Cargo.toml 2>/dev/null | command grep -vc 'ring_trace/Cargo.toml' )"
echo '    ring_bench files naming ring_trace, none of them a dependency:'
command grep -rln 'ring_trace' ring_bench/ 2>/dev/null | sed 's|^ring_bench/|      |'
echo '  -- the portability floor --'
# a crate declaring `no_std` cannot be using `std`, so a `std::` match inside one
# is prose about the attribute, not a dependency — those are excluded by testing
# for the declaration first rather than by filtering comments, which would also
# drop the three crates whose only `std::` sits in a doctest
n=0; d=0; declarers=''
for c in ring_*/; do
  lib="$c"src/lib.rs
  if command grep -q 'no_std' "$lib" 2>/dev/null; then
    d=$(( d + 1 )); declarers="$declarers $( basename "$c" )"
  elif command grep -q 'std::' "$lib" 2>/dev/null; then
    n=$(( n + 1 ))
  fi
done
printf '    crates touching std: %s   touching nothing from it: %s\n' "$n" "$(( 33 - n ))"
printf '    crates declaring no_std: %s of 33 —%s\n' "$d" "$declarers"
command grep -o 'std::sync::[A-Za-z_]*' ring_trace/src/lib.rs | sort -u | sed 's/^/    /'
# of the crates touching std, which name it in code rather than only in a doctest
printf '    of those, naming std:: outside a comment: %s\n' \
  "$( for c in ring_*/; do command grep -vE '^[[:space:]]*//' "$c"src/lib.rs 2>/dev/null | command grep -q 'std::' && echo x; done | wc -l )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR33 | `ring_trace` | n/a — coverage | The criterion is a counting statement — the trace "records one entry per sequence operation when enabled and zero when not" — and both halves are met, with the three disabled tests asserting `len() == 0`, `is_empty()`, `entries().is_empty()` and `count_of( op ) == 0` for all five kinds including under four-way contention at 8,000 calls; the module doc reads the second half instead as a cost requirement, calling it "the harder one" because "a trace that costs something when disabled is a trace nobody leaves compiled in, and the family's whole output is a measured comparison that an always-on trace would distort", which is a better requirement than the one imposed and entirely self-imposed since the criterion says nothing about time — and having set the higher bar the crate measures nothing against it, with no `Instant`, `Duration` or `nanos` in any file across source, tests and manual plan, so a timing claim stated in the same paragraph as the counting one, in language reading as though both were imposed, has only one of the two checked by anything |
| TR34 | `ring_trace` | n/a — coverage | `ring_bench` is the family's harness, owning the `Instant`, `VecDeque`, `Mutex` and scoped threads that drive the comparisons, and the "measured comparison" the module doc worries about distorting is its output — yet no file anywhere under `ring_bench/` mentions `ring_trace`, not the manifest, not the source, not the tests, so the crate that argued its disabled path must be free because a benchmark would otherwise be distorted is invisible to the benchmark: the distortion it guards against cannot occur, there being no configuration in which the harness has a trace to switch on, and the freedom it claims cannot be confirmed, the one crate equipped to time anything having no edge to it; this is not a defect in either crate, but it is a coverage hole with a specific remedy — one bench case driving a ring with the trace disabled against the same ring with no trace at all, which would turn the central argument into a number and give `ring_trace` its first caller |
| TR35 | `ring_trace` | n/a — doc gap | Eight of 33 crates reach for anything under `std::` and the eight divide cleanly: four take threads (`scope`, `spawn`, `yield_now`, `sleep`) in concurrency helpers rather than data structures, `ring_registry` takes `HashMap`, `ring_bench` takes everything as a harness should, and `ring_tls` takes exactly one item, `std::vec::Drain`, which a compiled conversion shows is `alloc::vec::Drain` re-exported — the same type, no conversion needed — leaving `ring_trace` with `Mutex`, `MutexGuard` and `PoisonError`, all under `std::sync` and none existing in `core` or `alloc`, while the log's own `Vec` comes from `alloc` as the same probe shows, so the entire `std` dependency is the lock and its poisoning API; that makes this the family's only library crate whose `std` use is structural, and the crate says a great deal about the lock in a section arguing a `Mutex` is the right cost without mentioning that choosing the lock also fixed the portability floor — one decision producing both of the crate's exceptions, with neither document naming the other |
| TR36 | `ring_trace` | n/a — unadopted | Twenty-five of the 33 crates reference nothing under `std::` — not a thread, not a collection, not a clock — and `Vec` being available from `alloc` means even the allocating crates are not thereby tied to `std`, yet only three crates declare `#![ no_std ]`, three of 33 in source, and no manifest carries a `std` feature to gate it behind, so the family has a portability posture and has stated it on three of the 25 crates that hold it; the current state is "we do not care" by default on the other 22 while looking like "these are `no_std`-capable" by construction on all 25, which is the worst combination, since a reader auditing for embedded use finds 25 crates that would compile under `no_std` today and a signal on only three of them — 22 crates are still one attribute line from a posture the compiler would then hold, which would also turn this crate's `std` dependence from an invisible fact into a stated exception with a reason, where today the first crate to reach for `std::thread` in a data structure will do it silently |
