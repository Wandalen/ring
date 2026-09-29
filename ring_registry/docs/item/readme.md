# item

Two declaration-level facts define this crate against its family. It wrote one of
the only two `reason` strings on any `allow` in thirty-three crates, and it put
`#[ must_use ]` on four of its eight methods. Both are deliberate, both are
better than the family average, and both stop one step short of where they would
have prevented something.

The suppression is explained twice — twenty lines of doc comment and a
twelve-word `reason` — and checked zero times, because `#[ allow ]` is silent
forever while `#[ expect ]` is the same attribute that reports when its lint
stops firing, and the family uses `#[ expect ]` nowhere. The four attributes sit
on the methods where ignoring a return wastes twenty nanoseconds, and not on the
one where ignoring a return destroys a ring.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_lints_one_allow_and_the_reason_beside_it.md) | Two Lints, One `allow`, and the Reason Beside It | The seven family suppressions, the two that explain themselves, and the form that would expire |
| [002](002_eight_declarations_and_four_must_use.md) | Eight Declarations and Four `must_use` | Which returns are guarded, which are guarded upstream, and the one that is not |

## Explanation Spent Where It Is Cheapest

Thirty-three crates hold seven `allow` attributes and two carry a `reason`. This
crate wrote one, on `clippy::result_large_err`; `ring_testkit` wrote the other,
on `clippy::too_many_lines`. Both are ergonomics lints — a wide `Result` and a
long function, neither able to produce a wrong answer.

The five bare ones run the other way. `clippy::mut_from_ref` is suppressed in
`ring_mpsc` and again in `ring_spsc`: the lint that fires when a function hands
out a `&mut` derived from a `&`, which is the closest thing clippy has to a
soundness warning, firing in lock-free ring crates on exactly the code where it
would matter. Whatever argument justified those two was made once, by someone,
and is recorded nowhere.

## A Type-System Default Meeting a Design Preference

`Result` carries `#[ must_use ]` upstream and `Option` does not, so a returning
method is guarded by default only if its author picked the error-shaped return.
Five methods across the family take `&mut self` and hand an owned value out
through `Option` — `ring_core::try_recv`, `ring_handle::try_recv`,
`ring_registry::remove`, and `ring_slot`'s `set` and `take` — and none is marked,
in a family that writes the attribute 251 times.

Every one of the five picked `Option` deliberately; `ring_handle:189` writes the
reason directly above its declaration. The choice is right and it silently
forfeits the guard, which is why the same hole will open again the next time
someone correctly prefers `Option` to `Result`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every allow in the thirty-three crates, and which carry a reason --'
command grep -rn '#\[ allow' --include=*.rs ring_*/src/ ring_*/tests/ 2>/dev/null |
  sed 's|||' | cut -c1-96 | sed 's/^/    /'
echo '  -- the four attributes this crate wrote --'
command grep -n '#\[ must_use \]' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- every pub fn in the family handing ownership out through Option --'
command grep -rn 'pub fn [a-z_]*( *&mut self.*) *-> *Option< [A-Z]' --include=lib.rs ring_*/src/ |
  sed 's|||' | cut -c1-88 | sed 's/^/    /'
echo '  -- and the two attribute forms across the family --'
printf '    must_use attributes: %s   expect attributes: %s\n' \
  "$( command grep -rc '#\[ must_use' --include=lib.rs ring_*/src/ | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -rc '#\[ expect' --include=*.rs ring_*/ | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG25 | `ring_registry` | n/a — inconsistency | Thirty-three crates hold **seven `allow` attributes and exactly two carry a `reason`** — this crate's twelve words on `clippy::result_large_err` and `ring_testkit`'s on `clippy::too_many_lines`, both ergonomics lints incapable of producing a wrong answer — while the five unexplained ones run the other way: `clippy::mut_from_ref` is suppressed bare in `ring_mpsc` and again in `ring_spsc`, the lint that fires when a function hands out a `&mut` derived from a `&`, which is the closest thing clippy has to a soundness warning, firing in lock-free ring crates on exactly the code where it would matter, the remaining three being genuinely trivial test-file suppressions; the family's explanations are therefore spent where the suppression is harmless and withheld where it is not, not by policy since there is no policy but as the accumulated result of each author deciding alone — the cheap repair is to require a `reason` on every `allow`, which the compiler already supports and which would have forced the two `mut_from_ref` sites to say what makes them sound |
| RG26 | `ring_registry` | n/a — unadopted | `register` carries its justification twice — twenty lines of doc comment at `:123-142` setting out the lint, both remedies clippy suggests, why each is refused and what the choice costs, then eight lines later the attribute repeating the conclusion in twelve words — and neither is checked, while the attribute form is the one that could be: `#[ allow ]` is silent forever, so if the payload were ever shrunk the suppression, the reason and the twenty-line argument would all remain in place describing a lint that no longer fires, whereas `#[ expect ]` is the same attribute with the opposite default and the probe shows it emitting `warning: this lint expectation is unfulfilled` on by default, naming the line; the family uses `#[ expect ]` **zero** times across every source and test file, so this is an available mechanism nobody has picked up rather than a local omission, and swapping the one word links the argument to the condition it was made under — worth more here than most places because the argument is unusually long and specific, and therefore unusually expensive to leave standing after it stops being true |
| RG27 | `ring_registry` | **latent hazard** | The four `#[ must_use ]` attributes sit on `new`, `contains`, `len` and `is_empty`, where ignoring the return costs a caller nothing — an empty struct with no allocation, or a `bool`/`usize` read out of the map in twenty-odd nanoseconds — and they are correct attributes at the cheap end of the range, while `register` and `names` are covered by `Result`'s and `Iterator`'s own upstream attributes, which is why the probe shows five warnings from four local ones; that leaves `get_mut` and `remove` silent and the two are not alike, since dropping `get_mut`'s return discards a borrow and nothing happens, while dropping `remove`'s discards a `Split< T >` **by value** — the registry has already given the ring away, so the ring, its buffer and every unread record in it are destroyed at the end of the statement, and `registry.remove( "events" );` compiles clean; the attribute was applied by asking "is this a query?" rather than "what happens if this value is dropped?", and the two questions disagree on exactly one method |
| RG28 | `ring_registry` | n/a — unenforced | The same shape appears five times across the thirty-three crates — `ring_core::try_recv`, `ring_handle::try_recv`, `ring_registry::remove`, and `ring_slot`'s `set` and `take` — all taking `&mut self`, all returning `Option< T >` where the `T` is owned and has just left the receiver, and the census checks the line above each declaration and finds the attribute on **none** of them, in a family that writes it 251 times; the cause is not indifference but that `Option` does not carry `#[ must_use ]` itself while `Result` does, so a returning method is guarded by default only if its author picked the error-shaped return, and every one of these five picked `Option` deliberately with `ring_handle:189` writing the reason directly above its declaration — "modelling it as one makes every caller unwrap a non-failure" — a right choice that silently forfeits the guard; severity varies, a dropped `try_recv` losing one record from a ring that has more against `remove` losing a whole ring and everything in it, so the uniform repair is five lines and the minimum one is `remove` per RG27 — and the census is worth keeping either way because it explains why the same hole will open again the next time someone correctly prefers `Option` to `Result` |
