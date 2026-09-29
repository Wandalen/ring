# algorithm: One

### Scope

**Purpose:** Give the corpus gates one instance whose recipe reproduces and
whose citation resolves, so that a fixture failing tells them which.

**Responsibility:** One recipe, one finding, one cited test.

**In Scope:** This file.

**Out of Scope:** The second half is [002](002_second.md).

---

## The Half This Covers

```sh
echo '  -- the first half --'
printf '    two plus two: %s\n' "$(( 2 + 2 ))"
```

Live output:

```
  -- the first half --
    two plus two: 5
```

---

### CT1 — The Tree Agrees With the Sentence

The recipe above prints four, and the sentence above it says four. That is the
whole of what this fixture instance establishes.

**Finding.** Recorded as an observation, because nothing here is reachable.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [002](002_second.md) | The second half |

### Sources

| Fact | Where |
|------|-------|
| Two plus two | The recipe above |

### Tests

| Test | Covers |
|------|--------|
| `the_first_branch_is_taken` | The half this file covers |
