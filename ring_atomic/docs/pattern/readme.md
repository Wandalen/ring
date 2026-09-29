# pattern

The crate has exactly two patterns, and they are the same idea applied twice.
Centralize where the family's atomics come from, and one file can swap the atomic
source for `loom`'s. Make one cell delegate to another rather than reimplement it,
and one type can stand in for the production type without becoming a fiction. Both
are good designs — the first is the best decision in the crate — and neither is
what the two instances here are about.

What they are about is that both designs are described by sentences claiming more
reach than the design has, and the overreach runs the same direction each time. The
creation-site paragraph says "every atomic" where it means every *sequence* atomic,
and says "instruments all of them" where centralized *creation* buys nothing for
observation. The counting cell's doc comment says "behaves exactly like `AtomicSeq`"
and argues the fidelity case at length, without ever naming the two axes — time and
space — on which the equivalence does not hold. In all three the design is sound
and the sentence is the defect.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_place_where_an_atomic_is_created.md) | One Place Where an Atomic Is Created | The single-creation-site pattern, the ten raw atomics outside it, and what centralizing creation does not buy |
| [002](002_the_counting_cell_is_not_a_mock.md) | The Counting Cell Is Not a Mock | Delegation instead of reimplementation, the one test the family's counting assertions rest on, and the fidelity claim nobody bounded |

## Creation Is Not Observation

The two words do different work and the crate uses one for both. Centralizing
*creation* means every `AtomicSeq` in the family is built by this file, so changing
what `AtomicSeq` wraps changes it everywhere at once — that is what makes the `loom`
seam a two-line edit rather than a thirty-three-crate edit, and it is completely
delivered. Centralizing *observation* would mean every atomic operation in the family
passes through something that can be watched, and that is not delivered, because
`counts` and `reset_counts` live on `CountingSeq` rather than on `SeqCell`. Only
code generic over the trait can be instrumented; a concretely-held `PaddedCursor`
cannot be, and `ring_stats`' seven counters are outside the seam entirely.

## Delegation Is What Makes the Substitution Honest

`CountingSeq` earns "not a mock" structurally rather than by assertion: it holds a
real `AtomicSeq` and all four of its methods increment a counter and then call
straight through to it. There is no scripted return value to diverge, no branch to
get wrong, no second implementation of `compare_exchange` to drift out of agreement.
This is the property that lets thirteen counting assertions in two other crates be
statements about production code rather than about a substitute for it — and the
crate's own doc comment makes the argument, correctly, in four lines.

The corresponding weakness is that the property is structural, so nothing forces it
to stay structural. One test — twenty lines, single-threaded, five fixed values —
checks that the two cells actually agree, and a future method that computed rather
than delegated would have to pass through it to be caught.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- pattern one: the claim, and the atomics standing outside it --'
command grep -m1 -A3 -F '//! the orderings are: this crate is the one place in 33 crates where an atomic' ring_atomic/src/lib.rs
command grep -rnE 'Atomic[A-Za-z0-9]+::new' --include=lib.rs ring_*/src/ \
  | command grep -v 'ring_atomic/\|AtomicSeq::new' | command grep -vE ':[0-9]+: *//' \
  | sed 's|/src/lib.rs||'
echo '  -- pattern two: delegation, not reimplementation --'
command grep -m1 -A26 -F 'impl SeqCell for CountingSeq' ring_atomic/src/lib.rs | command grep -E 'fn |self\.cell'
echo '  -- and the single test the downstream counting assertions rest on --'
command grep -n 'fn the_counting_cell_is_the_production_cell_plus_bookkeeping' \
  ring_atomic/tests/atomic_test.rs
printf '    counting assertions downstream : %s\n' \
  "$( command grep -rc 'counts()\.' --include=*.rs ring_batch/tests ring_tls/tests \
      | cut -d: -f2 | paste -sd+ | bc )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT37 | `ring_atomic` | **wrong doc** | "This crate is the one place in 33 crates where an atomic is *created*" is false — ten raw atomics live in `ring_stats`, `ring_shutdown` and `ring_bench` — and the unstated boundary hides a real question, since `ring_stats` holds seven concurrently-mutated `AtomicU64`s with zero `cfg( loom )` sites; inserting the word *sequence* makes the sentence true and the exception visible |
| AT38 | `ring_atomic` | **misleading doc** | "Instrumenting it here instruments all of them" holds for the `loom` swap and not for the counting instrument, which reaches only code generic over `SeqCell`; "no other crate needs to know the seam exists" holds for sibling crates and not for the workspace manifest above them, which must carry the `check-cfg` entry |
| AT39 | `ring_atomic` | n/a — coverage | Thirteen counting assertions across `ring_batch` and `ring_tls` are licensed by one twenty-line single-threaded parity test over five fixed values, which never exercises `fetch_add( 0 )`, the `u64::MAX` wrap, a `compare_exchange` whose `current` equals `new`, any other ordering, or any concurrency |
| AT40 | `ring_atomic` | **misleading doc** | The doc comment argues the substitution's fidelity at length — "behaves exactly like `AtomicSeq`", "the same values production would" — and bounds it nowhere, while the type costs ~1.9×/~2.7× per call and 40 bytes against 8, is exported with no feature gate, no `cfg( test )` and no `doc( hidden )`, and is kept out of production by convention across thirteen construction sites |
