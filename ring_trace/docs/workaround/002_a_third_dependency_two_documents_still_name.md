# Workaround: A Third Dependency Two Documents Still Name

### Scope

**Purpose:** Record that two of the crate's own documents name a dependency the
manifest does not declare and the code never imports, establish that the named
crate offers nothing this one could use, and measure how far the same
disagreement runs across the family.

**Responsibility:** The readme's dependency line, the task file's `Depends on`
field, the manifest's one path dependency, `ring_seqno`'s public surface, and the
readme-versus-manifest comparison for all 33 crates.

**In Scope:** `ring_trace/readme.md:5`;
`ring_trace/task/unverified/125_implement_ring_trace.md:19`;
`ring_trace/Cargo.toml`; `ring_seqno/src/lib.rs`; every `ring_*`
`readme.md` and `Cargo.toml`.

**Out of Scope:** The task file's unfilled Scope and Verification sections are
[`lifecycle/002`](../lifecycle/002_a_finished_crate_in_the_unverified_stage.md).
The crates this one names in its vocabulary but does not depend on are
[`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md).
The arithmetic the crate writes by hand is
[`workaround/001`](001_an_addition_the_types_crate_already_offers.md).

---

## What Two Documents Claim and Three Sources Deny

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
# both dependency lines are read by anchoring on the line that carries the
# claim, not by searching the files for the phrase: task 125 now also discusses
# its own `Depends on` field in prose, and an unanchored search cannot tell a
# claim from a description of one
echo '  -- the two dependency lines, and whether either still names a third --'
command grep -m1 '^Depends on' ring_trace/readme.md
command grep -m1 '^\*\*Depends on:\*\*' ring_trace/task/unverified/125_implement_ring_trace.md
echo '  -- against the manifest, the imports, and what ring_seqno actually offers --'
command grep -o 'path = "\.\./ring_[a-z_]*"' ring_trace/Cargo.toml | sed 's/^/    manifest: /'
printf '    ring_seqno named anywhere in src/ or tests/: %s\n' \
  "$( command grep -rc 'ring_seqno' --include=*.rs ring_trace/src ring_trace/tests 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
command grep -o 'pub fn [a-z_]*( [^)]*)' ring_seqno/src/lib.rs | sed 's/^/    /'
echo '  -- and how often a readme dependency line disagrees with its manifest --'
add=0; omit=0; both=0
for c in ring_*/; do
  rd=$( command grep -m 1 'Depends on' "$c"readme.md 2>/dev/null | command grep -o 'ring_[a-z_]*' | sort -u )
  md=$( command grep -o 'path = "\.\./ring_[a-z_]*"' "$c"Cargo.toml 2>/dev/null | command grep -o 'ring_[a-z_]*' | sort -u )
  e=$( comm -23 <( echo "$rd" ) <( echo "$md" ) | tr -d '\n' )
  m=$( comm -13 <( echo "$rd" ) <( echo "$md" ) | tr -d '\n' )
  if [ -n "$e" ]; then add=$(( add + 1 )); printf '    names an undeclared dependency: %-14s %s\n' "$( basename "$c" )" "$e"; fi
  if [ -n "$m" ]; then omit=$(( omit + 1 )); fi
  if [ -n "$e" ] || [ -n "$m" ]; then both=$(( both + 1 )); fi
done
printf '    over-claiming: %s   under-claiming: %s   disagreeing at all: %s of 33\n' "$add" "$omit" "$both"
```

Live output:

```
  -- the two dependency lines, and whether either still names a third --
Depends on [`ring_types`](../ring_types/readme.md).
**Depends on:** `ring_types`
  -- against the manifest, the imports, and what ring_seqno actually offers --
    manifest: path = "../ring_types"
    ring_seqno named anywhere in src/ or tests/: 0
    pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity )
    pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity )
    pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity )
    pub fn pending( producer : Seq, consumer : Seq )
    pub fn slowest( cursors : &[ Seq ] )
  -- and how often a readme dependency line disagrees with its manifest --
    names an undeclared dependency: ring_align     ring_types
    over-claiming: 1   under-claiming: 13   disagreeing at all: 14 of 33
```

---

### TR51 — The Named Crate Could Not Have Helped, Which Is Why the Claim Never Broke Anything

The readme's first line of substance says the crate depends on `ring_types` and
`ring_seqno`. Task 125 says the same, in the same order, in its `Depends on` field.
The manifest declares one path dependency, `ring_types`, and the string
`ring_seqno` appears zero times across every source and test file in the crate.

The claim was never plausible. `ring_seqno` is five free functions about position
comparison across laps, and three of the five take a `Capacity` — a ring's
capacity, which a trace does not have and has no way to obtain. The other two,
`pending` and `slowest`, compare producer and consumer cursors, which a log of
past operations does not hold either. There is no function in the crate a
`TraceEntry` could be passed to.

That is why nothing broke. An undeclared dependency that the code wanted would
have failed to compile immediately; this one is invisible, because no call site
was ever going to reach for it. The two documents agree with each other and with
nothing else, which is the signature of both having been written from the same
scaffold rather than from the code.

**Finding.** Recorded as a documentation defect with a specific, cheap remedy:
delete the second name from both files, in the same edit, since correcting one
would leave the two documents disagreeing where today they at least agree. This
is the second confirmed instance of the pattern — `ring_event`'s corpus records
the first, and it names the same absent crate — so the fix is worth doing in both
places at once.

```sh
cd "$(git rev-parse --show-toplevel)"
# anchored on the two lines that make the claim — see the paragraph below
echo '  -- readme --'
command grep -m1 '^Depends on' ring_trace/readme.md
echo '  -- task file --'
command grep -m1 '^\*\*Depends on:\*\*' ring_trace/task/unverified/125_implement_ring_trace.md
echo '  -- ring_seqno named in each of those two lines, and in the manifest --'
printf '    readme dependency line   : %s\n' "$( command grep -m1 '^Depends on' ring_trace/readme.md | command grep -c 'ring_seqno' )"
printf '    task 125 Depends-on line : %s\n' "$( command grep -m1 '^\*\*Depends on:\*\*' ring_trace/task/unverified/125_implement_ring_trace.md | command grep -c 'ring_seqno' )"
printf '    Cargo.toml               : %s\n' "$( command grep -c 'ring_seqno' ring_trace/Cargo.toml )"
```

Live output:

```
  -- readme --
Depends on [`ring_types`](../ring_types/readme.md).
  -- task file --
**Depends on:** `ring_types`
  -- ring_seqno named in each of those two lines, and in the manifest --
    readme dependency line   : 0
    task 125 Depends-on line : 0
    Cargo.toml               : 0
```

**Both greps had to be anchored, and the reason is the fix itself.** They used
to search each file for the phrase `Depends on` and print every hit. That was
adequate while the task file mentioned its dependency line exactly once — in
the line that *is* the claim. Task 125 then gained an `## Implementation
Record` whose account of this very correction says "both `readme.md`'s
dependency line and this task file's `Depends on` field", and an unanchored
search cannot tell a claim from a description of one, so the record of the fix
read as the defect coming back. `^`-anchoring on the two lines that carry the
claim, and counting `ring_seqno` inside each of them rather than across the file,
makes the prose below irrelevant to the measurement. The identical narrowing
landed on [`ring_event`'s twin
finding](../../../ring_event/docs/workaround/002_a_third_dependency_two_documents_still_name.md)
and on [`ring_overflow`'s mirror of
it](../../../ring_overflow/docs/lifecycle/002_implemented_tested_and_still_planned.md)
in the same pass, for the same reason and against the same three artefacts.

**Disposition:** applied — `ring_seqno` deleted from both `readme.md:5` and
task 125's `Depends on` field, in the same edit, leaving both documents
agreeing with the manifest's one real path dependency instead of agreeing
with each other and nothing else. All three now measure 0. Now prints: `    task 125 Depends-on line : 0`

---

### TR52 — The Dependency Line Is Wrong in Fourteen Crates, Mostly in the Other Direction

Comparing every crate's readme dependency line against its own manifest, 14 of
the 33 disagree. One over-claims — `ring_align`, naming a `ring_types`
dependency its own manifest does not declare. Thirteen under-claim, and the
omissions are much larger than the additions: `ring_mpsc`'s readme names one of
eight declared dependencies, `ring_tls` omits five, `ring_publish` four,
`ring_debug` four.

**Correction (2026-09-28):** this paragraph read "Two over-claim — `ring_event`
and `ring_trace`, both naming `ring_seqno`." That named the wrong crates. Neither
`ring_event` nor `ring_trace` is a family-wide over-claimer — `ring_trace`'s own
`ring_seqno` claim is TR51's fix above, and `ring_event` carries the identical fix
in its own corpus. The census's actual over-claimers are `ring_align` (naming
`ring_types`, absent from its manifest) and, until an independent fix removed
it, `ring_atomic` (naming `ring_seqno`). That `ring_atomic` fix is what today's
re-run changed: one over-claimer now, `ring_align`, not two. The rest of the
finding is unaffected — fourteen crates still disagree, thirteen still
under-claim, and nothing catches either direction.

