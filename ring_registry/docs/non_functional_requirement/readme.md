# non_functional_requirement

The crate's three most argued decisions — a 448-byte `Result`, a double
allocation on the refusal path, and an `Entry` match measurably slower than its
alternative on the arm it was built for — all rest on one sentence: `register` is
a setup-time call, once per ring, never in a loop. Remove the premise and every
one of the three needs re-deriving.

That sentence is written once, inside a doc subsection headed "the
`result_large_err` allow", where a reader arrives only if they are already
investigating a lint suppression. Nothing benchmarks it. And against the one
requirement the crate states, the four properties it actually holds — reads that
allocate nothing — are stated nowhere, in a crate whose every sentence about
allocation is an argument against adding one.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_setup_time_call_and_what_that_licenses.md) | A Setup-Time Call, and What That Licenses | The premise, where it is filed, and the benchmark suite that has never seen this crate |
| [002](002_no_allocation_on_any_read_unstated.md) | No Allocation on Any Read, Unstated | The allocation-free reads, and the family idiom that would have said so |

## A Premise Nothing Can Falsify

`ring_bench` declares nine `ring_*` dependencies and names `Registry` zero times,
so the crate that argues hardest about cost in this family is outside the only
measurement apparatus it has. That is defensible on its own — a setup-time call
is a poor benchmark subject, which is what the premise says — and the consequence
is that the premise is unfalsifiable in place: nothing in the repository would
notice if `register` became expensive or if someone started calling it in a loop.

Measured here for the first time, the numbers support the premise's conclusion
while undercutting its necessity. `register` refuses in 113 ns, allocates 1810
bytes on its first success and 12 on a refusal, and the reads run 24–29 ns
allocation-free. A thousand registrations in a loop would cost about 113
microseconds, so the constraint is more conservative than it needs to be — the
better direction to be wrong in.

## Every Sentence About Allocation Argues Against Adding One

The contract mentions allocation exactly once, refusing to box the error because
boxing would allocate on the failure path. Sentences saying a read makes none:
zero. Measured, `contains`, `get_mut`, `len` and a single-name `names()` walk
make zero allocations and touch zero bytes, and that is not incidental —
`names()` returns `impl Iterator< Item = &str >` rather than a `Vec< String >`,
and the other three return scalars.

The family has a phrase for exactly this. `ring_flush:501` and `ring_tls:99`
write "No atomic, no lock, no allocation" on a single operation, identically, in
two crates that do not depend on each other. All three negatives hold on every
read here, and a per-operation form would carry a fourth term the two existing
sites do not need — the write path *does* allocate, twice.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- where the setup-time premise is written --'
command grep -n 'setup-time\|once per ring\|never in a loop' \
  ring_registry/src/lib.rs ring_registry/readme.md |
  sed 's|ring_registry/||' | sed 's/^/    /'
echo '  -- every sentence in the compiled crate about allocation --'
command grep -n -i 'alloc' ring_registry/src/lib.rs | cut -c1-88 | sed 's/^/    /'
echo '  -- the family idiom, and the two crates that write it --'
command grep -rn 'No atomic, no lock, no allocation' --include=lib.rs ring_*/src/ |
  sed 's|||' | cut -c1-88 | sed 's/^/    /'
echo '  -- and whether the family benchmark knows this crate --'
printf '    ring_* dependencies declared by ring_bench: %s\n' \
  "$( command grep -c '^ring_' ring_bench/Cargo.toml || true )"
printf '    mentions of Registry in ring_bench sources:  %s\n' \
  "$( command grep -rc 'Registry' --include=*.rs ring_bench/ | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG33 | `ring_registry` | n/a — doc gap | `register` being a setup-time call — once per ring, never in a loop — is the single claim that makes the crate's three most argued cost decisions correct: a 448-byte `Result` is acceptable because the call is rare, a double allocation on the refusal path is acceptable because the call is rare, and an `Entry` match that RG2 measures as the slower of the two candidate forms on that same path is acceptable for the same reason, so removing the premise leaves all three needing re-derivation; it is written down **once**, at `src/lib.rs:141`, inside a doc subsection headed "`# The result_large_err allow`" — a place a reader arrives at only if already investigating a clippy suppression — while `register`'s summary line does not carry it, the readme's one-row-per-method operation table does not carry it, and the `docs/` tree restates the three conclusions in three separate files and the premise in none; the repair is placement rather than content, since the sentence is correct and well-phrased and belongs where a caller deciding whether to call in a loop would meet it, the reader who most needs the constraint being the one least likely to be reading about a lint allow |
| RG34 | `ring_registry` | n/a — coverage | `ring_bench` declares **nine `ring_*` dependencies and names `Registry` zero times** in any of its sources, so the crate that argues hardest about cost in the family — twenty lines on a `Result`'s width, a paragraph on a clone, a paragraph on a hash count — sits outside the only measurement apparatus the family has, which is reasonable on its own since a setup-time call is a poor benchmark subject and leaves the premise unfalsifiable in place: no artefact in the repository would notice if `register` became expensive, if the `Entry` form's margin grew, or if someone started calling it in a loop, the only thing standing between the crate and a wrong cost model being that nobody has changed the code; measured here for the first time the numbers are undramatic and support the premise's conclusion while contradicting one of its supporting arguments — **`register` refuses in 113 ns, allocates 1810 bytes on first success and 12 on a refusal, and the reads are 24–29 ns and allocation-free** — so a thousand registrations in a loop would cost about 113 microseconds and nothing in that set needs the "once per ring" constraint to be acceptable |
| RG35 | `ring_registry` | n/a — doc gap | The contract mentions allocation exactly once and argues one side of one question — boxing the error would allocate on the failure path, so boxing is refused — which is the write path and a cost being incurred, while sentences saying a read makes none number **zero**; measured, the four read operations `contains`, `get_mut`, `len` and a single-name `names()` walk make **zero allocations and touch zero bytes**, and that is not incidental since `names()` returns `impl Iterator< Item = &str >` rather than a `Vec< String >` and the other three return scalars, so whoever wrote those signatures made allocation-free reading possible and nothing records that they did; the asymmetry is the finding — a caller meets allocation once, as a cost on a path they were told is rare, and never learns the path they will actually take repeatedly costs nothing — and one clause on each read states it, following from the signatures rather than the bodies, worth stating because the natural assumption runs the other way and a reader who assumes `names()` builds a collection will cache its result to avoid an allocation that does not exist |
| RG36 | `ring_registry` | n/a — unadopted | The family has a phrase for this and two crates use it: `ring_flush:501` reads "Stage one record. No atomic, no lock, no allocation, and **no flush**." and `ring_tls:99` reads "Append one item. No atomic, no lock, no allocation." — a short negative list attached to a single operation, naming the three costs a caller of a concurrency primitive most wants ruled out, written identically in two crates that do not depend on each other; twelve of the thirty-three crates mention allocation in `lib.rs` at all and this crate is one of them only through the "Box it" sentence, so its membership in that census is an argument against an allocation rather than a statement about its own; `ring_registry` fits the idiom and is a slightly different case, having no atomics and no locks either so all three negatives hold on every read, with the interesting term being the fourth the existing sites do not need — the write path does allocate, twice — and adopting it here would produce the only per-operation cost statement in a crate whose entire `docs/` tree argues about cost, its absence being why RG35 was available to find at all |
