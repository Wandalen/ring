# item

Seven named members: five stored fields and two derived readings. Read as a list
of things rather than as a surface, they sort by how much of each one anything
outside the crate ever asks for — and the answer is that three of the seven are
consumed, one is consumed as a single bit, and three are consumed by nothing.

The two instances take one half each. The stored side turns out to have a second
home: `ring_bench::Workload` holds a `RingConfig` and re-declares two of its
fields alongside it, and its constructor leaves one of the copies thirty-two
apart from the record it was handed. The derived side has two members, one of
which documents itself as the only one.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md) | Five Fields, and the One Another Crate Keeps a Copy Of | The per-field read census and `Workload`'s shadow copies |
| [002](002_the_two_derived_readings.md) | The Two Derived Readings | `is_multi_producer` and `is_tick_safe`, their callers, and the miscount in one doc |

## Three Consumed, One Downcast, Three Untouched

Outside this crate, in non-doctest production code, the census reads: `capacity`
four times, `overflow` five, `is_multi_producer` once. Nothing else appears at
all — not `wait`, not `producers` as a number, not `batch`, not `is_tick_safe`.

The `is_multi_producer` entry is the interesting one, because it is how
`producers` reaches a consumer. The field is a `usize`; the single call that
touches it evaluates `self.producers > 1`. The record stores a magnitude and the
workspace asks it a yes-or-no question.

## The Fields That Went Somewhere Else

`ring_bench::Workload` is the only struct in the family that holds a `RingConfig`
in a field, and it declares `producers` and `batch` next to it as plain `usize`s
of its own. Both of its setters write through to the stored record; its
constructor does not, so a fresh `Workload` reports a batch of `32` from one
public getter and `1` from another.

That explains the shape of the census rather than sitting beside it. `batch` has
no reader on the record because the one crate that would read it keeps its own
copy, and the copy is what all seven of its call sites reach.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- seven named members of the record: five stored, two derived --'
command grep -n '^  [a-z_]* : \|pub const fn is_' ring_config/src/lib.rs
echo '  -- how many non-doctest reads each gets outside this crate --'
command grep -rnE '(config\(\)|config|cfg)\.(capacity|wait|overflow|producers|batch|is_multi_producer|is_tick_safe)\(\)' --include=*.rs */src | command grep -v '^ring_config/' | command grep -v '///\|//!' | command grep -oE '(capacity|wait|overflow|producers|batch|is_multi_producer|is_tick_safe)\(\)' | sort | uniq -c
echo '  -- the copies another crate keeps of two of them --'
command grep -m1 -A3 -F '  config : RingConfig,' ring_bench/src/lib.rs
# both readings by declaration, not by a sentence quoted out of one of them:
# RC27 is about how these two describe themselves, so anchoring on the wording
# would freeze this block the moment either doc comment is corrected
echo '  -- the two derived readings, each with the opening line of its own doc --'
awk '
  /^  \/\/\//                    { if ( buf == "" ) { sub( /^  \/\/\/ ?/, "" ); buf = $0 } next }
  /^  #\[/                       { next }
  /^  pub const fn is_[a-z_]*\(/ { sub( /^  /, "" ); printf "    %s\n      %s\n", $0, buf; buf = ""; next }
                                 { buf = "" }
' ring_config/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC25 | `ring_config` | n/a — observation | Of seven named members, non-doctest production code outside the crate reads `capacity` four times and `overflow` five, reaches `producers` only through `is_multi_producer`'s `self.producers > 1`, and never touches `wait`, `batch` or `is_tick_safe` at all — so the record stores a magnitude that is consumed as one bit and two fields that are consumed as nothing, and `ring_factory`'s own suite reached the same three-field result independently with a separate, unrelated reason for each |
| RC26 | `ring_bench` | **latent hazard** | `Workload` holds a `RingConfig` and re-declares `producers` and `batch` beside it; both setters write through to the stored record but `Workload::new` does not, so `Workload::new( cfg )` reports `batch() == 32` and `config().batch() == 1` from two public getters, and `.with_producers( 4 )` reconciles the producer count while leaving the batch divergence in place — harmless only because no backend reads `RingConfig::batch`, against a `with_batch` doc that states the stake exactly: "A batch size that only one arm observed would measure batching against nothing" |
| RC27 | `ring_config` | **wrong doc** | `is_multi_producer`'s documentation opens "The one derived reading in the record" at `:211` while `is_tick_safe` is declared twenty-seven lines below at `:238` in the same `impl` block and computed the same way — the miscount sits on the reading everything uses and omits exactly the one nothing uses, and no sentence anywhere in the crate describes the two as a pair |
| RC28 | `ring_config` | n/a — unadopted | `is_tick_safe` is called from three lines, all in this crate's own `config_test.rs`, with no production caller and no other crate's test anywhere in thirty-three crates — and the feature it cites, `183_try_only_operations_on_the_tick_path.md`, is `Status: planned` and rules out this shape of guard in its own definition: the restriction "is enforced by what is exposed, not by a rule in a document", which is precisely what a predicate a caller must remember to ask is |
