# workaround

External constraints `ring_registry` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

Two entries, and only one of them is a workaround in that sense. The `String`
clone on the refusal path is forced by `std` — `OccupiedEntry` offers no way to
take its key back that does not delete the entry — costs 22 ns per refusal, and
would be retired by an upstream method that does not exist yet. The
`result_large_err` suppression absorbs nothing external: it is downstream of a
decision this crate made and intends to keep, so what would retire it is
reversing that decision, which is to say nothing will.

Neither is written down where a reader of the source would meet it. The crate's
only `.clone()` carries no comment in a file that spends twenty lines refusing to
allocate on the same path, and no line names a circumstance under which the
suppression could go. Both entries below supply what the source does not.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_key_bought_back_from_the_map.md) | A Key Bought Back From the Map | The forced clone, what `std` offers instead, and what it costs per refusal |
| [002](002_a_lint_turned_off_with_no_condition_for_turning_it_back_on.md) | A Lint Turned Off With No Condition for Turning It Back On | The narrow scope that holds, the third lever, and a suppression with no exit |

## One Constraint Absorbed, Priced

`register` materializes an owned `String`, moves it into `entry`, and the
occupied arm needs it back for the error. `VacantEntry::into_key` exists;
`OccupiedEntry` has no `into_key` at all, and the one method that yields the key
by value is `remove_entry`, which deletes the already-registered ring on the way
out — the exact data loss `pitfall/001` exists to prevent. So `.clone()` at
`:158` is the only non-destructive exit from the arm, and it is the crate's only
one.

Measured, it costs 22.3–22.7 ns, which is 43–44% of the arm and almost all of the
gap `algorithm/001` attributed to hashing. The alternative that has no clone —
`contains_key` then `insert` — is faster and puts a live `insert` call back in the
source, which is the one thing `pitfall/001` argues hardest against. Trading 22 ns
on a setup-time call for that is the right trade and nobody has written it down.

## One Suppression That Is Not Debt

The `allow` is on `register` alone and the census finds zero crate-level ones, so
a different oversized `Result` appearing later still fails the build — the only
claim in the crate's cost argument that survives measurement unqualified. What it
has no exit from is the reason it exists: the variant is 448 bytes because it
carries a ring, and it carries a ring because a settled decision says it should.

The twenty-line argument that refuses to shrink or box the payload calls those
two remedies *both* of them. The lint also carries a configurable threshold,
which is named zero times in the source and readme and would silence the lint
crate-wide for every function forever — worse than what was chosen, for the same
reason the narrow `allow` was chosen, and so worth recording as declined rather
than left looking unconsidered.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order.
echo '  -- the one std constraint the crate absorbs, in one line --'
command grep -rn -i 'clone' --include=*.rs ring_registry/src |
  sed 's|ring_registry/||' | sed 's/^/    /'
echo '  -- and the one lint it suppresses, on one function, with a reason --'
command grep -n '#\[ allow' ring_registry/src/lib.rs | cut -c1-96 | sed 's/^/    /'
printf '    crate-level allow attributes: %s\n' \
  "$( command grep -c '^#!\[ allow' ring_registry/src/lib.rs || true )"
echo '  -- the enumeration that says both, and the third lever it omits --'
command grep -n 'suggested remedies\|Shrink the payload\|Box it\.' \
  ring_registry/src/lib.rs | cut -c1-92 | sed 's/^/    /'
printf '    large-error-threshold named in src or readme: %s   clippy.toml in the crate: %s\n' \
  "$( command grep -rc 'large-error-threshold' ring_registry/src/ ring_registry/readme.md 2>/dev/null |
       awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( ls ring_registry/clippy.toml 2>/dev/null | wc -l )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG49 | `ring_registry` | n/a — doc gap | `register` takes `impl Into< String >`, materializes an owned `String` at `:152` and moves it into `self.rings.entry( name )` at `:154`, so the occupied arm needs the name back for `RegistryError::NameTaken` and buys it with `occupied.key().clone()` — and the recovery is not a choice, since `VacantEntry::into_key()` exists and compiles while `OccupiedEntry` has **no `into_key` at all** (`error[E0599]`, the compiler suggesting `into` as the nearest name), and the one method on `OccupiedEntry` that yields the key by value is `remove_entry()`, which compiles and returns `( String, V )` at the price of removing the entry — here, deleting the already-registered ring and every unread record in it, precisely the data loss `pitfall/001` exists to prevent, arrived at from the other direction — leaving the clone as the only non-destructive way out of the arm; the source says none of this, `src/lib.rs:158` being the crate's only `.clone()` with the only other "clone" line being `RegistryError`'s derive, so in a crate that spends twenty lines refusing to allocate on the failure path — "*Box it.* That allocates on the failure path" — an unexplained allocation on that same path reads as an oversight rather than a forced move, and a reader with the `Into< String >` signature in front of them has every reason to think the name could simply have been kept; one line above `:158` converts a clone that looks avoidable into one a reader can verify is not |
