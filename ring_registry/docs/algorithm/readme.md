# algorithm

Two pieces of executable code decide everything this crate does. One is a
two-armed `match` on a `HashMap` entry, and it is the entire write path. The
other is four one-line forwards to the map, and it is the entire read surface.
Between them they are fewer than twenty statements, and the crate has argued for
their shape at greater length than it has measured it.

What links the four findings here is that every cost claim the crate makes about
these statements was reasoned from the source and none was taken from a
measurement. The refusal path is said to allocate once and allocates twice.
`Entry` is said to be chosen because the alternative hashes twice, and on the
refusal path the alternative is a third faster because its second hash never
happens. The read path warns that order "varies between runs" when it varies
between registries in one run, and dismisses the ordered map as "a real cost"
when at registry size it is half the price.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_branches_and_what_the_refusal_costs.md) | Two Branches and What the Refusal Costs | The `Entry` match, its two allocations, and the form that is faster on the arm that matters |
| [002](002_four_reads_and_the_order_they_do_not_promise.md) | Four Reads and the Order They Do Not Promise | `contains`, `get_mut`, `len` and `names`, and the ordering guarantee the crate understates |

## The Arm the Crate Was Built Around Is the Arm It Is Slower On

`register`'s whole shape — the tuple return, the private error type, the
drop-counter test, this crate's most-argued paragraph — exists for the occupied
arm. On that arm the `Entry` form costs 72–74 ns and `contains_key`-then-`insert`
costs 48 ns, reproduced to 0.05 ns across two runs, because a `contains_key` that
hits returns after one hash and never reaches the second. The two hashes the
argument names happen only on the arm where nothing goes wrong.

`Entry` is still right, and the crate says why in the next sentence: it removes
the `insert` call site, so no later edit can reintroduce the silent replace. That
reason costs nothing and needs no benchmark. Leading with the hash count instead
tells a reader the choice was made for speed on the one path where it is
measurably slower.

## A Read Path Whose Warning Is True and Too Weak

`names()` warns that `HashMap` iteration order "varies between runs", which
places the hazard at the process boundary and makes a within-run comparison look
sound. `RandomState` reseeds per map, not per process: five registries built by
one function in one process, given the same five names in the same order, produce
five distinct orderings, twice over. The advice attached to the warning — sort,
or collect into a set — already covers the real case; only the stated trigger is
wrong, and a caller who believes the trigger cannot fire will not follow advice
they think is about someone else's program.

The same comment prices the alternative as "a real cost paid for a property
nothing needs". At four names `BTreeMap::contains_key` is 15.4 ns against
`HashMap`'s 29.8 — 48% faster, reproduced to 0.03 ns. The crossover is between 4
and 16 names; at 64 the sentence is correct. A registry of rings is a setup-time
structure, and the property it dismisses is the same one it spends the previous
two sentences warning callers about.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two arms, and the allocation before them --'
command grep -m1 -A14 -F '    let name = name.into();' ring_registry/src/lib.rs
echo '  -- the four reads --'
command grep -n '^  pub fn contains\|^  pub fn len\|^  pub fn is_empty\|^  pub fn names' ring_registry/src/lib.rs
echo '  -- and the sentence that prices the ordered map --'
command grep -m1 -A6 -F '  /// Every live name, in no particular order.' ring_registry/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG1 | `ring_registry` | **wrong doc** | `type/001` prices the refusal path as "one allocation against an error already being constructed", and the path allocates the name twice: `register` takes `impl Into< String >` and calls `name.into()`, which allocates and is then consumed by `HashMap::entry`, and the occupied arm calls `occupied.key().clone()` for a second copy of the same six bytes — measured at **2 allocations totalling 12 bytes** for a refused `"events"`; the conclusion survives the correction, since 12 bytes on a setup-time refusal is genuinely not worth avoiding, but the figure a reader carries away does not, and the honest sentence is that the name is allocated once to hash it and once to report it |
| RG2 | `ring_registry` | **measured cost** | `pitfall/001` gives two reasons for `Entry` over `contains_key`-then-`insert` and leads with the weaker one — the alternative "performs two hashes for one decision" — which on the occupied path is false, because a `contains_key` that hits returns after one hash and never reaches `insert`: measured, `Entry` costs 72–74 ns there against the alternative's 48 ns, a gap of about a third of the `Entry` form's total, reproduced to within 0.05 ns across two runs, while the vacant path's lead for `Entry` is real in direction but inside the noise in magnitude (58.74 ns then 48.85 ns, against occupied medians that moved 0.1%); the decision is right and the *second* stated reason is why — `Entry` removes the `insert` call site so no later edit can reintroduce the silent replace — so the repair is to lead with it and drop or qualify the hash-count claim |
| RG3 | `ring_registry` | **misleading doc** | `names()`'s doc says `HashMap` iteration order "varies between runs", which is true and weaker than the truth: `RandomState` reseeds per `HashMap` rather than per process, so five registries built by the same function in one process from the same five names in the same order produce five distinct orderings, twice over — a caller comparing two registries fails on the first attempt, in this run, with no version change involved; the attached advice (sort, or collect into a set) already covers the real case, so the cost is only to the reader who believes the stated trigger cannot fire in their program, and one word repairs it — "varies between registries", or "between one `HashMap` and the next, including within a single run" |
| RG4 | `ring_registry` | **measured cost** | The same doc dismisses an ordered map as "a real cost paid for a property nothing needs", and at the population a setup-time registry actually runs at the ordered map is cheaper: at 4 names `BTreeMap::contains_key` costs 15.4 ns against `HashMap`'s 29.8 ns, **48% faster** and reproduced to 0.03 ns across two runs, with the crossover between 4 and 16 names and the sentence becoming correct by 64, where `BTreeMap` is 2× slower; the dismissed property is also the one RG3 shows the crate warning callers about two sentences earlier, so the repairs point in two directions — correct the sentence to rest on family convention or unknown population rather than cost, or take the measurement seriously, since `BTreeMap` is a drop-in for all eight methods, deletes RG3's hazard, and is the only one of the two that exists in `alloc` |
