# Algorithm: The Fixture

### Scope

- **Purpose**: Carry the seeded defect this fixture is named for.
- **Responsibility**: One reachable finding, one record finding.
- **In Scope**: Nothing real.
- **Out of Scope**: Everything real.

### Regenerate

```sh
echo 'counters that moved: 2'
```

Live output:

```
counters that moved: 2
```

### CX1 — Applied And Evidenced

**Disposition:** applied — both counters are written on the refusal path. Now prints: `counters that moved: 2`

### CX2 — Declined Concretely

**Disposition:** declined — the fix needs a generation counter in `RingSlot`, which `decisions/002` reserves and has not answered.

### CX3 — Recorded Only

A record tier, which this gate must ignore.
