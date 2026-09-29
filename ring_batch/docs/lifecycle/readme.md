# lifecycle

A `BatchClaim` has exactly one state transition and it happens before the value
exists: `fetch_add` moves the cursor, the range comes back, and from that instant
nothing can change it. There is no `Drop`, no release, no rollback, and no
mechanism anywhere in the crate for a claim that is taken and then not used.

That is the right design at this tier — a range with no ring reference cannot
commit itself, and subtracting from the cursor would hand the same sequences to
two producers. It is also undocumented here. The rule appears one crate up, in a
subordinate clause of `ring_tls::flush_into`'s doc comment, and the two ring
assemblies each solved the same problem by defining their own range type with a
destructor attached.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_no_lifecycle_and_no_rollback.md) | No Lifecycle and No Rollback | The missing destructor, the four that exist elsewhere, and where the rule is actually written |
| [002](002_the_empty_claim_as_a_first_class_state.md) | The Empty Claim as a First-Class State | Zero at every entry point, the one guard that had to be hand-written, and what an empty flush costs |

## The One State the Type Does Model

Zero is legal everywhere. `claim( cursor, 0, order )` returns an empty claim,
`claim_gated` grants one on a completely full ring, and all eight methods behave
sensibly on it — five because `start..start` is naturally empty, one because
`overlaps` carries an explicit guard without which an empty range strictly inside
another reports an overlap that is not there.

Five tests cover this state, more than cover any other single behaviour in the
crate, and the clearest statement of why it exists lives in one of them:
"Treating it as one would make every flush of an empty buffer look like a
protocol violation."

## Free as a Value, Not Free as an Operation

`ring_tls::flush_into` takes the empty path unconditionally, and documents that
choice: checking on the caller's behalf would make the operation count depend on
the data, which would break the counting shim the crate's own doctests rely on.
The doc warns that a zero-length `fetch_add` "still costs one atomic."

Measured, that atomic is not paid where the doc implies. A single thread flushing
nothing in a loop doubles a working producer's per-claim cost — 22.24 ns against
11.36 ns — because `fetch_add(0)` takes the cursor's cache line exclusively
exactly as a real claim does. The argument is zero; the memory traffic is not.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- no lifecycle here --'
echo "    Drop/release/cancel/rollback in ring_batch : $( command grep -c 'Drop\|release\|cancel\|rollback' ring_batch/src/lib.rs || true )"
echo '  -- four elsewhere --'
command grep -rn '^impl.*Drop for' --include=lib.rs ring_*/src/ | sed 's|ring/||'
echo '  -- the rule, one crate up --'
command grep -m1 -A1 -F '  /// [`Flush`] is fully consumed, because the sequences are already claimed:' ring_tls/src/lib.rs
echo '  -- and the empty path it takes without checking --'
command grep -m1 -A2 -F '  /// Flushing an empty buffer is legal and claims nothing — a zero-length' ring_tls/src/lib.rs
command grep -m1 -F '    let claim = claim( cursor, self.items.len(), order );' ring_tls/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA30 | `ring_batch` | n/a — doc gap | A claim is irreversible the instant it is returned, and the crate never says so; the rule appears only in `ring_tls::flush_into`'s doc, as a justification for something else |
| BA31 | `ring_mpsc` | n/a — duplication | Four range types across four crates — two with a committing `Drop`, two without — and nothing records that a range with no ring reference cannot have one |
| BA32 | `ring_batch` | n/a — observation | Zero is legal at both entry points and correct across all eight methods; five behaviours fall out of the arithmetic and only `overlaps` needed a hand-written guard |
| BA33 | `ring_tls` | **measured cost** | One thread flushing an empty buffer in a loop doubles a working producer's per-claim cost (22.24 ns against 11.36 ns); the doc warns of "one atomic" and does not say whose |
