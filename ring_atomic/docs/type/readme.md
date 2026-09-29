# type

The crate's public surface is three types and one trait, and they divide cleanly:
`SeqCell` is the contract everything upstream is written against, `AtomicSeq` and
`CountingSeq` are the two implementations behind it, and `OpCounts` is the only value
that ever comes back out. The two instances here take the trait and the report — the
two ends of the surface — and ask what each one actually obliges or offers.

Both answers come out the same way. What the trait requires is less than what every
one of its users needs, and what the report carries is more than any of them keeps.
`SeqCell` declines to demand `Sync`, which all three generic bounds in the family
silently depend on, and declines to mark its returns `#[ must_use ]`, including the
one whose loss strands a slot range permanently. `OpCounts` bundles five values that
every downstream consumer unbundles on the same line, paying for the bundling with a
four-read tear and a field that can contradict its own operands.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_what_the_trait_promises.md) | What the Trait Promises a Caller | The absent supertrait, the three unmarked returns, and the one contract clause in the file |
| [002](002_the_report_that_is_all_public.md) | The Report That Is All Public | Six derives, five public fields, no inherent impl, and 16 downstream reads that never keep a value |

## Requirements Satisfied by Accident

`SeqCell`'s every method takes `&self` and mutates, which is the shape of a type
meant to be shared — and also the shape of `Cell< u64 >`, which must never be. A
`Cell`-backed implementation satisfies the trait and is accepted by `ring_batch::claim`,
the family's multi-producer entry point. Nothing catches it until something
independently demands `Sync`, and the error then names the offending private field
rather than the trait's expectation. Both real implementations are `Sync` by
auto-derivation from the atomics inside them, so the requirement is met by composition
rather than by declaration and the gap has never been reached.

## The Attribute Spent Backwards

`ring_atomic` uses `#[ must_use ]` five times: four constructors and `counts()` — every
one on a value the caller could obtain again by calling again. `ring_types`, one crate
down, uses it eleven times, including on `next`, `advanced_by` and `distance_to`, pure
functions over a `Copy` type. Between them the family marks sixteen recoverable
returns and zero irrecoverable ones, while `SeqCell::fetch_add` — whose return value
is the caller's only record of which sequences it now owns — may be discarded in
silence.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the trait, and what bounds it in 33 crates --'
command grep -m1 -A1 -F 'pub trait SeqCell' ring_atomic/src/lib.rs
command grep -rn ': SeqCell' --include=*.rs */ | command grep -v 'ring_atomic/' | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
echo '  -- must_use, spent here and one crate down --'
printf '    ring_atomic : %-3s  ring_types : %s\n' \
  "$( command grep -c 'must_use' ring_atomic/src/lib.rs || true )" \
  "$( command grep -rc 'must_use' ring_types/src/ | cut -d: -f2 | paste -sd+ | bc )"
echo '  -- and what downstream does with the one value that comes back --'
printf '    field reads %-4s whole-value uses %s\n' \
  "$( command grep -rho 'counts()\.[a-z_]*' --include=*.rs ring_batch ring_tls | wc -l )" \
  "$( command grep -rho 'counts()[^.]' --include=*.rs ring_batch ring_tls | wc -l )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT45 | `ring_atomic` | **latent hazard** | `SeqCell` declares no supertrait, so a `Cell< u64 >`-backed implementation satisfies it and is accepted by `ring_batch::claim`, the family's multi-producer entry point; all three generic bounds in 33 crates are bare `C : SeqCell`, and both real implementations are `Sync` only by auto-derivation — `pub trait SeqCell : Sync` states the actual requirement at zero cost |
| AT46 | `ring_atomic` | **latent hazard** | `fetch_add` returns "the first sequence the caller now owns" and discarding it compiles silently, permanently stranding that range; only `compare_exchange` warns, and only because std marks `Result` — meanwhile the crate spends all five of its `#[ must_use ]` on four constructors and `counts()`, and `ring_types` spends eleven more on pure functions over a `Copy` type |
| AT47 | `ring_atomic` | n/a — observation | `OpCounts` carries `PartialEq`/`Eq` so a caller can assert a whole shape at once, and downstream takes that option zero times across 16 field reads; all nine whole-value uses are in `ring_atomic`, the three comparisons all in its own test file hand-writing `total` — while the bundling is what forces four separate reads and what creates a field able to contradict its operands |
| AT48 | `ring_atomic` | n/a — observation | The crate gives its two cells four `new` functions and two hand-written `Default` impls, and its one report no inherent impl at all — correct for a record, but `OpCounts` is a *reading*, and `Copy` with public fields leaves a value from a live cell indistinguishable from a hand-written literal and a snapshot taken under contention indistinguishable from one taken quiet |
