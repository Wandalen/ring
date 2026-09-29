# pattern

Two shapes carry this crate. The first is the range object — a start, a length,
and a method surface computed entirely from those two numbers — which the family
has written four times, twice with a ring reference attached and a committing
destructor, twice without. The second is the gated/ungated split: two public
functions with different signatures and different failure models, rather than one
function with an optional barrier.

Both decisions are right, and both are recorded in a way that outlives the
conditions they were made under. `ring_claim::Claim` is `BatchClaim` with one
field renamed, arrived at independently three tiers up with no dependency edge in
either direction. The split's stated reason names two crates that turned out not
to use this one at all.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_range_object.md) | The Range Object | Four types, two shapes, an identical eight-method surface, and one attribute that differs |
| [002](002_two_functions_where_one_would_have_hidden_it.md) | Two Functions Where One Would Have Hidden It | The gated/ungated split, its predicted callers, and the struct form one tier up |

## Convergence Is Evidence and It Is Also a Bill

Eight method names — `new`, `start`, `len`, `is_empty`, `end`, `contains`,
`sequences`, `overlaps` — appear on both `BatchClaim` and `ring_claim::Claim`,
same spelling, near-identical order, with no import between the crates. That
agreement is the strongest available evidence the shape is correct: two authors
solving the same problem reached for the same words.

It is also four implementations of a Tier 2 type in a family that has a Tier 2
crate for exactly that. The bill comes due at the edges: `ring_claim::Claim`
carries a type-level `#[ must_use ]` whose message states the irreversibility
rule `ring_batch` never writes down, and `ring_mpsc` carries two committing
destructors with no `must_use` at all, while `ring_spsc` guards its identical
pair.

## A Reason Should Name a Mechanism, Not a Caller

`claim`'s doc justifies the split by naming the SPSC and MPSC paths. Neither
depends on `ring_batch`. The one crate that does — `ring_tls` — imports
`{ claim, BatchClaim }` and takes the ungated path, which confirms the cheap
half's reason and leaves the expensive half with eleven call sites, all inside
this crate's own tests.

The mechanism the doc could have named instead is durable and still true: a
caller that already knows its consumer's position should not pay to read it
again. That sentence would have survived the callers moving.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the same eight methods, two crates --'
for m in new start len is_empty end contains sequences overlaps
do
  printf '    %-12s ring_batch %s   ring_claim %s\n' "$m" \
    "$( command grep -c "fn $m(" ring_batch/src/lib.rs || true )" \
    "$( command grep -c "fn $m(" ring_claim/src/lib.rs || true )"
done
echo '  -- type-level must_use across the family --'
command grep -rn 'must_use = ' --include=lib.rs ring_*/src/ | sed 's|ring/||'
echo '  -- the split, and who took it --'
command grep -m1 -A2 -F '/// exist because the SPSC path knows its own consumer and the MPSC path does' ring_batch/src/lib.rs
command grep -rln 'ring_batch' --include=Cargo.toml . | sed 's|ring/||;s|/Cargo.toml||' | sort
command grep -n 'use ring_batch' ring_tls/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA38 | `ring_claim` | n/a — duplication | `BatchClaim` and `ring_claim::Claim` expose eight identically-named methods on two identically-shaped fields, three tiers apart, with no dependency edge and no mention of each other |
| BA39 | `ring_batch` | n/a — doc gap | `ring_claim::Claim` carries a type-level `#[ must_use ]` whose message states the irreversibility rule `ring_batch` never writes; the same attribute would also close the crate's one silent-drop gap |
| BA40 | `ring_batch` | n/a — observation | The split's stated reason names the SPSC and MPSC paths; neither depends on this crate, and the one that does imports only the ungated half |
| BA41 | `ring_claim` | n/a — observation | The same gate is a free function here and a struct one tier up; the struct owns its producer cursor, which makes the same-cell-twice mistake unrepresentable |
