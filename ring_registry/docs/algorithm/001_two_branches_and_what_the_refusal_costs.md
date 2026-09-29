# Algorithm: Two Branches and What the Refusal Costs

### Scope

**Purpose:** Measure the crate's entire write path — one `Into< String >`, one
hash, one two-armed match — against the two cost claims the crate's own
documents make about it.

**Responsibility:** `register`'s body, its allocation count on each arm, its
per-call time, and the `Entry`-versus-`contains_key` comparison the crate
settled by argument.

**In Scope:** `ring_registry/src/lib.rs:165-179`;
`ring_registry/docs/type/001_registry_error.md:49-51`;
`ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md:54-59`.

**Out of Scope:** The reads are
[`algorithm/002`](002_four_reads_and_the_order_they_do_not_promise.md). The width
of the value the error carries is
[`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md).
What `Entry` prevents is
[`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md).

---

## Fourteen Lines, and the Two Sentences That Price Them

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the whole write path --'
command grep -m1 -A14 -F '    let name = name.into();' ring_registry/src/lib.rs
echo '  -- what the crate says the failure path allocates --'
command grep -A2 'The clone happens only' ring_registry/docs/type/001_registry_error.md
echo '  -- and what the crate says the alternative costs --'
command grep 'two hashes for one decision\|removes the .insert. call site' ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md
```

Live output:

```
  -- the whole write path --
    let name = name.into();

    match self.rings.entry( name )
    {
      Entry::Occupied( occupied ) =>
      {
        let name = occupied.key().clone();
        Err( ( RegistryError::NameTaken { name }, ring ) )
      },
      Entry::Vacant( vacant ) =>
      {
        vacant.insert( ring );
        Ok( () )
      },
    }
  -- what the crate says the failure path allocates --
have no lifetime to borrow from. The clone happens only on the failure path,
alongside the one allocation `into()` already made — two small allocations,
not one, and neither worth avoiding.
  -- and what the crate says the alternative costs --
The `Entry` API is the fix because it removes the `insert` call site entirely —
the silent replace. It also performs two hashes for one decision on the path
```

## Every Allocation Each Arm Makes

*The allocation counts below, and the Entry-versus-`contains_key` timings
further down, both come from one-off scratch binaries under `-rg_probe/` —
compiled, run once, and swept along with the rest of that gitignored
directory, per this project's convention for temporary files. There is no
binary left for either to run again. `register` and the four reads still
match what each probe exercises, so nothing here is known to have drifted —
but treat both sets of numbers as a historical snapshot, not a reproducible
live result.*

A counting global allocator wrapped around `System`, incremented in `alloc` and
read before and after each call:

```rust
// -rg_probe/src/bin/allocs.rs
struct Counting;
unsafe impl GlobalAlloc for Counting
{
  unsafe fn alloc( &self, layout : Layout ) -> *mut u8
  {
    ALLOCS.fetch_add( 1, Ordering::SeqCst );
    BYTES.fetch_add( layout.size(), Ordering::SeqCst );
    unsafe { std::alloc::System.alloc( layout ) }
  }
  ..
}
```

```
    Registry::new()                      0 allocations, 0 bytes
    register into a free name            2 allocations, 1810 bytes
    register into a taken name (refused) 2 allocations, 12 bytes
    contains + get_mut + len + names(1) 0 allocations, 0 bytes
```

## The Two Spellings of the Same Decision, Timed

Both arms of the comparison run against a `HashMap< String, [ u64; 48 ] >` — a
384-byte value, the width of the `Split< T >` the registry actually stores —
with `black_box` on the map and the name, `#[ inline( never ) ]` on each
harness, and a median of nine paired repetitions.

```rust
// -rg_probe/src/bin/entry_vs_check.rs
match black_box( &mut *m ).entry( black_box( name ).to_string() )
{
  Entry::Occupied( o ) => { black_box( o.key().clone() ); refused += 1; },
  Entry::Vacant( v ) => { v.insert( [ 0; 48 ] ); },
}
// against
let key = black_box( name ).to_string();
if black_box( &*m ).contains_key( &key ) { black_box( key ); refused += 1; }
else { m.insert( key, [ 0; 48 ] ); }
```

```
=== run 1 ===
    Entry match, occupied         median 74.01 ns/call  min 72.88  max 86.85
    contains_key then insert      median 48.11 ns/call  min 47.68  max 50.41
    the second hash costs         -25.89 ns/call, -35% of the Entry form
=== run 2 ===
    Entry match, occupied         median 72.10 ns/call  min 71.99  max 79.08
    contains_key then insert      median 48.06 ns/call  min 47.91  max 49.74
    the second hash costs         -24.04 ns/call, -33% of the Entry form
```

The vacant arm of the same comparison, where the two hashes the argument names
are both actually performed:

```
=== run 1 ===
    Entry match, vacant           median 877.03 ns/call  min 859.24  max 907.06
    contains_key then insert      median 935.77 ns/call  min 912.58  max 976.49
    the Entry form saves          58.74 ns/call on the path that inserts
=== run 2 ===
    Entry match, vacant           median 853.93 ns/call  min 772.91  max 887.85
    contains_key then insert      median 902.77 ns/call  min 875.24  max 927.42
    the Entry form saves          48.85 ns/call on the path that inserts
```

And `register` itself, on the refusal path, against the two reads the crate
offers, same methodology, two independent runs:

