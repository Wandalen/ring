# Workaround: The Manual Check That Named a Foreign Method

### Scope

- **Purpose**: Record the constraint that forces this crate's central guarantee to be checked by hand, and the drift that constraint has already produced once.
- **Responsibility**: Name what cannot be expressed in the language, show the negative-space checks that stand in for it, evidence the drift, and give the condition under which the checks can be deleted.
- **In Scope**: Why `tests/manual/readme.md` exists at all.
- **Out of Scope**: What the checks assert — see `tests/manual/readme.md` itself.

### The Constraint

The crate's load-bearing property is an **absence**: it must not contain the
load-and-fold that `ring_cursor::slowest` owns. The reason is stated in the
manual plan and is not stylistic —

> `ring_barrier`'s side of it must read at the same ordering, and two copies is
> exactly how one of them ends up `Relaxed`.
>
> — `tests/manual/readme.md` § M1

Rust has no way to say *this crate must not contain a fold over these cursors*.
There is no lint for "do not reimplement a function from a dependency", no
visibility rule that hides `&[ PaddedCursor ]`'s iterator while exposing the
slice, and no test that distinguishes a delegated minimum from a correct
hand-written one — they return the same value for every input.

So the crate absorbs the constraint as **source readings**: `grep` invocations
run by a person, whose expected result is nothing at all.

### The Negative-Space Checks

| Check | Asserts | Expected |
|-------|---------|----------|
| M1 | No fold, no indexing, no `.min(` | **no output** |
| M2 | No `Ordering::`, no `GATING`, no `SeqCell` | **no output** |
| M3 | One delegated call each; no arithmetic on `Seq.0` | 2 counted lines, then **no output** |
| M4 | `BatchTooLarge` before `Full` | two hits, ordered |
| M5 | One `map_or`, no `unwrap_or`, no `Seq::ZERO` | exactly one hit |
| M6 | Every declared dependency used | **no output** |
| M7 | The stall test present | non-zero, present |

Three of the seven expect nothing at all, and a fourth expects nothing from its
second command. **A check that passes by finding nothing passes equally well
when its pattern is wrong**, which is the cost this entry exists to record.

### The Drift, Evidenced

M1's pattern deliberately omits a bare `.map(`, and until this was corrected the
note explaining the omission read:

> `headroom` and `frontier` both `map` over the `Option` that `slowest` already
> returned

