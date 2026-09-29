# non_functional_requirement

This crate exists for one number. This crate's own contract says a batch of sixty-four costs
roughly what a single item costs, the module comment quotes that sentence back,
and the entire implementation of it is one `fetch_add` whose second argument
happens to be larger. Measured, the sentence is true and conservative: a batch of
sixty-four costs 11.49 ns against a single item's 5.93 ns on one thread, and at
eight threads the batch is *cheaper* than the single-item claim — 63.59 ns
against 80.92 ns — because the cache line transfer is the cost and the count
rides along free.

The second cost dimension the crate wins outright and declares nowhere. Nothing
in the body allocates, both iterator-returning functions are lazy, and the whole
working set is sixteen stack bytes — held in place by no `no_std`, no lint, and
no assertion.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_what_one_batch_actually_buys.md) | What One Batch Actually Buys | The cost curve at one and eight threads, and the two batch sizes that buy nothing |
| [002](002_sixteen_bytes_and_no_allocation.md) | Sixteen Bytes and No Allocation | The heap the crate never touches, and the family's allocating version of its fold |

## Both Ends of the Curve Are Degenerate and the Caller Checks Neither

At zero items the operation is pure overhead — a `fetch_add(0)` takes the cursor's
cache line exactly as a real claim does, which costs a working producer 2.0× per
claim at a single idle thread. At one item it is exactly break-even: 5.93 against
6.12 on one thread, 80.92 against 82.00 on eight, 1.0× both. By eight items the
ratio is 6.6×, and the crate's reason for existing has started to apply.

`ring_tls::flush_into` — the family's only caller — claims whatever is staged and
looks at neither end. Its doc warns about the empty case in prose and gives no
number; nothing anywhere mentions that a batch of one is a claim of one with an
extra `usize` attached.

## A Property Is Not a Guarantee

The allocation story is the same shape one level down. The crate genuinely does
not touch the heap, and the only thing keeping it that way is that nobody has
written a `Vec` yet — three of thirty-three crates are `no_std` and `ring_batch`
is not among them, its
sole crate attribute is `deny( missing_docs )`, and no test observes an
allocation. A `collect()` added for convenience would compile, pass, and change
the crate's cost class in silence.

One crate down, `ring_index::run` is that `collect()` already written: the same
fold, returning a `Vec`, 9.6× slower at one item and still 1.7× at sixty-four,
with no caller anywhere outside its own tests.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the claim, and the whole of what makes it true --'
command grep -m1 -A1 -F '//! per operation, not per item, so a batch of sixty-four costs roughly what a' ring_batch/src/lib.rs
command grep -m1 -F '  BatchClaim::new( cursor.fetch_add( count as u64, order ), count )' ring_batch/src/lib.rs
echo '  -- what could reach the heap --'
command grep -n 'Vec\|Box::\|String\|to_vec\|collect\|alloc' ring_batch/src/lib.rs | command grep -v '///' || echo '    (none)'
echo '  -- and what holds that in place --'
command grep -n '#!\[' ring_batch/src/lib.rs || echo '    (no crate-level attributes at all)'
printf '    crates with #![no_std] : %s of 33\n' "$( command grep -rl 'no_std' --include=lib.rs ring_*/src/ | wc -l )"
echo '  -- the allocating twin, and its callers --'
command grep -m1 -A3 -F 'pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >' ring_index/src/lib.rs
command grep -rn 'ring_index::run\|index::run' --include=*.rs . | command grep -v '^ring_index/' || echo '    (nobody)'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA34 | `ring_batch` | n/a — observation | This crate's own stated claim holds and understates: a batch of sixty-four costs 11.49 ns against a single item's 5.93 ns on one thread, and at eight threads is cheaper than a single-item claim (63.59 ns against 80.92 ns) |
| BA35 | `ring_tls` | **measured cost** | A batch of one is exactly break-even at both thread counts (1.0×), so a caller that flushes per push pays the whole machinery for none of the amortisation; the crossover is unstated and unchecked |
| BA36 | `ring_batch` | n/a — unenforced | The crate allocates nothing and nothing holds it there — three of 33 crates are `no_std` and this is not one of them, the sole crate attribute is `deny( missing_docs )`, and one `collect()` would compile and pass |
| BA37 | `ring_index` | n/a — duplication | `ring_index::run` is the same fold returning a `Vec`, 9.6× slower at one item and 1.7× at sixty-four, with no caller outside its own tests; neither crate mentions the other's version |
