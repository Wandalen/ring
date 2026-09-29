# api

The public surface of `ring_stats`: sixteen methods, two types, and two derives —
fourteen and one when these findings were written. It is a small enough API to read in
a minute and the interesting facts are all about what it declines — exclusive access,
comparison, copying — and about the one thing it declined for a long time and no longer
does: any means of obtaining two counters at once, which `RingStats::snapshot` now
provides and which every finding below was written against the absence of.

The two instances here take the surface from both sides. The first is the census —
fifteen methods on `&self`, one associated function, `must_use` on all ten
value-returning methods, and a crate-level `deny( missing_docs )` — and asks what a
uniformly shared reference forecloses, particularly on `reset`, the one method whose
contract is about the whole set. The second is the absence: no `Clone`, no `Copy`,
no `PartialEq`, no snapshot type, so every question about two counters is one the
caller assembles from separate loads — a cost the crate pays three times in its own
source, and one `ring_atomic` answered differently a crate away.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_fourteen_methods_and_no_exclusive_borrow.md) | Fourteen Methods, and No Way to Borrow the Type Exclusively | Every signature, the `must_use` marks — eight when written, ten now — and what `&self` on `reset` costs |
| [002](002_seven_readers_and_no_way_to_read_the_set.md) | Seven Readers, and No Way to Read the Set | The traits declined, the snapshot type that does not exist, and who assembles one anyway |

## A Surface Designed to Be Shared

`&mut self` does not appear in the crate. That is the point of the type: one
`RingStats` behind an `Arc`, written by every producer, read by whatever is
monitoring, with no lock anywhere in the family. Fourteen of the fifteen `&self`
methods want exactly that.

The fifteenth is `reset`, and it inherits the shape without wanting it. An
operation whose contract is "reset every counter to zero" is an operation about the
set, and a shared reference is precisely what makes it impossible to perform on the
set — hence seven independent stores and a window in which some are cleared and
others are not. The signature is still the right one; what is missing is the sentence
saying that the contract describes an outcome rather than an instant.

## The Attribute That Is Everywhere and the Traits That Are Nowhere

`#[ must_use ]` sits on all ten methods that return a value and on none of the six
that do not — eight against eight when this was written, and the two methods added
since were each marked on arrival — complete, correct, and spent entirely on returns
whose loss costs one relaxed load. It is the same habit visible across the family, where the attribute
tracks *does this return something* rather than *does dropping this lose something*,
and where the one return in `ring_atomic` that genuinely records an irreversible act
goes unmarked.

Against that, `RingStats` derives only `Debug` and `Default`. There is no `Clone`, no
`Copy`, and no `PartialEq` — so the seven readers were seven separate doors and nothing
let a caller through two of them at the same moment. The crate's three composing methods
are what that absence looked like from inside, and all three are findings. `StatsCounts`
is the snapshot type that was missing: a plain `Copy` value of nine `u64` fields, still
assembled from seven separate loads, but assembled once inside the crate rather than
once per caller.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the surface, by what it takes and gives --'
printf '    take &self %-3s take &mut self %-3s must_use %s\n' \
  "$( command grep -c 'fn [a-z_]*( &self' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '&mut self' ring_stats/src/lib.rs || true )" \
  "$( command grep -c 'must_use' ring_stats/src/lib.rs || true )"
echo '  -- the seven readers --'
command grep -n '  pub fn [a-z_]*( &self[^)]*) -> u64' ring_stats/src/lib.rs
echo '  -- and the traits declined --'
sed -n '/^#!\[ deny( missing_docs ) ]$/p;/^#\[ derive( Debug, Default ) ]$/p' ring_stats/src/lib.rs
printf '    Clone %s   Copy %s   PartialEq %s\n' \
  "$( command grep -c 'Clone' ring_stats/src/lib.rs || true )" \
  "$( command grep -c 'Copy' ring_stats/src/lib.rs || true )" \
  "$( command grep -c 'PartialEq' ring_stats/src/lib.rs || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST5 | `ring_stats` | n/a — doc gap | Every method but `new` takes `&self` and none takes `&mut self` — thirteen of fourteen when this was written, fifteen of sixteen now — which is correct for a set shared by every producer and consumer, but is also the reason `reset`, the one method whose contract covers all seven counters, has no choice but seven independent stores, and neither half of that trade is written on the method |
| ST6 | `ring_stats` | n/a — observation | Every value-returning method carries `#[ must_use ]` and all six `()`-returning ones correctly do not — eight against eight when this was written, ten against ten since, with both later additions marked on arrival — giving total coverage spent entirely on returns that can be re-obtained by calling again, the family's habit of applying the attribute by position rather than by consequence, at its most complete |
| ST7 | `ring_stats` | n/a — doc gap | `RingStats` is the live counter set, deriving `Debug` and `Default` and nothing else, so for a time no operation on the API returned more than one counter and even `{:?}` is seven reads at seven moments — the crate's three multi-counter methods were the three places it needed one anyway, and all three are findings; `RingStats::snapshot` is now a fourth, returning a `StatsCounts` whose own parts agree with each other though its loads are still seven |
| ST8 | `ring_stats` | n/a — duplication | `ring_atomic` ships `OpCounts` — a `Copy`, comparable four-field value returned by `counts()` — so the family already had the snapshot pattern, one crate over, with the tear confined to a single documentable function; `ring_stats` declined it without recording the choice, distributing the same tear across every caller, until `RingStats::snapshot` and `StatsCounts` brought the shape into this crate too |
