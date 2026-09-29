# docs

Design documentation for `ring_event`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Two executable statements, one branch, and a dispatch resolved before the program runs |
| `api/` | Two traits, three functions, one associated type — and a `Result` one impl cannot return |
| `data_structure/` | A crate that declares no data, and the residue the reset it calls leaves behind |
| `decisions/` | Which side each trait sits on, and why the three operations are functions rather than methods |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | One declaring manifest, under dev-dependencies, and the ring that writes slots without it |
| `invariant/` | Two readings of one emptiness, and a refusal that changes nothing |
| `item/` | Declaration-level reference — five public items, four impls, and their call traffic |
| `lifecycle/` | Four slot positions with three representations, and a finished crate marked unverified |
| `non_functional_requirement/` | An indirection the linker erases, and a `core`-only closure only one of whose three crates says so |
| `pattern/` | A trait on the varying side with one function over it, proved by monomorphisation |
| `pitfall/` | A zero-length payload that reads as nothing, and a recycled slot that is not equal to an empty one |
| `type/` | The family's only generic associated type, and a blanket impl over every type there is |
| `workaround/` | A slicing expression forced at every generic call site, and a dependency two documents still name |

Scope of this crate: slot translators that fill a claimed slot.

Thirteen definitions, twenty-six instances, fifty-two findings — the full inventory,
with every finding's severity and owning instance, is
[`definition/readme.md`](definition/readme.md).

What the corpus keeps arriving at, from most of the thirteen directions
independently: the abstraction is real, correct and unused, and almost everything
true about it is written one level away from where it is needed. Two hundred and
four lines declare half the family's public traits and all of its generic
associated types; the suite proves this crate's parity claim structurally rather
than by assertion; the indirection costs nothing measurable and nothing in the
emitted assembly; and exactly one manifest in thirty-three declares the crate,
under `[dev-dependencies]`. The ring that would carry the parity claim
monomorphises over `Slot` without ever naming this crate — so the strongest proof
in the corpus is of a property whose subject is one crate away.
