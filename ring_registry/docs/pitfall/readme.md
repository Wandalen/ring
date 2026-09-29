# pitfall

One hazard defines this crate: a registry that maintains one-value-per-key by
*replacing* destroys the previous ring and every unread record in it, silently,
with every count-based and lookup-based assertion still passing. The crate was
built around not doing that, and `pitfall/001` catalogues four places the shape
could appear — F1 through F4 — so a later maintainer meets the reasoning before
the temptation.

Three of the four describe code that was never written, which is what makes the
catalogue a record of decisions taken correctly. The two findings against it are
that the one live instance of the shape has no row, and that the panic F4 warns
will say nothing actually reports the loss precisely. The other two are about
`remove`, whose hazard the catalogue records as averted by a signature that makes
recovery possible without making discard hard.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_insert_would_have_replaced_silently.md) | `insert` Would Have Replaced Silently | The four counterfactuals, the panic's real content, and the fifth route that ships |
| [002](002_the_remove_that_drops_a_ring_without_a_word.md) | The `remove` That Drops a Ring Without a Word | F2 reproduced against the shipped signature, and the doc asymmetry behind it |

## Four Hypotheticals and One Live Instance

`self.rings.insert` appears zero times, there is no `clear()`, and `remove` ships
returning `Option< Split< T > >` rather than the `bool` F2 warns against. F4 is a
caller's choice. So the catalogue is four counterfactuals — and there is a fifth
that is not one.

Three methods take `&mut self` and can reach a registered ring. The F rows name
`register` twice and `remove` once, and name `get_mut` zero times, while
`*registry.get_mut( "events" ).unwrap() = fresh;` destroys six unread records in
one statement with `len` still 1 and `contains` still true. Read against this
document's own table of what a test would still report — retrievable by its name,
retrievable by no other, `len()` correct, `contains` correct, capacity as
expected — every row still says **Yes**.

## A Warning That Undersells the Runtime

F4 asks a debugger to supply from documentation the fact that the destroyed
records were in the *rejected* ring, on the premise that the panic carries the
caller's string and nothing else. It carries 169 characters, because `expect`
formats the error with `Debug` and the error is a tuple whose second element is
the whole rejected `Split< T >`: the name, the capacity, and
`produced: Seq(16), consumed: Seq(0)` — which *is* the count of records about to
be destroyed, on the panic line, with no debugger. Only the record values are
elided, behind `Ring`'s own `..`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
p=ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md
echo '  -- the three write paths --'
command grep -n '^  pub fn register\|^  pub fn get_mut\|^  pub fn remove' \
  ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- the four counterfactual rows, and the write path none of them names --'
command grep -n '^| F[0-9] |' "$p" | cut -c1-92 | sed 's/^/    /'
printf '    F rows naming get_mut: %s\n' "$( command grep -c '^| F[0-9] |.*get_mut' "$p" || true )"
echo '  -- and the calls three of those rows describe, none of which exist --'
printf '    self.rings.insert: %s   pub fn clear: %s\n' \
  "$( command grep -c 'rings.insert' ring_registry/src/lib.rs || true )" \
  "$( command grep -c 'pub fn clear' ring_registry/src/lib.rs || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG41 | `ring_registry` | **misleading doc** | F4 exists to warn a debugger that the panic "will say 'a free name' or whatever the caller wrote", so someone reading it "should know the records were in the *rejected* ring" — and the premise that it carries the caller's string and nothing else is false: `expect` on a `Result` formats the error with `Debug` and the error is a tuple whose second element is the whole rejected `Split< T >`, so the message runs **169 characters** and reads `a free name: (NameTaken { name: "events" }, Split { ring: Ring { storage: Spsc(Ring { capacity: 16, produced: Seq(16), consumed: Seq(0), .. }), overflow: DropNewest } })`, naming the refused name, the rejected ring's capacity, and — in `produced: Seq(16), consumed: Seq(0)` — the exact count of records about to be destroyed, with only the record values elided behind `Ring`'s own `..`; the repair is to replace the warning with the message and point at `produced`/`consumed`, and two consequences follow: the shape is fixed regardless of `T` since `Ring`'s `Debug` truncates its storage, so `.expect()` is cheap to leave in rather than a data-dumping hazard, and it is `.expect()` that drags in the `T : Debug` bound RG5 prices — this is the call site that pays for it and this diagnostic is what it buys |
| RG42 | `ring_registry` | **latent hazard** | F1 through F4 catalogue the pitfall's shape wherever it could appear and three of the four describe code that was never written — `self.rings.insert` appears **0** times, there is no `clear()`, and `remove` ships as `-> Option< Split< T > >` rather than the `bool` F2 warns against — while F4 is a caller's choice, so the catalogue is four hypotheticals whose value is as a record of decisions taken correctly; there is a fifth and it is not hypothetical, since three methods take `&mut self` and can reach a registered ring while the F rows name `register` twice and `remove` once and name `get_mut` **zero** times, yet RG21 measures `*registry.get_mut( "events" ).unwrap() = fresh;` destroying six unread records in one statement with `len` still 1 and `contains` still true — read against this document's own table of what the obvious tests would report, every row still says Yes; it is F1's failure exactly, reached without `insert` and without a mistake in this crate's source, so the repair is an F5 row and one sentence saying that unlike F1–F3 it needs no change here to occur |
| RG43 | `ring_registry` | **misleading doc** | `pitfall/001` names F2 as the near miss — "`remove( &str ) -> bool` reads naturally, matches `contains`, and is what a set-like API would offer. It also means the caller cannot get their ring back, and the records go with it" — and records the remedy as the signature, `Option< Split< T > >` being "what makes `a_removed_ring_carries_its_records_to_its_new_owner` expressible at all"; the signature makes the good path *expressible* and does not make the bad path *unreachable*, since `registry.remove( "events" );` written as a whole statement compiles with no warning under `-D warnings` and measured against six unread records destroys all six, leaving `len 0` and `contains("events") false` — the count-invisible destruction that is this pitfall's whole subject — with P2's own prediction "Nothing at the call site" still exactly correct and only the mechanism moved, from the registry dropping the ring internally to the caller's semicolon dropping it externally; the two routes are not equally dangerous and the difference favours the crate, F1 being a hazard a maintainer could reintroduce silently and `Entry` closing it permanently, but `pitfall/001` presents F2 as handled and F1 as live and on the evidence it is the other way round |
| RG44 | `ring_registry` | n/a — coverage | `register`'s doc opens on the hazard — "**Refuses rather than replaces.** A registry that silently replaced would drop the previous ring — and with it every record still unread in that ring — as a side effect of a name collision" — with **four** occurrences of *drop*, *destroy*, *lose* or *unread* across its thirty-three lines, while `remove`'s five-line doc contains **zero**, both its sentences being about what the method enables: freeing a name, making "a name that is taken" a temporary condition, making `register`'s refusal "recoverable rather than final" — every word true and not one telling a reader that the value being returned is the only remaining owner of a buffer full of records; the test suite mirrors the asymmetry and therefore cannot correct it, since all four `remove` call sites bind the value or assert on it and `a_removed_ring_carries_its_records_to_its_new_owner` proves the records survive *when the value is held*, leaving the discarded shape unexercised — so the suite's guard covers F1's route and not F2's, which is the same guard-quality question `pitfall/001` raises as P3 asked of one route and not the other; two sentences on `remove` and one bare-statement test are the repair |
