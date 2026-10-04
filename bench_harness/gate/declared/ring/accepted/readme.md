# accepted

Survivors of `mutant_survey.sh` that the family has ruled are decisions rather
than gaps. The survey subtracts these and reports what is left, so a sweep says
*what is new* instead of *what is settled*.

| File | Responsibility |
|------|-----------------|
| `ring_bench.accepted` | The three timing-only mutations that suite's no-clock rule makes permanently invisible |
| `ring_store.accepted` | An `is_empty` that is already `false` for every buffer that can exist |
| `ring_config.accepted` | A boundary comparison whose two branches return the same value on the boundary |
| `ring_cursor.accepted` | The alignment canary, true by the layout of one struct |
| `ring_mpsc.accepted` | The alignment canary, true across two separate allocations |
| `ring_poll.accepted` | Eleven mutations of a retry loop that a single-threaded test cannot observe |
| `ring_spsc.accepted` | The alignment canary, delegated rather than recomputed, and a bounded drain's reload test that differs only in cost |
| `ring_wait.accepted` | Two mutations of a spin hint's iteration count |

One file per crate, named `<crate>.accepted`. A crate with no file here has
accepted nothing, and every survivor it reports is new.

## Two kinds of acceptance, and only one of them costs anything

The entries are not all the same kind of decision:

| Kind | What it means |
|---|---|
| **Equivalent mutant** | No input separates the mutation from the original, so no test could kill it however the suite were written |
| **Refused test** | The mutation is killable in principle, but only by a test this family has decided not to write |

The first kind costs nothing. `ring_store`'s `is_empty` cannot return `true`
because a `Capacity` cannot be zero; `ring_config`'s `>` and `>=` return the
same value at the only input where they differ. Accepting these does not lower
a bar. There is no bar to lower, and a test written to chase one would be
asserting that the type system is doing what it does.

The second kind is a real cost, deliberately paid, and each entry says so
plainly. `ring_bench`'s are killable by a clock, and a flaky assertion
inside a benchmark harness discredits the measurement the harness exists to
produce. Two of `ring_poll`'s, the extra-attempt mutations in `push_within`
and `recv_within`, are killable by a concurrent fixture where another thread
drains between attempts. That fixture is a race, not a test. The other nine are
of the first kind.

Keeping the two apart is what stops this directory becoming a place things go
to be excused. An entry of the second kind is a standing invitation to revisit;
an entry of the first kind is a fact about the code.

## The alignment canaries

Three entries, `ring_cursor`, `ring_spsc` and `ring_mpsc`, are the same method
under three names. Read them together, because they are the clearest case of an
acceptance being correct *for the reason the method exists*.

`on_distinct_lines` asks whether two cursors share a cache line. It answers
`true` today for every input that can be built, because `PaddedCursor` has
`align_of == CACHE_LINE` (128), so two distinct ones sit at distinct multiples
of a line. Nothing
can make it answer `false` without breaking that alignment. `ring_align`'s and
`ring_cursor`'s own tests assert that alignment directly, and both crates swept
clean.

Which is precisely the point. `ring_mpsc`'s doc says why the check is there. A
sibling change that dropped the padding would cost this crate a contended line
on its hottest path **and break nothing that compiles**. The method is a canary.
A canary that is alive is supposed to read the same every time; the mutation
removes the canary, and no test can tell, because a live canary and a fake one
report identically. Only a dead one distinguishes them.

So these three are not untested. The logic is tested where it lives
(`ring_align`, at both polarities, clean), the contract is tested where it is
declared (`ring_cursor`'s `align_of` assertions), and these three are the alarm
wired to both. An alarm is not a gap.

## Why this exists at all

`mutant_survey.sh` reports what a suite failed to notice, which is not the same
question as what a suite failed to defend. Some things are not noticed on
purpose: `ring_bench` refuses to assert on a clock, so every timing-only
mutation survives it permanently and correctly. The tool cannot see that
distinction and never will. It reports the mutation and stops.

Left alone, that makes the survey worse the more it is used. Every sweep
re-presents the same rulings as fresh findings, the list grows, and the real
entries get read past. The survey subtracts the list instead of only filing
it, so the output stays worth reading.

## The two files a survivor can end up in

A survivor is a question with two honest answers, and the two directories are
the two answers:

| Answer | Goes to | Effect |
|---|---|---|
| Worth defending | `../mutant/*.mutant` | G12 reinstates it every run and requires the suite to go red |
| A decision, not a gap | here | The survey subtracts it and stops reporting it |

**A survivor in neither file is still decided. It is decided silently, by
whoever next reads past it.** That is the case this directory exists to
remove, and the reason an entry here carries its reasoning rather than only its
descriptor.

## Format

Comment lines start with `#`; every other line is one survivor, matched against
`cargo mutants`' own `missed.txt` output by exact whole-line comparison.

```
ring_bench/src/lib.rs:686:5: replace Outcome::write_nanos -> u128 with 0
```

Copy the line verbatim from `-mutants_out/mutants.out/missed.txt` rather than
retyping it. The descriptor carries a line number, so it is as fragile as a
`.mutant` declaration's `from` block and fails the same way when the code moves.

## Staleness is checked, not assumed

The subtraction runs in both directions. The survey reports an acceptance
matching no survivor as **STALE** and exits 3. Either the code moved under the
entry, or the suite grew an assertion that now catches it. As written, the
entry grants a permission for something that is not there.

This is `mutant/`'s stale-`from` rule applied to the other half of the same
problem. Both directories shrink under it rather than accumulate, which is the
property that keeps either one honest.

What neither check could do is fire on its own. A stale `from` block fails G12
on every gate run, but an acceptance can only be found stale by a sweep, and for
a while nothing decided when to sweep. `../surveyed/` closes that: the survey
records a digest of what it swept on every clean run, and G13 fails once the
digest stops matching. So an entry here is more than checkable. It is checked
against a crate that has not moved since somebody checked it.

```bash
# re-survey and see the subtraction; exit 0 clean, 1 new survivors, 3 stale entry
bash bench_harness/gate/mutant_survey.sh ring_bench
```
