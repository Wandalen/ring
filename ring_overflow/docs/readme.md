# docs

Design documentation for `ring_overflow`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Two `match` blocks over three policies, and where the outcome the consumer discards goes |
| `api/` | Four declarations, two predicates with no caller outside the crate, and the inverse that is absent |
| `data_structure/` | One byte with 253 spare niches, returned inside twenty-four |
| `decisions/` | The fourth variant that is not there, and the outcome sent out through the error channel |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | One consumer spending three lines on four functions, and a dependency edge nothing traverses |
| `invariant/` | A total, injective mapping, with the counts guarded and the readings not |
| `item/` | Declaration-level reference — two free functions one statement apart, and the enum's two readings |
| `lifecycle/` | A value that lives one expression, and a task that says the crate is unbuilt |
| `non_functional_requirement/` | The measured 7× nothing pays, and three `core`-only crates that never say so |
| `pattern/` | A naming convention with a sample size of one, and a type argued for on the weaker ground |
| `pitfall/` | The variant no shipping build can construct, and the refusal counted as a loss |
| `type/` | Six derives on a fieldless enum, and three variants encoding two questions |
| `workaround/` | An atomic two crates away, and one outcome expressed in two type systems |

Scope of this crate: full-ring overflow policies.

Thirteen definitions, twenty-six instances, fifty-two findings — the full inventory,
with every finding's severity and owning instance, is
[`definition/readme.md`](definition/readme.md).

What the corpus keeps arriving at, from most of the thirteen directions
independently: the 237 lines are correct throughout, and almost everything they are
careful about is unreachable in a shipping build. `EvictedOldest` is the sharp case
— no shipping configuration can construct it, it is the only value making
`accepted_incoming()` true, and it is the only witness keeping the crate's central
safety test non-vacuous.
