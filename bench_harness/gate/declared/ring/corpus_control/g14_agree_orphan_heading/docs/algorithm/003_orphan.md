# algorithm: Orphan

### Scope

**Purpose:** Seed a finding heading that names no definition-readme row and no
Module Index row, so G14's cross-count-agreement assertion — the one `shape.py`'s
own header comment calls "the third assertion... the one that does the work" —
has a case it must refuse rather than merely a directory to notice missing.

**Responsibility:** One heading, deliberately not restated anywhere else in the
tree.

**In Scope:** This file.

**Out of Scope:** `algorithm/readme.md`'s "Findings Recorded Here" table and
`definition/readme.md`'s "Findings" table are both left exactly as they were
before this file existed — that omission is the seeded defect, not an oversight.

---

## The Half This Covers

```sh
echo '  -- the orphan half --'
printf '    five squared: %s\n' "$(( 5 * 5 ))"
```

Live output:

```
  -- the orphan half --
    five squared: 25
```

---

### CT5 — Recorded Here and Nowhere Else

The recipe above prints twenty-five, and the sentence above it says
twenty-five — the tree agrees with itself, exactly like CT1-CT4. What makes
this instance different is deliberate: this heading is the only place CT5 is
named. `algorithm/readme.md` does not carry a row for it, and neither does
`definition/readme.md`'s Module Index. `finding_owners()` still counts it,
because it scans headings, not tables — so G14 must report both a finding with
a heading and no definition-readme row, and, transitively, a finding absent
from the Module Index.

**Finding.** Deliberately orphaned — this is the seeded defect, not a real
finding about anything.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [001](001_first.md) | Not related — carries a clean, fully-declared finding |
| [002](002_second.md) | Not related — carries a clean, fully-declared finding |

### Sources

| Fact | Where |
|------|-------|
| Five squared | The recipe above |

### Tests

| Test | Covers |
|------|--------|
| `the_orphan_heading_has_no_home` | The half this file covers |
