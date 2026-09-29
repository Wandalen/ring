# workaround

Two things the crate does instead of the direct thing, and both are about a
dependency. One is an addition written out by hand where the crate it already
depends on offers the same expression under a name. The other is a dependency two
of the crate's own documents insist it has, which the manifest does not declare
and the code never imports.

Neither costs anything today. What makes them worth recording together is that
they point in opposite directions: the crate reaches past a dependency it has,
and claims one it does not.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_an_addition_the_types_crate_already_offers.md) | An Addition the Types Crate Already Offers | `end`'s open-coded `+`, the family's idiom, and the one other site with the same expression |
| [002](002_a_third_dependency_two_documents_still_name.md) | A Third Dependency Two Documents Still Name | `ring_seqno` in the readme and the task file, and the same disagreement across 33 crates |

## Three Raw Additions, Two of Them Identical

`Seq::advanced_by` is `const`, `#[ must_use ]`, and its body is the same
`Self( self.0 + n )` that `TraceEntry::end` writes out. Ten crates call
`advanced_by` or `next`; three sites in the whole family build a `Seq` from raw
field arithmetic, and two of the three are the same expression — `ring_batch`
computing one past the end of a batch claim, `ring_trace` computing one past the
end of the record of one.

The two crates do not depend on each other, neither doc mentions the other, and
neither uses the named method. The duplication is semantic rather than accidental:
a `( Seq, count )` pair with a "one past the end" reading is a recurring quantity
here, spelled by hand in both places it appears.

## A Dependency That Could Not Have Helped

The readme and task 125 both name `ring_seqno`. The manifest declares one path
dependency and the string appears zero times in any source or test file. The claim
was never plausible — `ring_seqno` is five free functions about position comparison
across laps, three of which take a `Capacity` a trace does not have, and the other
two compare live cursors a log of past operations does not hold.

That is why nothing broke, and why nothing would have. Across the family, 14 of
33 readmes disagree with their own manifests; two over-claim and thirteen omit,
so the line is unmaintained in both directions with nothing checking it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the arithmetic written by hand, and the method that names it --'
command grep -rn 'Seq( [a-z_.]*\.0 [-+]' --include=*.rs ring_*/src/ | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sed 's/^/    /'
printf '    crates calling advanced_by or next instead: %s\n' \
  "$( command grep -rl '\.advanced_by(\|\.next()' --include=*.rs ring_*/src/ 2>/dev/null | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||;s|/src/.*||' | sort -u | wc -l )"
echo '  -- the dependency two documents name and three sources deny --'
printf '    readme names: %s   task names: %s   manifest declares: %s   imports: %s\n' \
  "$( command grep -o 'ring_seqno' ring_trace/readme.md | head -1 )" \
  "$( command grep -o 'ring_seqno' ring_trace/task/unverified/125_implement_ring_trace.md | head -1 )" \
  "$( command grep -c 'ring_seqno' ring_trace/Cargo.toml || true )" \
  "$( command grep -rc 'ring_seqno' --include=*.rs ring_trace/src ring_trace/tests 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
echo '  -- and the same comparison across the family --'
add=0; both=0
for c in ring_*/; do
  rd=$( command grep -m 1 'Depends on' "$c"readme.md 2>/dev/null | command grep -o 'ring_[a-z_]*' | sort -u )
  md=$( command grep -o 'path = "\.\./ring_[a-z_]*"' "$c"Cargo.toml 2>/dev/null | command grep -o 'ring_[a-z_]*' | sort -u )
  e=$( comm -23 <( echo "$rd" ) <( echo "$md" ) | tr -d '\n' )
  m=$( comm -13 <( echo "$rd" ) <( echo "$md" ) | tr -d '\n' )
  if [ -n "$e" ]; then add=$(( add + 1 )); fi
  if [ -n "$e" ] || [ -n "$m" ]; then both=$(( both + 1 )); fi
