# lifecycle

A ring enters through `register`, is lent through `get_mut`, leaves through
`remove` or through the registry's own drop, and the stage model that describes
this is the crate's most careful document. Its value is that it enumerates: five
stages, a transition per edge, a cleanup obligation, and a table of the failures
it would notice. What the four findings here have in common is that the
enumeration is short by exactly the transitions that would falsify its own
claims, and that the guarantees it makes about teardown are not the crate's to
make.

`T` carries no bound anywhere, which the crate names as a feature. That admits a
record type whose `Drop` panics, and one panic leaves eight of twelve records
never dropped — the stage the model rules out as unrepresentable. A `&mut Split`
is an assignable place, which gives ownership two exits the transition table does
not list. And the crate writes no `Drop` at all, so completeness comes from
`HashMap` and ordering does not come from anywhere.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_ring_from_registration_to_drop.md) | A Ring From Registration to Drop | The five stages, the panicking `T` the model excludes, and the ownership moves it does not count |
| [002](002_the_teardown_order_nothing_promises.md) | The Teardown Order Nothing Promises | Destruction order across rings, and whose guarantee teardown actually is |

## The Widest Input Space Has the Least-Stated Edge

The Dependencies table names the absence of a bound as a feature — "`T : `
nothing at all" — and N3 then excludes a partially-dropped registry from the
model because `HashMap`'s own `Drop` runs to completion. Both cannot be true at
once. A record type whose `Drop` panics is squarely inside the accepted input
space, and drop glue behaves differently either side of the map: it finishes the
affected ring's own records, then the panic escapes and `HashMap`'s element loop
stops where it stands, leaking every ring it had not reached along with the
table. Measured, one panic leaves eight of twelve records never entered.

Three claims rest on N3. L6's transitivity holds for one hop and fails after it;
the cleanup obligation's "exactly once" becomes *at most* once; and Q3, listed as
a hypothetical implementation failure, is a reachable behaviour of the current
one.

## Teardown Is Inherited, Including the Part `std` Declines to Promise

`command grep` finds zero `impl Drop` in `ring_registry/src`. The
destructor that runs is `HashMap`'s, then `Split`'s, then `Ring`'s — the right
design, since a registry writing its own `Drop` would add a step to a chain that
already works. The consequence the file does not draw is that the teardown
properties the crate can claim are exactly the ones `HashMap` guarantees:
completeness is on that list and ordering is explicitly not.

Measured, five registries built by one function from one fixed name list produce
five different teardown orders, and none of ten samples matched registration
order. In this family that is not academic — the two crates that write a `Drop`
use it to publish, `Reserved::drop` performing "the one `Release` store the whole
protocol turns on" — so for a record type whose destructor flushes or closes,
"every record is dropped" and "the records are dropped in a defined order" are
different guarantees, and the crate makes only the first.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every place a bound on T could have been written --'
command grep -n '^pub struct Registry\|^impl< T' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- who writes a Drop in this family --'
command grep -rn '^impl.*Drop for' --include=lib.rs ring_*/src/ | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sed 's/^/    /'
printf '    impl Drop in ring_registry/src: %s\n' \
  "$( command grep -rc 'impl.*Drop for' ring_registry/src/ | awk -F: '{ s += $2 } END { print s + 0 }' )"