Both names were wrong:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'frontier' ring_gating/src/lib.rs
# 0
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs | grep -E '\.map'
# 224:    self.slowest().map_or( self.capacity.get(), | slowest |
# 323:    self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
```

Live output:

```
0
    self.slowest().map_or( self.capacity.get(), | slowest |
    self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
```

| Named | Reality |
|-------|---------|
| `frontier` | `ring_barrier`'s method. Zero occurrences in this crate's source |
| `headroom` | Spells its consumption `map_or`, which a bare `\.map\(` pattern does not match |

The one method the omission actually protects is `limit`, which the note did not
name. So the sentence justifying a deliberate gap in a check's pattern described
a method from another crate and a method the pattern would not have caught —
while the check itself was, and remains, correct.

**How it got there:** the plan's own run record says M1–M3 were rewritten when
the fold moved out of this crate into `ring_cursor::slowest`, shared with
`ring_barrier`. The rewrite carried the sibling crate's vocabulary across with
it. That is exactly the failure mode a check expecting *no output* cannot
surface: the grep kept returning nothing, so nothing ever contradicted the
prose.

The note now reads `limit`, and says why the other two do not apply.

### The Cost

| Cost | Detail |
|------|--------|
| Runs by hand | Seven greps, no CI step — the run record is dated entries, not a build |
| Silent when wrong | A negative-space check reports the same success for a broken pattern as for clean code |
| Prose drifts freely | Nothing verifies the *explanation*, only the pattern, and only when someone runs it |
| Sibling vocabulary leaks | `ring_barrier` and `ring_gating` share a fold and a shape; names cross with the shared code |

### The Condition for Deletion

| Route | Deletes | Blocked by |
|-------|---------|------------|
| Make the delegation structural — the cursors reachable only through a fold-shaped API | M1, M2 | `cursors()` is public and returns the slice; `ring_barrier::Barrier::over` needs it |
| A lint expressing "no second implementation of `ring_cursor::slowest`" | M1 | No such lint exists |
| Run M1–M7 in CI | Nothing, but converts silence into a build failure | A shell step nobody has written |

The third is the cheap one and it does not delete the workaround — it makes it
noisy instead of quiet, which for an absence check is the whole difference. The
first would genuinely delete M1 and M2, and it is blocked by a real requirement:
`ring_barrier` reads the same slice to ask the opposite question, so the slice
has to stay public.

That is the honest shape of this entry: **the workaround is not removable while
`ring_gating` and `ring_barrier` share a cursor slice**, and it should be made
loud rather than removed.

### GT55 — Three of Seven Checks Pass on an Empty Result

```
manual checks M1 .. M7 : 7
expecting no output     : 3   ( M1, M2, M6 )
```

An absence check reports the same thing when the code is clean and when the
pattern no longer matches anything. Three of the seven are in that shape, and
they are the three guarding the properties hardest to express any other way.

**Finding.** Three of the seven expect no output at all, and a fourth expects none from its second command — an absence check passes identically whether the code is clean or the pattern is wrong

---

### GT56 — The Note Named Everything but the Method It Protects

```
M1's note names : frontier   -> ring_barrier's; 0 occurrences here
                  headroom   -> its map_or would not have matched anyway
does not name   : limit      -> the one method the check protects
```

The note explains a deliberate gap in the pattern by listing what the pattern
skips. The item it exists to cover is not in the list.

**Finding.** It justified a deliberate gap in its pattern by naming `frontier`, which is `ring_barrier`'s method and appears zero times in this crate, and `headroom`, whose `map_or` the pattern would not have matched — the one method it protects, `limit`, went unnamed

---

### GT57 — A Check That Cannot See Its Own Subject

```
the check greps for a method name defined in ring_cursor
this crate cannot fail a build when ring_cursor renames it
=> rename -> pattern matches nothing -> check reports clean
```

The workaround exists because no lint expresses "do not reimplement your
dependency's fold". The substitute is a grep whose subject lives outside the
crate that runs it.

**Finding.** The check names a method on a type this crate does not own, so it goes stale on a rename in a crate it cannot see. A rename leaves the check reading plausibly and matching nothing — the same failure shape as a corpus recipe quoting line numbers into a file someone else is editing

---


### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | The same class of unenforceable property, with no check at all rather than a manual one |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_other_half_of_feature_178.md](../integration/002_the_other_half_of_feature_178.md) | The sibling whose vocabulary leaked, and why the slice stays public |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_this_crate_names_no_ordering.md](../invariant/002_this_crate_names_no_ordering.md) | The absence M2 checks, stated as a property |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_gate_must_never_over_report.md](../non_functional_requirement/002_the_gate_must_never_over_report.md) | The other place an assertion was weaker than its name |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_reversing_the_two_refusals.md](../pitfall/002_reversing_the_two_refusals.md) | M4, the one positive-space check, and why a structural read beats an outcome test |

### Workarounds

| File | Relationship |
|------|--------------|
| [002_the_multi_consumer_path_no_ring_uses.md](002_the_multi_consumer_path_no_ring_uses.md) | The other constraint absorbed for a consumer that does not exist yet |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/tests/manual/readme.md` § M1–M7 | The workaround itself |
| `ring_gating/src/lib.rs:224,323` | The two `map` sites the note described wrongly |
| `ring_cursor/src/lib.rs:120-123` | The fold that must not be copied |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` § Run Record | The dated entries that stand in for a CI step |
| `tests/gating_test.rs:133-147` | The automated check that covers three of the fold's positions, and no more |