done
printf '    readmes over-claiming: %s   disagreeing at all: %s of 33\n' "$add" "$both"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR49 | `ring_trace` | n/a — duplication | `Seq::advanced_by` exists, is `const`, is `#[ must_use ]`, and its body is the same `Self( self.0 + n )` that `TraceEntry::end` writes out by hand, in a crate that already depends on `ring_types` and imports `Seq` from it, so calling the method costs nothing at all — and across all 33 crates exactly three sites build a `Seq` from raw field arithmetic while ten crates call `advanced_by` or `next`, making the named method unambiguously the family's idiom and the open-coded form the deviation; the probe confirms the two spellings agree on value and on failure, `Seq( 8 ).0 + 64` and `Seq( 8 ).advanced_by( 64 )` both giving 72 and both wrapping identically at the top of the range, so nothing is broken by writing it out and the substitution is direct, `self.seq.advanced_by( self.count as u64 )` replacing the body and dropping the field access — what it buys beyond consistency is one place to change, since whatever `ring_types` eventually does about the wrap it documents but does not have will land in `advanced_by`, which a call site inherits and an open-coded `+` does not |
| TR50 | `ring_trace` | n/a — duplication | `ring_batch/src/lib.rs:131` reads `Seq( self.start.0 + self.count as u64 )` and `ring_trace/src/lib.rs:171` reads `Seq( self.seq.0.saturating_add( self.count as u64 ) )` — one field name apart and character for character identical when this was recorded, `count as u64` cast included, until `pitfall/001` TR41's fix gave `ring_trace`'s half saturating overflow, both inside a method named `end` whose doc says "one past the last sequence" covered by a batch, while the third raw-arithmetic site, `ring_mpsc:506`, is a subtraction doing something else; the duplication is semantic rather than coincidental, `ring_batch` computing where a batch claim finishes and `ring_trace` computing where the record of one finishes because `count` is exactly `ring_batch`'s "one operation, not 64" and the trace stores the same two numbers the claim does, yet the two crates depend on each other in neither direction and neither doc mentions the other, and neither uses `advanced_by` although `ring_batch` also depends on `ring_types` and its own documentation already records what the underlying `+` does at the boundary — so a `( Seq, count )` pair with a "one past the end" reading is a recurring quantity in this family spelled by hand in both places it appears, the minimum fix being TR49's one clause in each crate and a shared helper in `ring_types` worth it only if a third caller appears, which recording the pair is what would let anyone notice |
| TR51 | `ring_trace` | **wrong doc** | The readme's first line of substance and task 125's `Depends on` field both name `ring_types` and `ring_seqno`, in the same order, against a manifest declaring one path dependency and a codebase where the string `ring_seqno` appears zero times in every source and test file; the claim was never plausible, `ring_seqno` being five free functions about position comparison across laps of which three take a `Capacity` — a ring's capacity, which a trace does not have and cannot obtain — while the other two, `pending` and `slowest`, compare producer and consumer cursors that a log of past operations does not hold, so there is no function in that crate a `TraceEntry` could be passed to; that is exactly why nothing broke, an undeclared dependency the code wanted having failed to compile immediately where this one stays invisible because no call site was ever going to reach for it, and the two documents agreeing with each other and with nothing else is the signature of both having been written from the same scaffold rather than from the code — the remedy being to delete the second name from both files in one edit, since correcting one alone would leave them disagreeing where today they at least agree, and this is the second confirmed instance of the pattern with `ring_event` recording the first against the same absent crate |
| TR52 | `ring_trace` | n/a — inconsistency | Comparing every crate's readme dependency line against its own manifest, 14 of the 33 disagree: two over-claim, `ring_event` and `ring_trace`, both naming `ring_seqno`, and thirteen under-claim by much larger margins — `ring_mpsc`'s readme names one of eight declared dependencies, `ring_tls` omits five, `ring_publish` four, `ring_debug` four — so the line is unmaintained in both directions with no mechanism by which it could be, nothing generating it, nothing checking it, and as TR51 shows an error in it producing no compile failure, no test failure and no warning, leaving prose beside a manifest that already states the same fact machine-readably where the only way to notice divergence is for someone to read both; that places this crate's error in context, not sloppiness particular to `ring_trace` but the one form of the error that happens to be visible, a name that should not be there reading as a claim while thirteen names that should be there read as nothing — and since the line does carry something the manifest does not, each entry being a link to the dependency's readme and genuinely useful navigation, deleting it is not the answer, generating it from the manifest is, and until something does the honest intermediate is to say on the line that it is a reading aid rather than the dependency list |