echo '  -- and the assertion that cannot see a reordering --'
command grep -m1 -A5 -F '    {' ring_registry/tests/registry_test.rs | sed 's/^/    /'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG29 | `ring_registry` | **latent hazard** | N3 excludes a partially-dropped registry from the model because `HashMap`'s own `Drop` runs to completion, which is true of a `HashMap` whose values drop without panicking, and the crate accepts values that do not: `T` carries no bound on the struct or either `impl` and the Dependencies table names that absence as a feature, so a record type whose `Drop` panics is squarely inside the accepted input space and one panicking drop leaves **eight of twelve records never entered at all** — the boundary being exactly the map, since drop glue inside the affected ring finishes its own four, then the panic escapes `Ring`'s drop and `HashMap`'s element loop stops where it stands, leaking the two rings it had not reached along with the table's own allocation; three of the document's claims turn on N3 — L6's transitivity holds for the one hop under the panic and fails at every hop after, the cleanup obligation's "dropped exactly once" becomes *at most* once, and Q3 is listed as a hypothetical failure of the implementation when it is a reachable behaviour of the current one — so the repairs are textual: qualify N3, L6 and the obligation with "unless `T::drop` unwinds", or declare unwinding `T` out of scope, which is a smaller claim than "no bound at all" and belongs in the Dependencies table too |
| RG30 | `ring_registry` | **misleading doc** | The transition table gives K3 exactly one exit, L4 "The borrow ends \| Lent back", and the paragraph beneath is explicit about the count — L1, L2 and L5 are "the three that move ownership, and each is asserted" — while a `&mut Split< T >` is an assignable place: `*registry.get_mut( "events" ).unwrap() = fresh;` sends the borrowed ring from K3 straight to K5, dropped with records destroyed and `len` unchanged, without passing through K4 which the model says is the only stage a registered ring can reach on its way out, and `core::mem::replace` through the same borrow sends it to K4 instead, where `remove` is supposed to be the sole road — five ownership moves, not three, and the two with no row are the two the model would rule out if it named them; the stage table has a matching slip, K2's records being "Preserved; unreachable except through `get_mut`" while L5 two rows below hands the whole ring to the caller; the repair is two rows and one word — L8 (K3 → K5, assignment through the borrow) and L9 (K3 → K4, `core::mem::replace`), with K2 widened to "through `get_mut` or `remove`" — and L8 carries a Q-row consequence for free, being Q1's damage arriving through a door Q1 does not watch |
| RG31 | `ring_registry` | **latent hazard** | `names`'s doc is the only place the crate says what it knows about `HashMap` ordering, and teardown inherits the same nondeterminism while never touching `names()`: dropping a `HashMap` walks its buckets, so the rings and every unread record in them are destroyed in bucket order, measured as **five different teardown orders from five registries built by one function from one fixed name list, with none of ten samples matching registration order** — a hazard a caller reaches without writing an iteration or calling any method at all, firing on the closing brace; whether it matters is entirely a question about `T`, and in this family `Drop` is not idle, the two crates that write one using it to *publish* — `Reserved::drop` performs "the one `Release` store the whole protocol turns on" — so for a record type whose destructor flushes, closes, decrements or logs, "every record is dropped" and "the records are dropped in a defined order" are different guarantees and the crate makes only the first; the cheap repair is one sentence on `Registry`'s own type doc rather than on `names`, and the expensive one is RG4's `BTreeMap`, which would delete this hazard and the `names()` one together — worth noting because neither alone would justify the change and together they are the second and third reason |
| RG32 | `ring_registry` | n/a — doc gap | `lifecycle/001` is confident about L6 — "The registry drops each `Split< T >`, each `Split` drops its `Ring< T >`, and each `Ring` drops the records still unread in it" — and the cleanup table promises "Each `Split` once — no leak, no double drop", both true and neither this crate's code, since `command grep` finds **zero `impl Drop` in `ring_registry/src`** and the destructor that runs is `HashMap`'s, then `Split`'s, then `Ring`'s; that is the right design and it has one consequence the file does not draw — the teardown properties the crate can claim are exactly the ones `HashMap` guarantees, completeness among them and ordering explicitly not — so RG31's gap is not an oversight in documenting a chosen behaviour but the shape of a behaviour inherited whole, including the part `std` declines to promise; the test mirrors it precisely, asserting `DROPS == 0` before and `13` after for rings of 4, 7 and 2, and a total is the one observation that cannot detect a reordering since every permutation sums to thirteen — two lines in the Dependencies table naming which teardown properties come with the map locate the only real lever, which is the map type rather than a `Drop` this crate could write, because a hand-written `Drop` would still be iterating the same unordered map |
