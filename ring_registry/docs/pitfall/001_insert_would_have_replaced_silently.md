# Pitfall: `insert` Would Have Replaced Silently

### Scope

- **Purpose**: Record the one-line mistake that satisfies every count-based assertion and destroys data, and why the acceptance criterion asks for a drop counter.
- **Responsibility**: The failure, why it is invisible to the obvious tests, and the three other places the same shape appears.
- **In Scope**: `HashMap::insert` versus `Entry`; the tests that would and would not catch the difference.
- **Out of Scope**: The invariant it would violate (→ [`invariant/001`](../invariant/001_one_name_one_ring.md)).

### Abstract

**A registry built on `HashMap::insert` passes every test you would naturally
write, and silently destroys a ring on every name collision.** `insert` returns
the displaced value; ignoring that return drops it, and dropping a `Split< T >`
drops the `Ring< T >`, and dropping a `Ring< T >` drops every record still unread
in it — measured, in the probe that preceded this crate's implementation:

```
after 5 pushes,            drops = 0
after dropping the Split,  drops = 5
```

So a caller who registers `"events"` twice by mistake loses the first ring's
entire unread contents, and nothing anywhere returns an error.

### Why the obvious tests do not catch it

| Test you would write | Passes under `insert`? |
|---|---|
| A registered ring is retrievable by its name | **Yes** — the second one is |
| Retrievable by no other name | **Yes** |
| `len()` is 1 after two registrations under one name | **Yes** — the map still holds one entry |
| `contains( "events" )` | **Yes** |
| The retrieved ring has the expected capacity | **Yes**, if both rings were built alike |

Every count, every lookup, every boolean is correct. The map genuinely holds one
ring under one name. What is wrong is *which* ring, and that the other one is
gone — neither of which any of the above can express.

**Only two things distinguish the implementations**: an error returned on the
second registration, and a drop counter. The acceptance criterion asks for the
drop counter explicitly, and this is why.

### The fix, and why it is not just "check first"

```rust
match self.rings.entry( name )
{
  Entry::Occupied( occupied ) => Err( ( NameTaken { .. }, ring ) ),
  Entry::Vacant( vacant )     => { vacant.insert( ring ); Ok( () ) },
}
```

The `Entry` API is the fix because it removes the `insert` call site entirely —
the alternative,
`if self.contains( &name ) { return Err( .. ) } self.rings.insert( name, ring );`,
is correct here, since `&mut self` excludes concurrent modification, but it
leaves that call in the code where a later edit can reach it and reintroduce
the silent replace. It also performs two hashes for one decision on the path
that inserts, where `Entry` is measurably cheaper; on the path that refuses,
`Entry` costs the occupied-arm clone instead and is the slower of the two.

### The same shape, three more places

| # | Where | The silent-destruction version |
|---|---|---|
| F1 | `register` | `insert`, discarding the displaced ring |
| F2 | `remove` | Returning `bool` instead of `Option< Split< T > >` — makes recovery *possible*; a bare `registry.remove( name );` still discards it, with no warning |
| F3 | A `clear()` or `Drop` that forgot the values | Would satisfy `is_empty()` afterwards and account for nothing |
| F4 | A caller `.expect()`ing on `register` | Panics on collision — and the panic unwinds, dropping the rejected ring; the message reports `produced`/`consumed` via `Ring`'s `Debug`, so the unread count is on the panic line |
| F5 | `get_mut` | Assignment through the returned borrow (`*registry.get_mut( name ).unwrap() = fresh;`) drops the displaced ring in place — unlike F1–F3, this ships today and needs no change to this crate to occur |

**F2 is the one that was nearly written.** `remove( &str ) -> bool` reads
naturally, matches `contains`, and is what a set-like API would offer. It also
means the caller cannot get their ring back, and the records go with it. The
signature returning `Option< Split< T > >` is what makes
`a_removed_ring_carries_its_records_to_its_new_owner` expressible at all.

**F4 is not a defect and is listed to be complete** — a caller who panics has
made their own choice about the collision. The panic is `expect`'s `Debug`
formatting of the rejected `Split< T >`, so `produced`/`consumed` on the ring
inside it already states the unread count, without a debugger or a breakpoint.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| P1 | `register` uses `insert` | Nothing. All counts correct, all lookups correct, one ring's contents gone |
| P2 | `remove` returns `bool` | Nothing at the call site; the records are dropped by the registry |
| P3 | A drop counter test is written but the records are never actually pushed | Passes trivially — 0 expected, 0 observed |

