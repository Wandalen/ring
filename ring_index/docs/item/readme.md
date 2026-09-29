# item

The three public functions read one at a time, as code items rather than as an
API surface: what each one's doc comment claims, what its doctest demonstrates,
and whether the two agree with the body between them. `api/` asks what the
surface is; this definition asks what each item says about itself.

The crate makes this an unusually productive question, because it has seventeen
lines of code and one hundred lines of doc comment. Everything a caller acts on
is prose. A sentence that is one lap too narrow is a defect of the same order as
an expression that is off by one, and both findings below are about sentences.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_fold_itself.md) | The Fold Itself | `of` — the only reached function, and the aliasing claim its comment states narrowly |
| [002](002_the_two_that_nothing_calls.md) | The Two That Nothing Calls | `aliases` and `run` — one comment that justifies its own zero callers correctly, one that cites the feature implemented by its replacement |

## The Same Property, Stated Twice, Forty Lines Apart

`of`'s comment says two sequences "exactly one lap apart" return the same slot.
`aliases`' comment, forty lines below, says "true exactly when they are a whole
number of laps apart." The second is the property; the first is a corollary of it
with the word "exactly" pointing the wrong way.

Which of the two is on the function people call is the whole finding. `of` has
callers in two crates and reaches a third transitively; `aliases` has none. The
crate states the general form only in the place nobody reads and the narrow form
in the place everybody does.

## Two Comments About Uselessness, One of Them Load-Bearing

`aliases` and `run` are both uncalled, and their comments handle it oppositely.
`aliases` explains that it is a testing affordance rather than machinery — a
claim that makes zero callers the expected outcome — and names the real gate in
another crate accurately enough to resolve two hops out. `run` cites
this family's own batch-claim contract as its reason to exist; the
crate that implements that contract imported `of`, declined `run`, and wrote a lazy
iterator instead.

The difference is not prose quality. It is that one comment's stated purpose
survives contact with the codebase and the other's is contradicted by it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\/\/\/ The slot a sequence addresses\.$/,/^\/\/\/ Two sequences exactly one lap apart return the same slot, which is the$/p;/^\/\/\/ Whether two sequences address the same slot — true exactly when they are a$/,/^\/\/\/ whole number of laps apart\.$/p;/^\/\/\/ The slots a contiguous run of `count` sequences starting at `start`$/,/^\/\/\/ addresses, in order\.$/p' ring_index/src/lib.rs
echo "  doc-comment lines / code lines: $( command grep -c '^\s*//[/!]' ring_index/src/lib.rs ) / $(( $( wc -l < ring_index/src/lib.rs ) - $( command grep -c '^\s*//[/!]' ring_index/src/lib.rs ) - $( command grep -c '^\s*$' ring_index/src/lib.rs ) ))"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX33 | `ring_index` | **misleading doc** | `of`'s comment says "exactly one lap apart," one lap narrower than the property; the general form is stated correctly on `aliases`, which nothing calls |
| IX34 | `ring_index` | n/a — observation | Seventeen lines of code carry 100 lines of doc comment and 188 lines of test, so a wrong sentence is as costly here as a wrong expression |
| IX35 | `ring_index` | n/a — observation | `aliases`' comment justifies its own zero callers coherently and its two-hop cross-crate reference resolves — the one in this crate that does — while no gate test uses it |
| IX36 | `ring_index` | n/a — drift | `run` cites this family's own batch-claim contract as its reason to exist; `ring_batch` implements that contract by importing `of` and writing `drain_order` instead |
