# api

Three types, eleven inherent methods, three trait impls, and exactly one private
item. The surface is small enough to read in one sitting and disciplined in the
ways that are easy to check mechanically — every returning method carries
`#[ must_use ]`, which only four other crates in the family manage — and less
disciplined in the way that is not, which is where the reasons live.

The pattern across both instances is placement. The crate knows why it does what
it does: the shared receiver has a justification, the copied return has a
justification, the frozen flag has a justification. Each of those is written
where the author was working rather than where a reader would look for it, so the
type-level facts arrive one method at a time and the method that most needs the
argument is the one that did not get it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_three_types_nine_attributes_and_one_private_item.md) | Three Types, Nine Attributes and One Private Item | The declaration census, the private guard, and family-wide `#[ must_use ]` coverage |
| [002](002_shared_reference_everywhere_and_what_it_forces.md) | Shared Reference Everywhere and What It Forces | Zero `&mut self`, one explained mutator, and what the other one permits |

## One Keyword Holds the Line

`entries_guard` is the only private item in the crate, and it is the reason no
caller can hold this lock. `entries()` copies rather than borrowing and says so,
but that choice is a consequence, not the mechanism: making the guard method
public would leave every other line correct and the property gone. The crate
documents the consequence and not the cause.

The `#[ must_use ]` count is the mirror image — a place where the crate is
careful and the family is not. Nine attributes over nine returning methods, each
verified by reading. Of the twenty-seven `ring_*` crates that declare a
single-line returning inherent method, five have the counts agree; the largest
gap elsewhere is eight. Two crates report more attributes than methods, which is
the census admitting it cannot see a signature broken across lines.

## The Door Beside the One That Was Locked

`Trace` freezes its enabled flag after construction on an explicit argument: a
trace switched on mid-run leaves a silent hole at the front that reads like a run
where nothing happened, and that is the one misreading a diagnostic must not
invite. The argument is correct and the flag is genuinely immutable.

`clear` takes `&self`, so any holder of a shared reference can produce the same
hole at any point. Measured: a producer recording 200,000 operations beside one
clearing holder finishes with 115 entries in one run and none at all in the
other, with `record` returning `()` and `is_enabled` still reporting true. The
case is currently unreachable because nothing in the family calls this crate,
which makes it a hazard to document rather than a bug to fix — but it is the same
hazard, one door along.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- public declarations, then every fn not declared pub --'
echo '  -- (three of the four are trait methods, reachable through their traits) --'
command grep -c '^  pub \|^pub ' ring_trace/src/lib.rs
command grep -n '^  fn ' ring_trace/src/lib.rs
echo '  -- receivers --'
printf '    &self %s   &mut self %s\n' \
  "$( command grep -c 'fn [a-z_]*( *&self' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'fn [a-z_]*( *&mut self' ring_trace/src/lib.rs || true )"
echo '  -- must_use here, against the family --'
m=$( command grep -c '#\[ must_use \]' ring_trace/src/lib.rs || true )
r=$( command grep -c '^  pub \(const \)\?fn [a-z_]*(.*) ->' ring_trace/src/lib.rs || true )
echo "    ring_trace: $m attributes, $r returning methods"
agree=0
for c in ring_*/; do
  cm=$( command grep -c '#\[ must_use \]' "$c"src/lib.rs 2>/dev/null || true )
  cr=$( command grep -c '^  pub \(const \)\?fn [a-z_]*(.*) ->' "$c"src/lib.rs 2>/dev/null || true )
  if [ "$cr" != 0 ] && [ "$cm" = "$cr" ]; then agree=$(( agree + 1 )); fi
done
echo "    crates in the family where the two agree: $agree"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR5 | `ring_trace` | n/a — doc gap | Everything the crate declares is public but one line — `fn entries_guard` at `:277`, which returns the `MutexGuard` — and that keyword is the entire mechanism preventing a caller from holding the producers' lock across arbitrary code; `entries()` documents the *consequence*, saying it returns a copy because handing out a guard would allow exactly that, while `entries_guard`'s own doc claims a different and lesser property, that going through one accessor stops call sites disagreeing about poisoning, so the containment invariant survives by convention with no line stating it, and making the method `pub` would leave every other line correct, compiling and passing all nineteen tests with the property silently gone |
| TR6 | `ring_trace` | n/a — observation | Nine `#[ must_use ]` attributes sit over exactly the nine returning inherent methods — `name`, `end`, `enabled`, `disabled`, `is_enabled`, `len`, `is_empty`, `entries`, `count_of`, each verified by reading — while `record` and `clear` return `()` and cannot carry one; against the family this is unusual, since of the twenty-seven `ring_*` crates declaring a single-line returning inherent method only five have the two counts agree and gaps elsewhere reach eight, and the census is a lower bound rather than a measurement because `ring_cursor` and `ring_overflow` report more attributes than methods where a signature is broken across lines — the attribute matters here because a discarded `count_of` is a wasted lock acquisition and, at 100,000 entries, a 200 µs stall on the producers' path for nothing |
| TR7 | `ring_trace` | **latent hazard** | `Trace` freezes `enabled` after construction on an explicit and correct argument — a trace switched on mid-run "would produce a log with a silent hole at the front, which reads exactly like a run where nothing happened early — the one misreading a diagnostic tool must not invite" — and `clear` produces that same hole through the adjacent door, taking `&self` so every holder of a shared reference can call it: measured, a producer recording 200,000 operations beside one clearing holder ends with 115 entries in one run and zero in the other, `record` returns `()` so the producer cannot tell, and the trace then reports `is_empty` true with `is_enabled` true, which reads as a correctly-configured trace that saw no traffic; unreachable today only because nothing in the family calls this crate, and closed either by a paragraph on `clear` making the reach explicit or by giving it a `&mut self` receiver, which costs nothing at the three call sites that exist |
| TR8 | `ring_trace` | n/a — doc gap | The receiver census is eleven `&self`, one `self`, zero `&mut self`, which is a decision about the whole type — `Trace` is uniformly interior-mutable and any shared holder can do anything the type does — stated once, in the middle of `record`'s documentation, phrased as a fact about that one method; `ring_stats`, the crate that sentence points at, states it above the struct instead, so a reader arriving at its destructive method already holds the rule and can ask what it permits, where a reader arriving at `clear` here has met the rule only if they read `record` first — moving one sentence to `Trace`'s own doc while keeping `record`'s note about producers would make TR7's gap visible at the point a reader can act on it, and the suite shows the same placement problem in miniature, calling `clear` at three lines all single-threaded while both `thread::scope` tests only record |