**P3 is a defect in the test rather than in the crate**, and it is worth naming
because it is the way this pitfall's own guard gets neutered. A drop-counter test
whose rings are empty asserts nothing. The test here pushes 4, 7 and 2 records
into three rings and asserts 13, so the count would be wrong if any single ring
were skipped — a single total of 13 across three distinct sizes is much harder to
reach accidentally than 3 rings × 1 record.

---

## What the Panic Says, and the Shape That Had No Row

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order. Patterns against this
# file are anchored at column 0 and carry no line numbers: the findings below
# discuss these same rows, and would otherwise match themselves.
i=ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md
echo '  -- the five shapes this pitfall catalogues --'
command grep -h '^| F[0-9] |' "$i" | cut -c1-98 | sed 's/^/    /'
echo '  -- and what F4 predicts a caller who panics will see --'
command grep -h -A 3 '^\*\*F4 is not a defect' "$i" | cut -c1-96 | sed 's/^/    /'
echo '  -- which of the catalogued mistakes is present in the shipped source --'
printf '    self.rings.insert (F1):      %s\n' "$( command grep -c 'self\.rings\.insert' ring_registry/src/lib.rs || true )"
printf '    a clear() method  (F3):      %s\n' "$( command grep -c 'pub fn clear' ring_registry/src/lib.rs || true )"
printf '    remove as shipped (F2):      %s\n' "$( command grep -h 'pub fn remove' ring_registry/src/lib.rs )"
echo '  -- every method that can replace a registered ring, and whether an F row names it --'
for m in register get_mut remove
do
  printf '    %-9s named in an F row: %s\n' "$m" \
    "$( command grep -h '^| F[0-9] |' "$i" | command grep -c "$m" || true )"
