# Data Structure: The Flush Log

### Scope

- **Purpose**: Describe the structure this crate's acceptance criterion forces into existence — a record of every flush and its cause — and account for the hazard of a test-only structure adjacent to a hot path.
- **Responsibility**: Fix the shape, the operations, the compilation boundary, and what the log must record to be worth having.
- **In Scope**: Entry shape; where the log lives; the release-build guarantee.
- **Out of Scope**: The invariant it verifies (→ [A Policy Fires Only at Its Trigger](../invariant/001_a_policy_fires_only_at_its_trigger.md)); the outcome type it should be derived from (→ [Flush Outcome](../type/002_flush_outcome.md)).

### Abstract

**This structure exists because the acceptance criterion is negative.**

This crate's row in
[the acceptance table](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md)
requires each policy to fire "at exactly its stated trigger and at no other
point, asserted by a scripted sequence with a **recorded flush log**." Proving
a flush happened needs no log — observe the ring. Proving no *other* flush
happened requires a record of every flush that did, compared against the
expected set. **You cannot observe an absence; you can only enumerate the
presences and find the set complete.**

That makes the log the only mechanism in this crate whose existence is mandated
by a test rather than by a feature — which is exactly what makes it dangerous.

### Structure

```text
FlushLog
└─ entries : Vec< FlushEntry >

FlushEntry
├─ policy   : FlushPolicy      which policy was in effect
├─ cause    : FlushCause       why it fired  ( Full | Barrier | Batch | Shutdown )
└─ outcome  : FlushOutcome     what happened  ( the same value the call returned )
```

**The third field is the outcome, not a count, and that is a change from the
structure this instance originally specified.** A `count : usize` cannot tell
`Rejected { staged }` from `TriggeredEmpty` — both moved zero records, and they
mean opposite things about whether the ring had room. Worse, it is a *second*
record of an event the caller already holds, and the "one derivation rule"
below exists precisely because two records of one event can disagree. Carrying
the outcome makes that rule structural instead of advisory: there is nothing
left to derive, so nothing left to derive wrongly.

The count is still available — `FlushEntry::count()` reads it out of the
outcome — so nothing that wanted a count lost it.

