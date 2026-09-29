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

### CX1 — Ruled On Twice

**Disposition:** applied — the counter is now written. Now prints: `counters that moved: 2`

**Disposition:** declined — on reflection `RingStats` should not carry it.

### CX2 — Recorded Only

A record tier, which this gate must ignore.
