# lifecycle

Two lifecycles run here and they disagree about the same crate. The type's own is
tiny and fully determined: three states, three edges, one of them absorbing by
design. The crate's documentary one has four stages and has never advanced past
the first, while the crate itself is finished, tested and indexed as green.

The pair is worth reading together because the first is a model of a lifecycle
being taken seriously — every transition is a named method, every state is
reachable only through it — and the second is a lifecycle whose machinery exists
in full and has never been operated.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_three_states_and_the_two_ways_out_of_one.md) | Three States and the Two Ways Out of One | The type's state graph, its missing exit, and how fast the log grows |
| [002](002_a_finished_crate_in_the_unverified_stage.md) | A Finished Crate in the Unverified Stage | Task 125 against the readme and the family index, times 33 |

## A State Graph With No Way Out of the Process

Two constructors fix the flag and nothing else writes it. Exactly two methods
write the log — `record` pushes, `clear` empties — so the graph is disabled
(no outgoing edge), enabled-empty, and enabled-populated. That is the whole
model, and it is enforced rather than described.

What has no exit is the log itself. No `Drop` impl, no `flush`, no `dump`, no
`persist`; `entries()` is the only way content leaves the `Trace`. For most types
that is unremarkable — for a diagnostic aimed at concurrency failures, where the
interesting runs are the ones that abort or hang, a log that requires a living
reader in the same process cannot testify about how the process ended. The crate
should not grow a file writer; it should say this.

## A Growth Rate and a Stage Count

The module doc says the log "grows without bound", and nothing caps it — zero
`with_capacity`, `truncate`, `reserve` or `capacity` in the crate. Measured, one
producer puts 5.8–6.0 million entries into it in a 200 ms window: 667 and 681 MiB
per second across two runs, with four producers slower in aggregate because the
lock throttles them. The claim is right; the budget is the half nobody wrote down.

The documentary stage count is starker. All 33 task files sit in `unverified/`
with `verified/`, `executed/` and `accepting/` empty, while all 33 crate rows in
the family index read implemented. A four-stage lifecycle with its entire
population in stage one is not tracking anything.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the type: two flag writers, two log writers, no exit --'
command grep -n 'entries_guard().push\|entries_guard().clear' ring_trace/src/lib.rs | sed 's/^/    /'
printf '    Drop impls: %s   flush/dump/persist/save methods: %s   caps on the Vec: %s\n' \
  "$( command grep -c '^impl Drop' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'fn flush\|fn dump\|fn persist\|fn save' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'with_capacity\|truncate\|reserve\|capacity' ring_trace/src/lib.rs || true )"
echo '  -- the crate: one stage, thirty-three times --'
for st in unverified verified executed accepting; do
  n=0
  for c in ring_*/task/; do n=$(( n + $( ls "$c$st" 2>/dev/null | wc -l ) )); done
  printf '    %-11s %s\n' "$st/" "$n"
done
printf '    family index rows marked implemented: %s\n' \
  "$( command grep -c '| `ring_[a-z_]*` |.*implemented' docs/crate/readme.md || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR29 | `ring_trace` | n/a — doc gap | Two constructors fix the flag with no third writer and exactly two methods write the log — `record` pushes, `clear` empties — so the state graph is three nodes and three edges with disabled carrying no outgoing edge at all, which is the crate's own decision and a good one; what is not a decision, because nothing records it as one, is that the enabled states have no exit out of the process either — no `Drop` impl, no `flush`, `dump`, `persist` or `save`, and `entries()` the only way log content leaves the `Trace`, so the log lives in process memory and is gone when the type drops; unremarkable for most types and worth stating for this one, since the module doc positions the crate as what a diagnosis reaches for after the counter while the failures a sequence trace helps most against — an abort, a hang, a run under a harness that kills the process — are exactly the ones where nobody gets to reach, and a trace requiring a living reader in the same process cannot testify about how that process ended; the remedy is not a file writer, which is a caller's job and would drag `std::fs` into the family's only `std`-needing crate, but the sentence saying the log is process-local and unflushed and must be copied out before the end being diagnosed |
| TR30 | `ring_trace` | **measured cost** | The module doc is honest about direction — a trace "grows without bound", said in the same breath as `ring_stats`' constant space — and nothing caps it, with zero occurrences of `with_capacity`, `truncate`, `reserve` or `capacity`, which is consistent with the claim rather than a gap in it; the rate is what nobody has, and one producer recording as fast as it can puts 5.83 and 5.95 million entries into the log in a 200 ms window across two runs, 133 and 136 MiB of live `TraceEntry` or 667 and 681 MiB per second, with four producers slower in aggregate at 186–202 MiB/s because the lock throttles them — contention doing the crate a favour rather than a design margin — so an enabled trace on a hot producer exhausts a gigabyte in under two seconds, which is not a defect since an unbounded log is what was asked for and the crate is off by default precisely so this cannot happen accidentally, but does mean the enabled state is bounded in practice by wall-clock seconds rather than by anything in the code, and a caller switching it on for a diagnostic run needs to know the run has to be short |
| TR31 | `ring_trace` | **wrong doc** | Task 125 carries `**State:** ❓ Unverified`, the stage meaning the work has not passed the readiness gate that would let anyone claim it, with a Scope section reading "Not yet worked out" and requiring itself to "name concrete deliverables rather than restate the goal", and a Verification section reading "Not yet defined — see Scope" — against a crate of 328 lines with nineteen tests, a 115-line manual plan, a readme stating "Implemented … every line is covered", and a family index row giving implemented, 328 lines, 15 doc items, 19 tests, green; the document whose job is to say what should be built says nothing has been decided while both documents describing what exists say it is finished, and the task file is not merely stale in its marker since its two substantive sections were never filled in, so it never described this crate at any point including before the work — repairs must land together, advancing the marker and filling Scope and Verification with what the crate delivers, because advancing alone would move a placeholder into a stage meaning "ready to claim" |
| TR32 | `ring_trace` | n/a — inconsistency | The stage census is unanimous in both directions: all 33 `ring_*` task files sit in `unverified/` with `verified/`, `executed/` and `accepting/` empty across the whole family, and all 33 crate rows in `docs/crate/readme.md` are marked implemented — a symmetry that rules out the single-stale-file explanation, since this is not a task somebody forgot to advance but a task tree generated once before the work and never touched again while 33 crates were written, tested and indexed; the lifecycle machinery is fully present, four stage directories per crate, a readiness gate in the governing rulebook, a state marker in every file, and nothing has ever moved through it, so every consumer reading it — a person picking up work, a tool counting what remains — gets "none of the 33 crates has started" from a workspace where all 33 are green, and the more useful outcome than a 33-file batch edit is deciding whether the task tree is meant to be maintained at all, because a stage system nobody advances misleads more than no stage system while the family index already carries the state honestly |