```
=== run 1 ===
    contains( "events" )   median 24.03 ns/call  min 23.92  max 26.53
    get_mut( "events" )    median 28.42 ns/call  min 28.29  max 28.52
    register into a taken name  median 113.28 ns/call  min 112.21  max 115.90
=== run 2 ===
    contains( "events" )   median 24.34 ns/call  min 24.19  max 25.30
    get_mut( "events" )    median 28.64 ns/call  min 28.47  max 28.98
    register into a taken name  median 114.36 ns/call  min 112.51  max 160.51
```

---

### RG1 — The Refusal Path Allocates the Name Twice, Where the Crate Says Once

`register` takes `impl Into< String >` and calls `name.into()`, which allocates.
It hands that `String` to `HashMap::entry`, which consumes it. On the
`Entry::Occupied` arm it then calls `occupied.key().clone()` — allocating a
*second* copy of the same name, from the key the map already holds — and returns
that one in the error.

The measurement is unambiguous: registering `"events"` into a taken name makes
**2 allocations totalling 12 bytes**, which is six bytes of name, twice.

[`type/001`](../type/001_registry_error.md) prices this path in one sentence:
"The clone happens only on the failure path, where one allocation against an
error already being constructed is not worth avoiding." The reasoning is sound
and the count is wrong. There is one *clone*, and there are two *allocations* —
the `into()` on the way in is the one the sentence does not see, because it
happens before the branch that decides which arm runs.

**Finding.** Recorded as a false number in a cost argument, not as a performance
problem: 12 bytes on a setup-time refusal is genuinely not worth avoiding, and
the conclusion survives the correction intact. What does not survive is the
figure a reader would carry away. The honest sentence is that the failure path
allocates the name twice — once to hash it and once to report it — and that both
are small. If the second one is ever worth removing, the shape that removes it is
`contains_key` before `entry`, which trades an allocation for a hash and is the
form [`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md)
rejects for a different reason.

**Disposition:** applied — `docs/type/001_registry_error.md:50-51` now reads
"alongside the one allocation `into()` already made — two small allocations,
not one, and neither worth avoiding", replacing the "one allocation... is not
worth avoiding" undercount the finding names; the `into()` allocation on the
way in is now named alongside the failure-path clone. Now prints: `alongside the one allocation`

---

### RG2 — On the Refusal Path, `Entry` Is the Slower of the Two Forms by a Third

[`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md) gives two
reasons for `Entry` over `contains_key`-then-`insert`, in this order: the
alternative "performs two hashes for one decision", and it "leaves the `insert`
call in the code where a later edit can reach it". The first is stated as the
mechanism. Measured, it is the weaker of the two, and on the path the crate
exists to get right it points the other way.

On the **occupied** path — a name collision, which is what `register`'s whole
signature, error type and drop-counter test are built around — the `Entry` form
costs 72–74 ns and `contains_key`-then-`insert` costs 48 ns. The gap, 24–26 ns,
is about a third of the `Entry` form's total, and it reproduces: two independent
runs put the alternative's median within 0.05 ns of itself. The reason is that a
`contains_key` which *hits* returns after **one** hash and never reaches `insert`
at all — the second hash the argument names does not happen on this path. What
the `Entry` form pays instead is an `OccupiedEntry` and the key clone of RG1.

The two hashes are real on the **vacant** path, and there `Entry` is ahead. But
this harness cannot put a number on that lead honestly: the vacant arm is
dominated by building a fresh key and moving a 384-byte value into a growing
map, its medians moved by 3% between two runs whose occupied medians moved by
0.1%, and the saving it reports moved from 58.74 ns to 48.85 ns across the same
pair. The direction is stable; the magnitude is inside the noise.

**Finding.** Recorded as a correct decision resting on the weaker of its two
arguments. `Entry` is right, and the sentence *after* the one that explains why
is the reason: it removes the `insert` call site entirely, so no later edit can
reintroduce the silent replace. That reason costs nothing to hold and does not
depend on a benchmark. The remedy is to lead with it, and to drop the hash-count
claim or qualify it to the insert path — as written, the order tells a reader
the crate chose `Entry` for speed on the one path where it is measurably slower.

**Disposition:** applied —
`docs/pitfall/001_insert_would_have_replaced_silently.md:54-61` now leads with
call-site removal ("The `Entry` API is the fix because it removes the `insert`
call site entirely") and qualifies the hash-count claim to the path that
inserts, naming the occupied-arm clone as the cost on the path that refuses —
the remedy the finding names. Now prints: `call site entirely`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_four_reads_and_the_order_they_do_not_promise.md) | The four operations this one is not |
| [`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md) | The failure `Entry` actually prevents |
| [`type/001`](../type/001_registry_error.md) | The sentence RG1 corrects, and the `String` it is about |
| [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md) | The 384-byte value both arms move |
| [`pattern/001`](../pattern/001_an_error_that_hands_the_payload_back.md) | The error-with-payload shape the Occupied arm builds |

### Sources

| Fact | Where |
|------|-------|
| The fourteen-line write path | `ring_registry/src/lib.rs:165-179` |
| "The clone happens only on the failure path" | `ring_registry/docs/type/001_registry_error.md:49` |
| "two hashes for one decision" | `ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md:59` |
| 2 allocations, 12 bytes on the refusal path | Probe above |
| 72–74 ns against 48 ns on the occupied path | Probe above, two runs |
| The vacant-path saving moving 58.74 → 48.85 ns | Probe above, two runs |
| 113 ns per refused `register` against 24 ns per `contains` | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `a_second_registration_under_a_live_name_is_refused` | The Occupied arm |
| `a_refused_registration_hands_the_ring_back` | The `Split< T >` the arm returns |
| `removing_a_name_frees_it_for_reuse` | The Vacant arm, twice, under one name |