**`cause` is separate from `policy`, and the reason given here was wrong.** The
argument was that a log recording only "a flush occurred under `OnBatch( 64 )`"
cannot detect
[the invariant's V2](../invariant/001_a_policy_fires_only_at_its_trigger.md).
**V2 is unreachable** — validation caps `n` at capacity, so an `OnBatch` policy
never observes a full buffer (`tests/manual/readme.md`'s F3, and V2's own row).
A field cannot earn its place by detecting something that cannot happen.

It earns its place twice over anyway, for reasons the original text did not
give:

1. **`Shutdown` is not a policy.** A final drain and a policy firing both
   produce one entry; only `cause` says which. This is the discrimination that
   is actually exercised, by `the_final_drain_publishes_whatever_the_policy_would_have_held`.
2. **V1 remains reachable and detectable.** An `OnFull` that also fires on a
   size threshold is a real defect with no validation excluding it.

| Field | Without it | With it |
|-------|-----------|---------|
| `policy` | Cannot attribute entries in a multi-policy test | Each entry is attributable |
| `cause` | V1 undetectable; a final drain indistinguishable from a policy firing | A flush firing for the wrong reason is a visible mismatch |
| `outcome` | Cannot distinguish `Rejected` from `TriggeredEmpty`, and duplicates what the caller holds | Log and caller cannot be told different stories |
| A timestamp | — | **Deliberately absent** — reading a clock is forbidden near this path (→ [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)'s forbidden table) and ordering is already given by `Vec` position |

### Operations

| Operation | Cost | When |
|-----------|------|------|
| Append an entry | One `Vec` push, amortised; **may allocate** | Once per flush — the cold path |
| Read entries | Slice access | Test assertions only |
| Clear | `Vec::clear` | Between scripted scenarios |

**Appending may allocate, and that is acceptable only because it happens on the
flush path, not the append path.** A flush already does a claim, a copy and a
publish; one amortised push is noise against that. The same push on the
per-record path would violate `ring_tls`'s zero-allocation criterion, which is
why [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)
explicitly forbids writing to the log from policy evaluation.

### The compilation boundary — the question that dissolved

**Read this section as the reasoning that led somewhere better than its own
conclusion.** Its three risks are all real and all still hold. Its framing —
that the log must be *compiled out* — is what turned out to be too strong, and
every option in the table below inherits that framing.

**What was built instead: the log is an `Option< FlushLog >` field on the
driver, opted into by `Flusher::with_log()`.** No `cfg`, no cargo feature. A
build that never calls `with_log` allocates nothing, which is what all four
options were trying to buy, and the acceptance criterion holds identically with
and without default features — measured, not assumed
(`tests/manual/readme.md`'s F5). Full reasoning and what the fifth option costs
in exchange: [`decisions/`](../decisions/readme.md)'s Pending 2.

**The three risks below are answered by opt-in rather than by compilation:**
unbounded growth and flush-path allocation happen only in a process that asked
for them, and `ring_bench` measures a driver without a log unless it deliberately
builds one with. The residual exposure is that a caller *can* opt in on a hot
path — a visible call at a known site, rather than a property of the build.

**The log must not exist in a release build**, and the reason is not
performance in the abstract:

| Risk if always compiled | Consequence |
|------------------------|-------------|
| Unbounded growth | A long-running process accumulates one entry per flush forever — a leak whose rate is the flush rate |
| Allocation on the flush path in production | The path the benchmark is measuring gains an allocation that is not part of the design being measured |
| The benchmark measures the log | `ring_bench`'s numbers include an instrument that ships to nobody |

**The third is the one that corrupts the benchmark's output.** A benchmark
that measures the measurement apparatus produces a verdict about a system that
will never run. Compiling the log out of release builds is what keeps
`ring_bench`'s figures about the flush path rather than about the flush path
plus its own observation.

| Option | Assessment |
|--------|------------|
| `#[cfg(test)]` | Simplest. **Insufficient** — integration tests in `tests/` are a separate crate and do not see the flag |
| `#[cfg(debug_assertions)]` | Available to integration tests; excluded from release. Ties observability to the debug profile, which is coarse but honest |
| A cargo feature (`flush-log`) | Explicit, benchmarkable both ways, and the only option that lets `ring_bench` measure the instrument's own cost deliberately |
| Always compiled, caller supplies the sink | Most flexible; adds a generic parameter or a trait object to the driver, which the export Contract makes expensive to change |
| **Always compiled, opt-in per driver** | **Taken.** An `Option` field and one builder method. No generic parameter, no `cfg`, no feature — the last row's flexibility without its cost to the export Contract |

**A cargo feature was called the likely answer and was not taken.** The stated
objection to it was real — the criterion would hold only under a non-default
feature, so `g3_features.sh` would have to enable it or silently test nothing —
and the fifth row avoids that entirely rather than managing it.

**Why the fifth option was not in the original table** is worth keeping: the
four candidates are all answers to "when should this be compiled," and the fifth
is an answer to a different question, "when should this run." Four bad options
is often a sign the constraint is stated too strongly rather than that the
problem is hard.

### One derivation rule

**The log entry should be written from [`FlushOutcome`](../type/002_flush_outcome.md),
not alongside it.** Two independent records of the same event can disagree, and
if they disagree the acceptance criterion is checking a record that need not
match what the caller was told — that is
[`FlushOutcome`](../type/002_flush_outcome.md)'s M5. One source, two readers,
no divergence possible.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_evaluating_a_policy_at_an_append.md](../algorithm/001_evaluating_a_policy_at_an_append.md) | Forbids writing here — the log records flushes, not evaluations |
| [../algorithm/002_sequencing_seal_drain_reset.md](../algorithm/002_sequencing_seal_drain_reset.md) | Step 5, where the entry is written |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_the_policy_enum.md](001_the_policy_enum.md) | The `policy` field's type, and the opposite cost profile |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) | Its E1 — this structure is that invariant's only real enforcement, and `cause` is what makes it work |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md) | The criterion mandating this structure |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | M5 — the derivation rule above |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's "recorded flush log" row, the mandate for this structure |
| [`ring_bench/readme.md`](../../../ring_bench/readme.md) | The benchmark whose figures the compilation boundary protects |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | The log's opt-in shape — `a_driver_has_no_log_unless_asked` flushes successfully with `log()` still `None` and calls `clear_log` on the absent log without panicking; `the_log_can_be_cleared_between_scenarios` covers both reset routes, the driver's and `FlushLog::clear` directly |
| `tests/flush_test.rs` | The only consumer. `every_entry_names_its_own_policys_trigger` asserts on `cause` across all three policies; `the_log_agrees_with_every_outcome_it_recorded` asserts the entry and the returned outcome are the same value, which is true by construction now that the entry carries the outcome rather than a count |

### FL11 — A Section That Opens by Retracting Its Framing Keeps the Bolded Requirement That Framing Produced

The retraction and the requirement are thirteen lines apart:

```sh
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/data_structure/002_the_flush_log.md
echo '  -- the retraction, the built answer, and the surviving requirement --'
awk '/^### FL/{ exit }
     /turned out to be too strong|What was built instead|must not exist in a release build/ {
       printf "%d: %s\n", NR, substr( $0, 1, 88 ) }' "$F" 
echo '  -- how long the retracted section is, and how much of it describes what shipped --'
awk '/^### The compilation boundary/{ s = NR }
     s && /^### One derivation/{ printf "    section: lines %d-%d  (%d lines)\n", s, NR - 1, NR - s; exit }' "$F"
awk '/^### The compilation boundary/{ s = 1; next } /^### One derivation/{ exit }
     s && /Taken\.|What was built instead|answered by opt-in/ { n += 1 }
     END { printf "    lines in it describing the shipped design: %d\n", n }' "$F"
```

