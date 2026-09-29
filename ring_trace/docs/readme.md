# docs

Design documentation for `ring_trace`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Two executable statements — a branch that is the whole disabled feature, a scan that is the whole counting one |
| `api/` | Three types, eleven inherent methods, and reasons written where the author was rather than where a reader looks |
| `data_structure/` | Forty bytes and twenty-four — a vector that never allocated, and an entry carrying seven bytes of padding |
| `decisions/` | A flag frozen at construction and a lock chosen over a lock-free log, both argued and neither placed |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | No declarer, no importer, and a five-name vocabulary for work owned by five other crates |
| `invariant/` | Zero forever when disabled, and an order that is lock order rather than the one the doc sells |
| `item/` | Declaration-level reference — what the lint reaches, and which of two claimed tripwires is real |
| `lifecycle/` | Three states with one absorbing edge, and a finished crate that never left the first documentary stage |
| `non_functional_requirement/` | A self-imposed cost requirement nothing checks, and the family's one genuine `std` dependency |
| `pattern/` | Five lock sites behind one accessor, and an exhaustive match written as a tripwire |
| `pitfall/` | A range that prints backwards at the top of the sequence space, and a recovery no caller can reach |
| `type/` | Which of three declarations the compiler ties, and the `Sync` the whole design rests on |
| `workaround/` | An addition the types crate already names, and a dependency two documents still claim |

Scope of this crate: optional sequence-operation trace log.

Thirteen definitions, twenty-six instances, fifty-two findings — the full inventory,
with every finding's severity and owning instance, is
[`definition/readme.md`](definition/readme.md).

What the corpus keeps arriving at, from most of the thirteen directions
independently: the crate reasons carefully and correctly about cost, safety and
misreading, and reaches every one of those conclusions from the source rather
than from a measurement or a census — so each argument is right about the
algorithm and silent or wrong about the artefact. The module doc prices the
disabled path as "already a branch on a `bool`", which is true of the source and
2.07 ns per call short of the built artefact; it calls the lock "affordable",
which is also true, at seven times the disabled cost unshared and an order of
magnitude more under four producers, written down nowhere. The poisoning branch
is guarded, argued at length, and unreachable through the public API; the
unguarded `+` three lines away prints a range backwards from a public constant
the family publishes by name.