So the line is not maintained, in either direction, and there is no mechanism by
which it could be. Nothing generates it, nothing checks it, and — as TR51 shows —
an error in it produces no compile failure, no test failure and no warning. It is
prose next to a manifest that already states the same fact machine-readably, and
the only way to notice a divergence is for someone to read both and compare.

That places this crate's error in context. It is not sloppiness particular to
`ring_trace`; it is the one form of the error that happens to be visible, since a
name that should not be there reads as a claim while thirteen names that should
be there read as nothing at all.

**Finding.** Recorded as a family-wide inconsistency in a hand-maintained
restatement of machine-readable data, which is the shape that always drifts. The
line does carry something the manifest does not — each entry is a link to the
dependency's readme, which is genuinely useful navigation — so deleting it is not
the answer. Generating it from the manifest is, and until something does, the
honest intermediate is to state on it that it is a reading aid rather than the
dependency list, so a reader knows which of the two to trust.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](../lifecycle/002_a_finished_crate_in_the_unverified_stage.md) | The other thing wrong with the same task file |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | The crates named in the vocabulary but not depended on |
| [`workaround/001`](001_an_addition_the_types_crate_already_offers.md) | What the crate does by hand instead of depending on more |
| [`integration/002`](../integration/002_the_criterion_lives_one_document_away.md) | The third documentary claim the code does not support |

### Sources

| Fact | Where |
|------|-------|
| The readme's dependency line | `ring_trace/readme.md:5` |
| The task file's `Depends on` | `ring_trace/task/unverified/125_implement_ring_trace.md:19` |
| The manifest's one path dependency | `ring_trace/Cargo.toml` |
| Zero mentions of `ring_seqno` in code | Census above |
| `ring_seqno`'s five signatures | `ring_seqno/src/lib.rs` |
| 14 of 33 readmes disagreeing with their manifests | Census above |

### Tests

| Test | Covers |
|------|--------|
| `an_enabled_trace_records_exactly_one_entry_per_operation` | The `Seq` from the one real dependency |
| `an_entry_reports_the_range_it_covers` | The arithmetic `ring_seqno` was never going to do |
| `a_batch_is_one_entry_carrying_its_count_not_n_entries` | The count that needs no capacity |
