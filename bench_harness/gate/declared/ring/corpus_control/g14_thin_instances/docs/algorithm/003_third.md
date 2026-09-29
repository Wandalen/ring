# algorithm: Three

### Scope

**Purpose:** Compensate `invariant/`'s missing second instance so the corpus'
aggregate totals (4 instances, 4 findings) stay identical to
`g14_definition_present_clean` — the seeded defect is a per-definition
imbalance, not a change to what the whole tree adds up to.

**Responsibility:** One recipe, one finding, one cited test — same shape as
[001](001_first.md) and [002](002_second.md).

**In Scope:** This file.

**Out of Scope:** The other two halves of `algorithm/`, and `invariant/`'s own
single remaining instance.

---

## The Half This Covers

```sh
echo '  -- the third half --'
printf '    four plus one: %s\n' "$(( 4 + 1 ))"
```

Live output:

```
  -- the third half --
    four plus one: 5
```

---

### CT5 — The Tree Agrees Here As Well

The recipe above prints five, and the sentence above it says five.

**Finding.** Recorded as an observation, because nothing here is reachable —
same as CT1 and CT3.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [001](001_first.md) | The first half |
| [002](002_second.md) | The second half |

### Sources

| Fact | Where |
|------|-------|
| Four plus one | The recipe above |

### Tests

| Test | Covers |
|------|--------|
| `the_third_branch_is_taken` | The half this file covers |
