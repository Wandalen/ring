# item

Fourteen public methods and one type when this was written; sixteen and two now. Read
as a list of named things rather than as code, the surface separates cleanly into five
recorders that write one counter each, seven readers that return a `u64`, `new`, and
`reset` — and in each half there is one member that does not belong with the others
despite looking exactly like them. The two later arrivals, `RingStats::snapshot` and
`StatsCounts::checked_in_flight`, join neither half: one returns the whole set, the
other reads a value the caller already holds.

The two instances here take one half each. On the write side, five recorders share a
single shape down to the explicit amount parameter, and one of them — `record_wait`,
whose counter this crate's own contract asked for by name — is called from nowhere in the
workspace. On the read side, six readers accumulate and the seventh observes, and the
two ways to obtain an all-zero set have opposite atomicity behind one-line docs a
clause apart.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_five_recorders_and_the_one_nothing_calls.md) | Five Recorders, and the One Nothing Calls | The write-side family, the explicit-amount contract, and `wait_nanos`' missing producer |
| [002](002_the_two_methods_that_are_not_one_operation.md) | The Two Methods That Are Not One Operation | `in_flight` among the counters, and `reset` against `new` |

## A Uniform Write Side With One Silent Member

Every recorder takes `&self` and a `u64` amount and adds it to one counter. None
increments by an implicit one, which is what makes `record_claim( 0 )` meaningful,
makes `n` singles provably equal to one batch, and lets `ring_bench` write a whole
run in four calls. Shape, contract and tests agree exactly — the baseline the rest of
this corpus is measured against.

`record_wait` shares all of it and is invoked by nothing outside this crate's own
tests. `ring_wait` spins, yields, and sleeps 50µs at a time, measures none of it, and
declares `ring_types` and `ring_cursor` — not `ring_stats`. So `wait_nanos()` returns
zero in every buildable configuration, and zero is a legitimate reading meaning
nothing waited.

## A Read Side Where One Name Asks a Different Question

`claimed`, `published`, `consumed`, `dropped`, `dropped_total`, `wait_nanos` — six
past participles and totals, each a monotone number answering how much has happened.
`in_flight` is present tense: a gauge, rising and falling, with no history and no
monotonicity, presented with the same return type, the same attribute, and a doc the
same length.

`new` and `reset` have the same relationship. Both produce an all-zero set; one is a
`const fn` building a value with no atomic operation in it, the other walks seven
counters through a shared reference. A caller choosing between them is choosing
between an atomic transition and a torn one, and nothing on the page says so.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the write side, and who calls each --'
for m in record_claim record_publish record_consume record_drop record_wait; do
  printf '    %-16s callers outside this crate: %s\n' "$m" \
    "$( command grep -rn "\.$m(" --include=*.rs | command grep -v '^ring_stats/' | wc -l )"
done
echo '  -- the read side, by name --'
command grep -o 'pub fn \(claimed\|published\|consumed\|dropped\|dropped_total\|wait_nanos\|in_flight\)' ring_stats/src/lib.rs
echo '  -- and the two routes to zero --'
command grep -n 'pub const fn new\|pub fn reset' ring_stats/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST25 | `ring_stats` | n/a — observation | All five recorders take `&self` and an explicit `u64` amount and add it to one counter, with no implicit-one form anywhere — which is what makes `record_claim( 0 )` a meaningful no-op, makes `n` single calls provably equal one batched call, and lets a caller write a whole run's totals in four calls; shape, contract and tests agree exactly, the one place in the crate where they do |
| ST26 | `ring_stats` | **latent hazard** | `record_wait` is invoked by nothing outside this crate — while `wait_nanos` is one of the four counters this crate was originally asked to provide, and `ring_wait`, the crate that spins, yields and sleeps 50µs at a time, measures none of it and declares `ring_types` and `ring_cursor` rather than `ring_stats` — so `wait_nanos()` returns zero in every buildable configuration and zero legitimately means nothing waited |
| ST27 | `ring_stats` | n/a — doc gap | Six readers are past participles or totals answering how much has happened, each one monotone; `in_flight` is present tense, a gauge that rises and falls with no history — and the API presents both kinds with the same return type, the same `#[ must_use ]`, and docs of the same length, so nothing separates the six that accumulate from the one that observes |
| ST28 | `ring_stats` | n/a — doc gap | `new` is a `const fn` building a value with no atomic operation in its body, and `reset` walks seven counters through a shared reference — the same all-zero postcondition reached atomically by one and observably torn by the other, eight lines apart in the same `impl` under one-line docs a clause apart, with nothing marking that a caller choosing between them is choosing a guarantee |
