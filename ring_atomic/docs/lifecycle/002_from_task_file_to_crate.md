# Lifecycle: From Task File to Crate, With the Gate Left Behind

### Scope

**Purpose:** Record how this crate came to exist against how its own task system
says a crate is supposed to come to exist, and what the resulting record does and
does not preserve.

**Responsibility:** `task/unverified/102_implement_ring_atomic.md`, the readiness
gate it names, and the same file's 32 siblings.

**In Scope:** `ring_atomic/task/unverified/102_implement_ring_atomic.md`;
`ring_atomic/task/readme.md`;
`ring_tls/task/unverified/070_implement_ring_tls.md`;
`ring_mpsc/task/unverified/071_implement_ring_mpsc.md`.

**Out of Scope:** The life of a *cell* — construction to drop — is
[`lifecycle/001`](001_born_at_zero_climbing_until_dropped.md). The acceptance
criteria the crate actually answers to, which live in `bench_harness` rather than
in the task, are
[`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md).

---

## The Task, the Gate, and the 33 That Never Crossed It

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- this crate task, and the gate it names --'
# anchored on headings and on the last line of content, not on line offsets:
# the fixed-length window this replaces began printing one line past the
# Verification section the moment an Implementation Record was appended below it
command grep -m1 -A1 -F -e '- **State:** ❓ Unverified' ring_atomic/task/unverified/102_implement_ring_atomic.md
awk '/^## Scope$/{ p = 1 } p { print } /^Not yet defined/{ exit }' ring_atomic/task/unverified/102_implement_ring_atomic.md
echo '  -- where the 33 implementation tasks sit --'
for s in unverified verified executed accepting accepted
do printf '    %-11s %s\n' "$s" "$( find ring_*/task/$s -name '*.md' 2>/dev/null | wc -l )"; done
echo '  -- against the source those tasks were never claimed to write --'
printf '    crates with >200 lines of src : %s of 33\n' "$( for c in ring_*/src; do [ "$( cat $c/*.rs | wc -l )" -gt 200 ] && echo x; done | wc -l )"
printf '    tasks carrying a verified_by  : %s\n' "$( command grep -rh 'verified_by' ring_*/task/unverified/*.md | command grep -vc null || true )"
printf '    tasks whose Scope is unwritten: %s\n' "$( command grep -rl 'Not yet worked out' ring_*/task/unverified/*.md | wc -l )"
echo '  -- and the deliverable the one long task names, against the crate it names it for --'
printf '    BumpLog in ring_tls task : %s\n' "$( command grep -c BumpLog ring_tls/task/unverified/070_implement_ring_tls.md || true )"
printf '    BumpLog in ring_tls src  : %s\n' "$( command grep -c BumpLog ring_tls/src/lib.rs || true )"
command grep '^pub struct' ring_tls/src/lib.rs
```

Live output:

```
  -- this crate task, and the gate it names --
- **State:** ❓ Unverified
- **Executor:** any
## Scope

Not yet worked out. This task is `❓ Unverified` — it must pass the readiness
verification gate before it can be claimed, and that gate requires this Scope
section to name concrete deliverables rather than restate the goal.

## Verification

Not yet defined — see Scope.
  -- where the 33 implementation tasks sit --
    unverified  33
    verified    0
    executed    0
    accepting   0
    accepted    0
  -- against the source those tasks were never claimed to write --
    crates with >200 lines of src : 30 of 33
    tasks carrying a verified_by  : 0
    tasks whose Scope is unwritten: 31
  -- and the deliverable the one long task names, against the crate it names it for --
    BumpLog in ring_tls task : 23
    BumpLog in ring_tls src  : 0
pub struct TlsBuffer< T >
pub struct Flush< 'a, T >
```

---

### AT31 — The Task That Says the Crate Cannot Be Started Yet Describes a Crate That Is Finished

`102_implement_ring_atomic.md` sits in `task/unverified/`. Its Scope section reads,
in full, "Not yet worked out. This task is `❓ Unverified` — it must pass the
readiness verification gate before it can be claimed." Its Verification section
reads "Not yet defined — see Scope." Its `State` is `❓ Unverified`, and no task in
the family carries a `verified_by`.

The crate those two sentences forbid claiming is 376 lines of source, 17 tests, a
`loom` seam, two acceptance criteria answered in `bench_harness`, and — at the time
of writing — a doc corpus of its own. All 33 implementation tasks are in the same
position: 33 in `unverified/`, zero in `verified/`, `executed/`, `accepting/`, or
`accepted/`, while 29 of the 33 crates carry more than two hundred lines of source
each.

**Finding.** The gate was not failed, or waived, or argued about. It was bypassed,
uniformly, 33 times, and the task system's own record of that is a directory
listing that says the work has not begun.

The consequence is not that the crates are worse — the source is thorough and the
suites are real. It is that the *decisions* are unrecoverable from the task system.
A task file is where the "why this shape and not that one" of an implementation is
supposed to live, and for all 33 crates that section is the sentence declining to
write it. Everything actually decided about `ring_atomic` — the trait rather than a
struct, the hand-written `Default`, the nine `Relaxed` literals, the packed counter
block — survives only where its author happened to put it: three of those four in
the module comment, and the fourth nowhere
([`item/001`](../item/001_six_constructors_for_two_types.md) AT25).

---

### AT32 — Three Task-File Shapes, and the One That Worked Out Its Scope Is the One Most Wrong

The 33 files come in three sizes: twenty-six at 29 lines, five between 65 and 75,
and two at 204 and 213. Only the two longest — `ring_tls` and `ring_mpsc` — have a
Scope section that names deliverables instead of declining to.

`ring_tls`'s does it properly. It describes the crate as "a documented skeleton — a
17-line `lib.rs` doc comment... and no code yet", then names four deliverables, the
first being "`BumpLog`: an owned, `Send`-but-not-`Sync` handle around one contiguous
byte region, implementing the tagged-record bump-append procedure and its
rewind-only `reset`." `BumpLog` appears nineteen times in that file. It appears zero
times in the crate, which built `TlsBuffer< T >` — a typed, `Vec`-backed buffer with
a `flush_into` that claims through a `SeqCell`. Not a variant of the specified
mechanism; a different one.

**Finding.** The two files that did the most work are the two that are most
specifically wrong, and they are wrong in a way the thin files cannot be. A file
that says "Not yet worked out" is stale but honest — it claims nothing. A file that
names `BumpLog` nineteen times, in a crate with no `BumpLog`, reads as a
specification of what is there.

Both failure modes have the same cause and the same cheap remedy. Nothing recomputes
a task file against the crate it describes, and the check is a `grep` per named
deliverable — the same shape of check that would have caught the unused `ring_seqno`
dependency ([`integration/001`](../integration/001_two_declared_one_used.md) AT17)
and the stale "six manifests" count
([`integration/002`](../integration/002_five_crates_downstream.md) AT20). Three
findings, three documents, one missing habit.

**Disposition:** declined — the stale `BumpLog` deliverable is
`ring_tls`'s own task file,
`ring_tls/task/unverified/070_implement_ring_tls.md`; correcting it
belongs to `ring_tls`'s own crate-scoped disposition pass, not this
ring_atomic-scoped one.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_born_at_zero_climbing_until_dropped.md) | The other lifecycle — a cell from construction to drop |
| [`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md) | The acceptance criteria the crate answers to, which live outside the task |
| [`integration/001`](../integration/001_two_declared_one_used.md) | The same never-recomputed claim, in a manifest instead of a task |
| [`integration/002`](../integration/002_five_crates_downstream.md) | The same never-recomputed claim, in a count instead of a name |
| [`item/001`](../item/001_six_constructors_for_two_types.md) | Where this crate's decisions actually survive, since the task holds none |

### Sources

| Fact | Where |
|------|-------|
| The unwritten Scope and Verification | `ring_atomic/task/unverified/102_implement_ring_atomic.md:20-30` |
| Task ID allocation and index | `ring_atomic/task/readme.md` |
| 33 in `unverified/`, zero promoted | Census above |
| The four named `ring_tls` deliverables | `ring_tls/task/unverified/070_implement_ring_tls.md` |
| What `ring_tls` actually exports | `ring_tls/src/lib.rs:70`, `:294` |

### Tests

| Test | Covers |
|------|--------|
| *(not applicable)* | A task file's agreement with its crate is not testable from within the crate; it is a repository-level check, and none exists |
