# Item Doc Definition

### Scope

- **Purpose**: Catalog every item `ring_types` defines — all forty of them — one file each, with the kind, the definition, and the measured usage inside and outside the crate.
- **Responsibility**: Be the exhaustive inventory the other twelve doc definitions can point at instead of restating signatures.
- **In Scope**: Items whose Defining Crate is `ring_types`: 4 modules, 6 use declarations, 3 structs, 3 enums, 8 implementations, 13 associated functions, 3 associated constants.
- **Out of Scope**: Items this crate merely *uses* — `core::fmt::Formatter`, `core::error::Error`, `u64`, `usize` — which belong to their own defining crates; the twenty-eight behavioural instances under the other twelve definitions, which argue about these items rather than cataloging them; and the two instances alongside this readme, which read the catalogue as a set rather than extending it.

### Type Declaration

- **Decision Criteria**: Use `item/` when the thing being documented is a *language item* in the Rust grammatical sense — a named declaration you could point at with a line number — rather than a behaviour, a rule, or a shape. No existing standard type fits: `api/` documents an exported surface as a whole, `type/` documents a type's validation rules, and neither has a place for a private module, a `use` declaration, or an `impl` block, all of which are items that exist and none of which are a surface or a rule.
- **Contrast with `type/` and `api/`**: `type/` answers *what must be true of a value of this type* and covers only the two types with rules to state; `api/` answers *what a consumer may rely on across the whole export*. `item/` answers neither — it answers *what exists, where, and who touches it*, for every declaration including the twelve that are private, unexported, or both. The three overlap on `Capacity` and `RingError` and diverge everywhere else: `item/` has thirty-eight files those two definitions have no reason to open.
- **Required Sections**: Representation, Kind, Definition, File Usage, Crate Usage
- **Optional Sections**: Caller Tree, Callee Tree
- **Overview Table Columns**: `ID`, `Name`, `Kind`, `Defined In`, `Status`
- **Quality Checklist**:
  - [ ] Does `## Kind` cite a numbered kind from `item_des.rulebook.md § Item Kind Taxonomy`, rather than a free-text label?
  - [ ] Do the File Usage line citations survive a re-run of the grep that produced them, against the current `src/`?
  - [ ] Does Crate Usage list every consumer crate, or state the measured zero — never omit the section because the answer is empty?
  - [ ] Do the two tree sections appear on exactly the fourteen function-kind instances and nowhere else?
  - [ ] Would deleting this file lose something no other doc definition records? (Documentation Necessity Test)

### Overview Table

