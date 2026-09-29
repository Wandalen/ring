# Associated Constant Items

### Scope

- **Purpose**: Catalog the crate's three associated constants — one origin value and two variant rosters.
- **Responsibility**: Give each one's type, its definition site, and its measured usage inside and outside the crate.
- **In Scope**: `Seq::ZERO`, `WaitKind::ALL`, `OverflowPolicy::ALL`.
- **Out of Scope**: The `impl` blocks holding them (→ [`../implementation/`](../implementation/)); the thirteen associated functions, which are a different kind and carry Caller/Callee trees this kind does not (→ [`../associated_function/`](../associated_function/)).

### Overview Table

| ID | Name | Kind | Defined In | Status |
|----|------|------|-----------|--------|
| 001 | [Seq::ZERO](001_seq_zero.md) | Associated Constant (A#2) | `src/id.rs:30` | 🔄 |
| 002 | [WaitKind::ALL](002_wait_kind_all.md) | Associated Constant (A#2) | `src/policy.rs:58` | 🔄 |
| 003 | [OverflowPolicy::ALL](003_overflow_policy_all.md) | Associated Constant (A#2) | `src/policy.rs:131` | 🔄 |

**These three are the crate's entire constant surface.** There is no free
`const` and no `static` anywhere in `ring_types`
(→ [`../readme.md`](../readme.md) § What the crate does not declare) — every
constant is associated with a type, which is the same shape the function
inventory has.

### No trees on this kind

`item_des.rulebook.md § Item Documentation : Required Sections` (OT006) attaches
`## Caller Tree` and `## Callee Tree` to function kinds only. A constant is not
called, so the five required sections are the whole of each instance here, and
`## Crate Usage` carries what a caller tree would have carried.

That is not merely a formatting rule — it changes what can be said. A function's
callers can be enumerated with an enclosing-function lookup; a constant's
*references* can be enumerated but its **uses** cannot, because
`OverflowPolicy::ALL.iter().map( … ).sum()` is one reference doing three things.
The counts below are reference counts and nothing more.

### Production references, measured

| Constant | `src/` lines matched | Non-doc | Crates (non-doc) | Test refs |
|----------|---------------------:|--------:|-----------------:|----------:|
| `Seq::ZERO` | 59 | **6** | 2 | 103 |
| `WaitKind::ALL` | **0** | 0 | 0 | 9 |
| `OverflowPolicy::ALL` | 4 | **1** | 1 | 22 |

```sh
cd "$(git rev-parse --show-toplevel)"
for id in 'Seq::ZERO' 'WaitKind::ALL' 'OverflowPolicy::ALL'; do
  a=$( command grep -rn "$id" ring_*/src   | command grep -v '^ring_types/' | wc -l )
  b=$( command grep -rn "$id" ring_*/src   | command grep -v '^ring_types/' | command grep -vc ':[0-9]*: *//' )
  c=$( command grep -rn "$id" ring_*/tests | command grep -v '^ring_types/' | wc -l )
  printf '%-22s src=%-4s non-doc=%-3s tests=%s\n' "$id" "$a" "$b" "$c"
done
```

Live output:

```
Seq::ZERO              src=59   non-doc=6   tests=103
WaitKind::ALL          src=0    non-doc=0   tests=9
OverflowPolicy::ALL    src=4    non-doc=1   tests=22
```

**The `src/` and non-doc columns disagree by an order of magnitude on `Seq::ZERO`
and that is the point of showing both.** 59 lines mention it; 6 execute it. The
other 53 are doc examples — the constant is the natural starting value in an
illustration, so every crate that documents a cursor writes `Seq::ZERO` and
almost none of them ever assigns it. Reporting 59 as usage would be the exact
error [`../readme.md`](../readme.md) § Where the counts come from warns about,
and it was made in two files of this catalog before being corrected.

**`WaitKind::ALL` has zero references in any sibling crate's source at all** —
not even a doc comment. It exists to be swept by tests, and only tests sweep it.

### The two `ALL` arrays are equally defended

Both are hand-written rosters that no language feature keeps in sync with their
enum, and both now carry the identical three-mechanism test: a length assert, a
per-variant `contains` loop, and a wildcard-free exhaustive `match`
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)
T6, closed). A duplicate entry preserving either roster's own variant-count
split now fails that array's `contains` loop.

**The two inline doc examples still differ in assertion style, but the
difference is now cosmetic rather than a defended/undefended split.**
`WaitKind::ALL`'s asserts a length; `OverflowPolicy::ALL`'s asserts a single
`contains`. Neither is the array's real defense — the suite's `contains` loop
is.

### Procedure

Adding or updating an instance: [`../procedure.md`](../procedure.md).
