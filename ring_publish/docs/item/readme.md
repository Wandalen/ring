# item

The six methods taken one at a time — three that read the cursor, two that move
it, and the constructor covered with the type it constructs. Where
[`api/`](../api/readme.md) treats the surface as a whole, this directory treats
each item as its own object with its own contract, call-site census, and reason
for existing.

Both files converge on the same question from opposite sides: the surface carries
a *derivable* method on each half — `is_published`, which is `seq < published()`,
and `publish`, which is `try_publish` in a loop — and neither is redundant for the
reason it first appears to be.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Three Readings of the Cursor](001_the_three_readings_of_the_cursor.md) | PB21, PB22 — `cursor`/`published`/`is_published` side by side, ten call sites that are one expression repeated verbatim, and the six cursor accessors in the family of which none is read-only |
| 002 | [The Two Publications](002_the_two_publications.md) | PB23, PB24 — `try_publish` and `publish` contract by contract, the family's four `# Panics` sections, and the single branch repo-wide on the `Result` that exists to be branched on |

### The Six

| Item | Reads | Writes | Atomics | `const` | Derivable from |
|------|:-----:|:------:|:-------:|:-------:|----------------|
| `new` | — | — | 0 | no | `default` |
| `cursor` | the address | — | **0** | **yes** | nothing |
| `published` | the value | — | 1 load | no | `cursor` + a load |
| `is_published` | the value | — | 1 load | no | **`published`** |
| `try_publish` | — | the cursor | 1 RMW | no | nothing |
| `publish` | — | the cursor | 1+ RMW | no | **`try_publish`** |

The derivability chain runs strictly one way on each half:

```
cursor()  ──load(GATING)──▶  published()  ──` < `──▶  is_published( seq )
try_publish()  ──loop──▶  publish()
```

Left to right each step is one operation on the value before it. Right to left is
impossible at every step — a `bool` does not determine a `Seq`, a `Seq` does not
determine an address, and a value that always succeeded does not tell you what a
refusal would have returned.

### Why Each Redundant Method Is Kept

| | `is_published` | `publish` |
|--|----------------|-----------|
| Could be written at the call site | `seq < published()` | a three-line loop |
| Kept because | it names the requirement the feature is graded on | the refusal decision belongs in one place |
| And because | the exclusive-boundary convention lives in exactly one comparison operator | it is the crate's only entry in the family's waiting-primitive census |
| And because | the 320-assertion test would otherwise assert a tautology | — |
| Its sibling would otherwise be | the only read of the value | private, costing eleven tests that pin the refusal semantics |

The last row is the sharper argument for keeping *both* halves public. `publish`
genuinely needs `try_publish` as its body; nothing needs `try_publish` on the
public surface — except the tests that are how the refusal decision is checked
behaviourally at all. The public `try_publish` is a testability seam documented as
a caller affordance, and only the first role has ever been exercised.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_publish

# the ten identical call sites
grep -hoE '.{0,45}publisher\.cursor\(\).{0,12}' tests/*.rs | sort | uniq -c

cd "$(git rev-parse --show-toplevel)"

# every cursor accessor in the family, and any read-only cursor type
grep -rnE 'fn [a-z_]+\([^)]*\) *-> *(&|Option< *&) *PaddedCursor' ring_*/src/*.rs
grep -rnE 'pub (struct|trait) .*(Reader|ReadOnly|View)' ring_*/src/*.rs

# the family's documented-failure census
grep -rc '# Panics' ring_*/src/*.rs | grep -v ':0'
grep -rc '# Errors' ring_*/src/*.rs | grep -v ':0'

# every site that actually branches on try_publish
grep -rnE '(if let|match|\.is_ok\(|\.is_err\(|\?)' --include='*.rs' */ | grep 'try_publish'
```

| | Value |
|--|------:|
| Public methods | 6 |
| …that read | 3 |
| …that write | 2 |
| …that perform no atomic operation | 1 — `cursor` |
| `publisher.cursor()` call sites in tests | 13 |
| …that are one expression repeated verbatim | **10** |
| Cursor accessors in the family | 6, across 5 crates |
| …handing out a writable handle | **6** |
| Read-only cursor types in the family | **0** |
| `# Errors` sections family-wide | 49, in 26 files |
| `# Panics` sections family-wide | 4, in 3 files |
| …reading "Never" | 2 — both waiting loops |
| …that name a worse outcome than a panic | **1** |
| `try_publish` call sites in tests | 11 — every one inside an `assert_eq!` |
| Sites repo-wide branching on its `Result` | **1** |
| `publish` call sites in tests | 21 |
| …discarding the return value | 15 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB21 | `ring_publish` | n/a — duplication | Ten of thirteen `cursor()` call sites are one expression repeated to the character; the accessor has exactly one real use and the crate never abstracted it, because the helper that would collapse them can only live in the test file and the file's three helpers stop short of an eleventh |
| PB22 | family | n/a — unenforced | Six cursor accessors across five crates all hand out the same mutable-through-shared handle, and there is no read-only cursor type anywhere in the family; the hole is not local to this crate, and nothing checks it in any crate |
| PB23 | family | n/a — observation | The family writes 49 `# Errors` sections and 4 `# Panics`; two say "Never", and the difference is that a bounded loop's "Never" is complete in one sentence while this one needs three, because the honest answer is *"no — it does something worse"* |
| PB24 | `ring_publish` | n/a — observation | One site repo-wide branches on `try_publish`'s `Result`, and it is `publish`, the method whose purpose is to make branching unnecessary; of eighteen sites, fifteen assert the value and two discard it |