| ID | Name | Kind | Defined In | Status |
|----|------|------|-----------|--------|
| — | []() | Module (#1) | `lib.rs` | 🔄 |
| — | [use_declaration/](use_declaration/) | Use Declaration (#3) | `lib.rs`, `capacity.rs`, `error.rs` | 🔄 |
| — | [struct/](struct/) | Struct (#6) | `capacity.rs`, `id.rs` | 🔄 |
| — | [enum/](enum/) | Enum (#7) | `error.rs`, `policy.rs` | 🔄 |
| — | [implementation/](implementation/) | Implementation (#12) | all four modules | 🔄 |
| — | [associated_function/](associated_function/) | Associated Function/Method (#1) | all four modules | 🔄 |
| — | [associated_constant/](associated_constant/) | Associated Constant (#2) | `id.rs`, `policy.rs` | 🔄 |

Seven subdirectories, forty instances. Each subdirectory's own `readme.md` carries
the per-instance Overview Table; this one indexes the kinds.

### What the crate does not declare

**Eleven of the eighteen taxonomy kinds have zero instances here**, and the list
is more characterizing than the seven that do:

| Absent kind | # | Why |
|-------------|---|-----|
| Extern Crate Declaration | 2 | 2018 edition; `[dependencies]` is empty anyway (→ [`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)) |
| **Function** | **4** | **Every callable in the crate is associated with a type. Not one free function.** |
| Type Alias | 5 | None |
| Union | 8 | None |
| **Constant** | **9** | All three constants are associated (`Seq::ZERO`, two `ALL` arrays) |
| Static | 10 | None — the crate holds no global state, which is what makes it `Sync` by construction |
| **Trait** | **11** | **The crate declares no trait.** It implements two foreign ones (`Display`, `Error`) and derives the rest |
| External Block | 13 | No FFI |
| Macro Definition | 14 | None |
| Macro Invocation | 15 | None at item position — `write!` and `matches!` appear inside function bodies, which is expression position, not item position |
| Associated Type | A#3 | No trait declared, and neither implemented trait has one |

**Read together, the absences say the crate is four types and the operations
hanging off them, and nothing else.** There is no free function to call without
naming a type, no trait to implement, no macro to expand, no static to
synchronise on, and no alias to indirect through. That is unusually narrow for
607 lines, and it is the reason `docs/api/001`'s export surface can be described
exhaustively in one table.

### The internal dependency graph is one edge

Filtering the crate's own type references down to code — excluding the ninety-odd
doc-comment mentions — leaves a graph small enough to state in four lines:

```sh
cd "$(git rev-parse --show-toplevel)"
cd ring_types
for id in Capacity Seq SlotIndex WaitKind OverflowPolicy RingError; do
  command grep -n "\b$id\b" src/*.rs | command grep -v ':[0-9]*: *//' | command sed -E 's/^([^:]+):[0-9]+:/\1:/'
done
```

Live output:

```
src/capacity.rs:pub struct Capacity( usize );
src/capacity.rs:impl Capacity
src/lib.rs:pub use capacity::Capacity;
src/id.rs:pub struct Seq( pub u64 );
src/id.rs:impl Seq
src/lib.rs:pub use id::{ Seq, SlotIndex };
src/id.rs:pub struct SlotIndex( pub usize );
src/id.rs:impl SlotIndex
src/lib.rs:pub use id::{ Seq, SlotIndex };
src/lib.rs:pub use policy::{ OverflowPolicy, WaitKind };
src/policy.rs:pub enum WaitKind
src/policy.rs:impl WaitKind
src/lib.rs:pub use policy::{ OverflowPolicy, WaitKind };
src/policy.rs:pub enum OverflowPolicy
src/policy.rs:impl OverflowPolicy
src/capacity.rs:use crate::RingError;
src/capacity.rs:  pub const fn new( slots : usize ) -> Result< Self, RingError >
src/capacity.rs:      return Err( RingError::CapacityZero );
src/capacity.rs:      return Err( RingError::CapacityNotPowerOfTwo( slots ) );
src/error.rs:pub enum RingError
src/error.rs:impl RingError
src/error.rs:impl fmt::Display for RingError
src/error.rs:impl core::error::Error for RingError {}
src/lib.rs:pub use error::RingError;
```

Every hit is one of three things: the item's own definition, its own `impl`
header, or a `pub use` in `lib.rs`. **Exactly one line is a genuine cross-module
reference — `capacity.rs`, `use crate::RingError;`** — and it is there because
`Capacity::new` returns a `Result`. The other three modules (`error`, `id`,
`policy`) reference nothing outside themselves but `core::fmt`.

So the module graph is:

```
lib ──► capacity ──► error ──► core::fmt
 ├────► error
 ├────► id
 └────► policy
```

**This is why the crate's compile is fast and its coverage gate is meaningful.**
There is no internal fan-out for a change to propagate through: touching
`policy.rs` cannot break `id.rs`, because nothing in `id.rs` can see it. The
corollary is the uncomfortable one — the four modules are not a decomposition of
a problem, they are four independent problems filed in one crate, and nothing
except the export Contract argues they belong together.

### Where the counts come from

Every File Usage and Crate Usage table in this subtree is generated, not recalled.
The two recipes:

```bash
# in-crate, per item — production lines only
cd ring_types && grep -n '\bIDENT\b' src/*.rs | grep -v ':[0-9]*: *//'

# consumer crates, workspace-wide
grep -rln '\bIDENT\b' ring_*/src | sed 's|ring/||;s|/src.*||' | sort -u
```

**Two hazards apply to both, and both have bitten this crate's docs already.**
The installed `grep` is `ugrep`, which groups multi-file output per file in an
order that varies between consecutive runs — so any quoted output is `| sort`ed
or reduced to a count. And a raw identifier count conflates doc examples with
production calls: `Capacity::new` has 113 references across `ring_*/src`,
of which 112 are documentation and comment references and 1 is a real call
(→ [`../pattern/002`](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).
The File Usage tables here separate the two rather than reporting the sum.

**State the scope with the number.** `ring_*/src --include='*.rs'` is what
this subtree uses; `pattern/002` measures the same identifier while *excluding*
`ring_types` itself and reports 105/104/1. The eight-reference difference is this
crate's own doc examples, and both figures are correct — a count without its
scope is not.

Widening from `/src` to the whole crate tree — `ring_*` — sweeps in
`tests/`, `tests/manual/readme.md`, and these doc files themselves: the same
`Capacity::new` count becomes 387, most of it this catalog quoting itself.

### Procedure

Adding or updating an instance: [`procedure.md`](procedure.md).


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/item
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               2
# finding headings inside:  8
# rows in the table below:  8
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY9 | ring family | n/a — coverage | `is_configuration`, `is_transient`, `reports_failure` and `drops_silently` are called from no library outside this crate; `is_non_blocking` is called once |
| TY10 | ring family | n/a — coverage | One of 113 references across `ring_*/src` is a call — TY20's fix removed the other; a reach figure that does not strip comment lines overstates this crate by roughly a hundredfold |
| TY11 | ring family | n/a — observation | The method that encodes the power-of-two invariant is the least-used thing the type exports, and both its callers use it for the same expression |
| TY12 | `ring_types` | n/a — observation | No free function, trait, static, type alias or macro — 40 items across 7 of the taxonomy 18 kinds, so a consumer cannot partially adopt the crate |
| TY13 | `ring_types` | n/a — inconsistency | `Capacity`'s field is private and `Seq`'s and `SlotIndex`'s are `pub`; one of three newtype invariants is in the type system and two are in prose |
| TY14 | ring family | n/a — unenforced | `Seq::next` and `advanced_by` are conveniences over an open field, so no monotonicity property of a received `Seq` can be relied on by its receiver |
| TY15 | ring family | n/a — inconsistency | Production code uses `Seq::ZERO` exclusively; the spelling the documentation demonstrates is the one that would stop compiling if the field were sealed |
| TY16 | ring family | n/a — observation | Against `RingError`'s 18 crates, `Seq`'s 16 and `Capacity`'s 11 — the least-adopted exported name, used only by the three crates owning the sequence-to-slot fold |