done
```

Live output:

```
  -- the five shapes this pitfall catalogues --
    | F1 | `register` | `insert`, discarding the displaced ring |
    | F2 | `remove` | Returning `bool` instead of `Option< Split< T > >` — makes recovery *possible*
    | F3 | A `clear()` or `Drop` that forgot the values | Would satisfy `is_empty()` afterwards and ac
    | F4 | A caller `.expect()`ing on `register` | Panics on collision — and the panic unwinds, drop
    | F5 | `get_mut` | Assignment through the returned borrow (`*registry.get_mut( name ).unwrap() = f
  -- and what F4 predicts a caller who panics will see --
    **F4 is not a defect and is listed to be complete** — a caller who panics has
    made their own choice about the collision. The panic is `expect`'s `Debug`
    formatting of the rejected `Split< T >`, so `produced`/`consumed` on the ring
    inside it already states the unread count, without a debugger or a breakpoint.
  -- which of the catalogued mistakes is present in the shipped source --
    self.rings.insert (F1):      0
    a clear() method  (F3):      0
    remove as shipped (F2):        pub fn remove( &mut self, name : &str ) -> Option< Split< T > >
  -- every method that can replace a registered ring, and whether an F row names it --
    register  named in an F row: 2
    get_mut   named in an F row: 1
    remove    named in an F row: 1
```

F4's prediction, run. Sixteen records are pushed into the ring the caller is
about to have rejected, and the `.expect()` panic is caught and read:

```rust
let payload = std::panic::catch_unwind
(
  std::panic::AssertUnwindSafe( move | | { registry.register( "events", rejected ).expect( "a free name" ); } )
)
.unwrap_err();

let message = payload.downcast_ref::< String >().cloned().unwrap_or_default();
```

Its output, identical on two runs:

```
    `.expect( "a free name" )` on a taken name panics with:
      message length: 169 characters
      first 200:
        a free name: (NameTaken { name: "events" }, Split { ring: Ring { storage: Spsc(Ring { capacity: 16, produced: Seq(16), consumed: Seq(0), .. }), overflow: DropNewest } })
      mentions the rejected ring's type: true
      mentions the name it refused:      true
      mentions a record value:           false
```

---

### RG41 — The Panic F4 Warns Will Say Nothing Reports the Unread Count Exactly

F4 exists to warn a debugger: the message "will say 'a free name' or whatever the
caller wrote", so someone reading it "should know the records were in the
*rejected* ring". The premise is that the panic carries the caller's string and
nothing else.

It carries 169 characters, and they are the ones that matter.
`expect` on a `Result` formats the error with `Debug`, and the error here is a
tuple whose second element is the whole rejected `Split< T >` — so the message
reads `a free name: (NameTaken { name: "events" }, Split { ring: Ring { storage:
Spsc(Ring { capacity: 16, produced: Seq(16), consumed: Seq(0), .. }), overflow:
DropNewest } })`. It names the refused name. It names the rejected ring's
capacity. And `produced: Seq(16), consumed: Seq(0)` **is** the count of records
about to be destroyed — sixteen written, none taken — stated on the panic line,
without a debugger, without a breakpoint. Only the record *values* are elided,
behind `Ring`'s own `..`.

**Finding.** Recorded as a **misleading doc** that undersells the crate. F4 asks
the reader to supply from documentation a fact the runtime already prints better
than prose could, and a debugger who believes F4 will not look. The repair is to
replace the warning with the message: quote it, and point at
`produced`/`consumed` as where the loss is counted. Two consequences fall out
worth a line each. The message is a fixed shape regardless of `T`, since `Ring`'s
`Debug` truncates its storage — so `.expect()` is cheap to leave in, not a
data-dumping hazard. And it is `.expect()` that drags in the `T : Debug` bound
[RG5](../api/001_the_registry_surface.md) prices; this is the call site that pays
for it, and the diagnostic above is what it buys.

**Disposition:** applied — the F4 row (`:70`) and its paragraph (`:79-82`) now
say the panic reports `produced`/`consumed` via `Ring`'s `Debug`, replacing the
claim that the message "will not mention the records" / will say only "a free
name". Now prints: `already states the unread count`

---

### RG42 — Four Counterfactuals, and the One Instance of This Shape That Ships

F1 through F4 catalogue the pitfall's shape wherever it could appear. Three of
the four describe code that was never written: `self.rings.insert` appears **0**
times, there is no `clear()` method, and `remove` ships as
`-> Option< Split< T > >` rather than the `bool` F2 warns against. F4 is a
caller's choice, not the crate's. So the catalogue is four hypotheticals, and its
value is entirely as a record of decisions taken correctly.

There is a fifth, and it is not hypothetical. Three methods take `&mut self` and
can reach a registered ring; F rows name `register` twice and `remove` once, and
name `get_mut` **zero** times. Yet
[RG21](../invariant/001_one_name_one_ring.md) measures
`*registry.get_mut( "events" ).unwrap() = fresh;` destroying six unread records
in one statement, with `len` still 1 and `contains` still true. Read that against
this document's own table two sections up — retrievable by its name, retrievable
by no other, `len()` correct, `contains` correct, capacity as expected — and
every row still says **Yes**. It is F1's failure exactly, reached without `insert`
and without a mistake in this crate's source, by a caller doing something the
surface permits.

**Finding.** Recorded as a **latent hazard** in the catalogue rather than in the
code: an inventory of a failure shape that omits the only live instance teaches
the reader the shape is retired. The repair is an F5 row — assignment through the
borrow `get_mut` returns, the displaced ring dropped in place — and one sentence
saying that unlike F1–F3 it needs no change to this crate to occur. F5 also
answers the "same shape, three more places" heading honestly: the shape is in
four more places, and one of them is reachable now.

**Disposition:** applied — added the F5 row (`get_mut`, line 71) to the table
above, naming assignment-through-`get_mut` as the one instance of this shape
that ships unchanged. The section heading was corrected to name the row that
was missing rather than the row this document now has, and the shared
Live-output block was regenerated: the catalogue count reads "five shapes" and
`get_mut named in an F row` reads 1, up from the 0 this finding reported.
Inserting the row shifted every later line in the file by one; two downstream
citations were repaired in the same pass so neither goes stale — RG41's own
disposition two sections up (`:78-81` → `:79-82`) and `pitfall/002`'s Scope and
Sources citations into this file (`:72`/`:75`/`:88` → `:73`/`:76`/`:89`). Full
crate suite re-verified passing (`cargo test --all-features -p ring_registry`,
2026-09-04). Now prints:
`get_mut   named in an F row: 1`

---

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_name_one_ring.md](../invariant/001_one_name_one_ring.md) | V1 — this failure as a violation; E2 as the mechanism that prevents it |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_registry_surface.md](../api/001_the_registry_surface.md) | A3 and A4 — the guarantees this pitfall is the reason for |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_ring_from_registration_to_drop.md](../lifecycle/001_a_ring_from_registration_to_drop.md) | F2 — where ownership must go on `remove` |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The `Entry` match |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | The drop behaviour the probe measured — a ring drops its unread records |

### Tests

| File | Relationship |
|------|--------------|
| `tests/registry_test.rs` | P1 — `a_refused_registration_does_not_drop_the_ring_already_there`, the only test that distinguishes the two implementations; P2 — `a_removed_ring_carries_its_records_to_its_new_owner`; P3's avoidance — `dropping_the_registry_drops_every_record_still_in_every_ring` uses three distinct non-zero counts |