Live output:

```
  -- the retraction, the built answer, and the surviving requirement --
93: that the log must be *compiled out* — is what turned out to be too strong, and
96: **What was built instead: the log is an `Option< FlushLog >` field on the
110: **The log must not exist in a release build**, and the reason is not
  -- how long the retracted section is, and how much of it describes what shipped --
    section: lines 89-143  (55 lines)
    lines in it describing the shipped design: 3
```

Line 93 states the section's framing — that the log must be *compiled out* — was
too strong. Line 96 states what was built: an `Option` field, no `cfg`, no
feature, present in every build. Line 110 then asserts, in bold, that **the log
must not exist in a release build**, and the three-row table under it explains
why.

**The bolded line is not a leftover the corrective header covers.** The header
says the section's *framing* was too strong and that its three risks "are all
real and all still hold" — which is true of the risks and false of the sentence,
because the sentence is not a risk. It is the requirement the risks were marshalled
to justify, and it is the one thing in the section the built design does not
satisfy. A reader arriving at line 110 has been told the section is superseded in
a way that specifically exempts the material they are now reading.

**Fifty-five lines and three of them describe what shipped.** The rest is a
correctly-reasoned answer to a question that dissolved: four options, three
risks, and a recommendation, all preserved because the reasoning is good — and it
is good. The corpus convention that keeps superseded analysis rather than
deleting it is right, and the cost it does not price is that a retracted section
still reads as instruction, at bold weight, to anyone who does not read the
header first.

The shape worth keeping: **a correction added at the top of a section does not
demote what is below it.** Markdown has one weight for a requirement and the same
weight for a requirement that was withdrawn.

### FL12 — Both Rows of This Instance's Tests Table Name the Same File, and the Checker Reads Neither

The corpus has two incompatible spellings for a Tests table and this crate uses
the one that is not machine-readable:

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate/corpus
D=../../../ring_flush/docs
echo '  -- this instance'\''s Tests table --'
awk '/^### Tests/{ i = 1; next } i && /^\|/ { printf "    %s\n", substr( $0, 1, 78 ) } i && /^###/{ exit }' \
  "$D/data_structure/002_the_flush_log.md"
echo '  -- the header each instance uses for its own Tests table --'
command grep -rl '^### Tests' "$D" --include='[0-9][0-9][0-9]_*.md' \
  | while read -r f
    do awk '/^### Tests/{ i = 1; next } i && /^\| (File|Test) \| Relationship \|/ { print $2; exit }' "$f"
    done | sort | uniq -c | sed 's/^/    /'
echo '  -- tests actually in the file both rows name --'
printf '    %s\n' "$( command grep -c '^fn ' ../../../ring_flush/tests/flush_test.rs )"
echo '  -- and what the citation gate makes of the crate --'
python3 citations.py "$D" 2>&1 | tail -2 | sed 's/^/    /'
```

Live output:

```
  -- this instance's Tests table --
    | File | Relationship |
    |------|--------------|
    | `tests/flush_test.rs` | The log's opt-in shape — `a_driver_has_no_log_unless
    | `tests/flush_test.rs` | The only consumer. `every_entry_names_its_own_policy
  -- the header each instance uses for its own Tests table --
         22 File
          4 Test
  -- tests actually in the file both rows name --
    33
  -- and what the citation gate makes of the crate --
    0 problem(s)
```

`citations.py` matches a Tests row on `^\| \`?[a-z_][a-z0-9_]* \|` — a bare test
function name in the first cell — and checks that a function by that name exists
in the crate's own `tests/`. A first cell reading `tests/flush_test.rs` contains
a slash and a dot, matches nothing, and is skipped in silence.

**This instance's table is the sharpest case available:** two rows, both naming
the same file, distinguished only by their prose. Nothing in the table says which
of that file's tests each row means; the names are in the Relationship column, in
running text, where no checker looks. The five names it
mentions all exist — verified by hand for this finding — which is the point:
nothing verified them, and nothing would have said so if they did not.

**Twenty-two of this crate's instances use the invisible spelling and four use
the checked one**, and the four are recent. So a gate that reports zero citation
problems for `ring_flush` is reporting on four instances' worth of rows and
staying quiet about twenty-two — a green verdict whose coverage is a fifth of
what its name suggests.

The general shape: **a checker that skips what it cannot parse converts a schema
disagreement into a coverage gap, and reports the gap as a pass.** A row it
rejected would be a finding. A row it never recognised as a row is nothing at
all, and the file it did not check looks exactly like a file that passed.