| RG50 | `ring_registry` | **measured cost** | The recovery costs **22.3–22.7 ns per refusal, 43–44% of the arm's total**, reproducing across two runs to within 0.4 ns, which settles an attribution RG2 asserted without isolating: the `Entry` form is 24–26 ns slower than `contains_key`-then-`insert` on the occupied path and **22 of those nanoseconds are this clone**, so almost none of the gap is the hash count the crate's own argument names — with the recovery removed the `Entry` arm runs at 51.7 ns against the alternative's 48; both halves of what this definition asks for are available, the cost being 22.3–22.7 ns and one `String` allocation on the refusal path only, and the exits being an `into_key` on `OccupiedEntry` or any keyed-lookup entry API that borrows the key rather than consuming it, or else accepting the `contains_key`-then-`insert` form, which compiles today with no clone at the price of a live `insert` call site; the second escape is real and its price is exactly the argument `pitfall/001` makes second and best — `insert` in the source is a call a later edit can reach, and `Entry` removes it — so trading 22 ns on a setup-time call for that guarantee is the right trade, and the finding is that the trade appears nowhere in the crate: the twenty lines of cost argument are all about the `Result`'s width, and the one line that actually costs measurable time is unremarked |
| RG51 | `ring_registry` | **misleading doc** | `src/lib.rs:127` writes "Both of the lint's suggested remedies are refused" and enumerates them — shrink the payload, box it — each refused with a reason that holds, in the strongest piece of argument in the crate, and the word doing the damage is *both*: the lint carries a configurable threshold, `large-error-threshold` in `clippy.toml`, which is a real key — a bogus one is rejected outright with exit 101 and a list of the valid fields, while this one is accepted silently — and measured against the same two functions the boundary is exact, **at 448 the warning still fires and at 449 it stops**, so setting it to 449 or above silences the lint everywhere in the crate, for every function, forever, and would have made the twenty lines of argument unnecessary; that option is worse than the one taken, for exactly the reason the decisions file gives for keeping the `allow` narrow, but "both" says the enumeration is complete and it is not, so a reader who later discovers the threshold has no way to tell whether it was considered and rejected or simply not known — two words repair it, "both of the lint's suggested remedies" becoming "both of the remedies the lint suggests", which is true since the lint's own help text offers shrinking and boxing and not the config key, and one clause naming the threshold as a third lever deliberately not pulled turns an omission into a decision |
| RG52 | `ring_registry` | n/a — observation | `decisions/readme.md:42` claims a guarantee — the `allow` is written on `register` alone, never crate-wide, so a different oversized `Result` appearing later still fails the build — and tested against the real types it **holds exactly**: the suppressed function produces nothing, the identical function one declaration later fires, reports the same 448 bytes, and would fail a build run under `-D warnings`, with the census confirming the other half at **zero crate-level `allow` attributes** in the file, making this the only claim in the crate's cost argument that survives measurement unqualified and worth saying so, the narrow scope having been chosen for a stated reason that is true; what the entry still lacks is the second half of what this definition promises, since **zero lines in the source** name a circumstance under which the suppression could go, and unlike `workaround/001` — whose clone a future `OccupiedEntry::into_key` would retire — no upstream change deletes this one: the lint fires because the `Err` variant is 448 bytes, the variant is 448 bytes because it carries a ring, and it carries a ring because Closed 2 decided it should, so the suppression is downstream of a permanent decision and is therefore permanent; the honest entry says as much — it absorbs no external constraint awaiting an upstream fix, and what would retire it is reversing Closed 2 — which is what stops a later reader from treating it as debt to be paid off, and is the difference between a suppression that was decided and one that was merely tolerated |
