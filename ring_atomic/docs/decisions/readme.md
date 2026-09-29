# decisions

The crate's module comment argues for two things: that orderings are always the
caller's to name, and that the cell is a trait rather than a struct so a counting
implementation can be substituted for a production one. Both arguments are written
out, both are checkable, and both check out — the second completely, the first
within a scope narrower than the paragraph around it implies.

What is missing is everything else. The crate makes four more decisions with real
consequences — nine `Relaxed` literals on its own bookkeeping, five atomics packed
onto one cache line, no `must_use` on the trait, no `Sync` supertrait — and records
none of them. The two decisions with a paragraph are the two that are hardest to
get wrong; the four without are where every reachable hazard in this corpus lives.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_orderings_named_never_defaulted.md) | Orderings Named, Never Defaulted | The claim's true scope, the nine literals outside it, and why no ordering would help |
| [002](002_a_trait_because_the_criteria_needed_two.md) | A Trait, Because Two Criteria Needed Two Implementations | The argument, both criteria as written, and the fourteen assertions built on them |

## The One That Paid Off

The `ring_tls` and `ring_batch` criteria both make negative claims about atomic traffic, and a
negative claim about a bare `AtomicU64` is unassertable from outside — the count
simply is not observable. `SeqCell` is the answer to that, and the answer worked:
both criteria's named test files drive the code under test through `CountingSeq`
and assert on `counts()`, fourteen assertions in four files, none of them writable
without the trait.

That is a design decision with an argument, a forcing requirement, and delivered
evidence, all reachable from eleven lines of module comment. It is the best-
documented thing in the crate.

## And the Seam Inside It

The `ring_batch` criterion asks for a *fence* count. The shim reports a *call* count, and each
counted call issues two hardware read-modify-writes rather than one — the
instrument changes the quantity it reads. For the amortisation argument the crate
actually cares about, the call count is the right number and the criterion's
wording is the thing that is off. Nothing in either document says so.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two documented decisions --'
command grep -n '^//! ## ' ring_atomic/src/lib.rs
echo '  -- the ordering choices the crate makes for itself --'
command grep -cE 'Ordering::(Relaxed|Acquire|Release|AcqRel|SeqCst)' ring_atomic/src/lib.rs
echo '  -- and the crates the trait decision was made for --'
command grep -rl 'CountingSeq' --include=*.rs */ | sed -E 's|/.*||' | sort -u
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT13 | `ring_atomic` | **misleading doc** | "This crate never picks one on a caller's behalf" is true of the caller's cell and silent about the crate's own nine `Relaxed` literals — including the four at `:310-313` that make the torn snapshot possible, in a crate whose stated purpose is to make the family's ordering decisions readable in one place |
| AT14 | `ring_atomic` | **measured cost** | `SeqCst` on every counter operation does not close the torn-snapshot window: ten runs, ten non-zero counts, no ordering effect, and its widest observed skew the largest of the twenty measured — the question is not an ordering question, which nothing states |
| AT15 | `ring_atomic` | n/a — observation | The trait-versus-struct decision is fully realized: both cited criteria exist as quoted, both name a test file that uses `CountingSeq`, and fourteen assertions across four files depend on it — the crate's only decision with a written argument and delivered evidence |
| AT16 | `ring_atomic` | **misleading doc** | The `ring_batch` criterion asks for "one fence, not 64" and is asserted as one *call*, which the shim's own bookkeeping turns into two hardware atomics — six of the fourteen assertions carry messages naming atomics or operations where they measure calls |
