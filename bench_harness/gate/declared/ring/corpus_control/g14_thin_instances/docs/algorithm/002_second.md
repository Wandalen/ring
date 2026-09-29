# algorithm: Two

### Scope

**Purpose:** Give the corpus gates a second instance, so a per-definition
instance floor of two has something to clear.

**Responsibility:** One recipe, one finding, one cited test.

**In Scope:** This file.

**Out of Scope:** The first half is [001](001_first.md).

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

### CT2 — The Tree Agrees Here Too

The recipe prints nine and the sentence says nine.

**Finding.** Recorded as a latent hazard so the fixture carries one reachable
tier and one record tier, which is what makes a hybrid distinguishable from
either.

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
