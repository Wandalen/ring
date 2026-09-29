# invariant: Four

### Scope

**Purpose:** Give the corpus gates a second instance, so a per-definition
instance floor of two has something to clear — but deliberately no finding, so
this fixture's seeded defect is the total finding count falling below the
declared floor while every definition and instance count stays clean.

**Responsibility:** One recipe, one cited test, and — unlike every other
instance in this fixture family — no finding heading at all.

**In Scope:** This file.

**Out of Scope:** The first half is [001](001_first.md), which still carries
CT3.

---

## The Half This Covers

```sh
echo '  -- the second half --'
printf '    three times three: %s\n' "$(( 3 * 3 ))"
```

Live output:

```
  -- the second half --
    three times three: 9
```

---

### Observation — Not Recorded As a Finding

The recipe prints nine and the sentence says nine, same as every other half in
this fixture. What is deliberately different here: this heading does not match
`finding_owners()`'s `### <ID> — ` pattern, so it contributes an instance
without contributing a finding. That is the seeded defect — `min_findings`
counts findings, not instances, and a corpus can hold every declared
definition and clear every instance floor while still falling short of it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [001](001_first.md) | The first half |

### Sources

| Fact | Where |
|------|-------|
| Three times three | The recipe above |

### Tests

| Test | Covers |
|------|--------|
| `the_second_branch_is_taken` | The half this file covers |
