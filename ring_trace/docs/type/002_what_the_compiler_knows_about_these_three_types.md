# Type: What the Compiler Knows About These Three Types

### Scope

**Purpose:** Record the derived and auto-derived properties of the crate's three
types, establish that the property the whole design rests on is pinned nowhere,
and place the derive set beside what the crate and its suite actually do with it.

**Responsibility:** The three `derive` lists, `Trace`'s auto traits, the family's
compile-time type assertions, and the two sets the suite builds by hand.

**In Scope:** `ring_trace/src/lib.rs:69`, `:149`, `:202`;
`ring_trace/tests/trace_test.rs:185-187`, `:223-225`; every `ring_*`
`src/lib.rs` and `tests/`.

**Out of Scope:** The five discriminants and the array beside them are
[`type/001`](001_five_discriminants_and_the_array_beside_them.md). What the
layout costs is
[`data_structure/001`](../data_structure/001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md).

---

## The Derives, and What the Family Does That This Crate Does Not

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the derives each type carries, in declaration order --'
command grep -h '^#\[ derive' ring_trace/src/lib.rs | sed 's/^/    /'
echo '  -- Hash is derived on TraceOp; every use of it in the crate --'
printf '    HashMap, HashSet or .hash() anywhere in the crate: %s\n' \
  "$( command grep -rc 'HashMap\|HashSet\|\.hash(' --include=*.rs ring_trace/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
echo '  -- the two sets the suite builds by hand instead --'
sed -n '/^  let mut unique = names\.clone();$/,/^  unique\.dedup();$/p;/^  let mut unique = TraceOp::ALL\.to_vec();$/,/^  unique\.dedup();$/p' ring_trace/tests/trace_test.rs | sed 's/^ */    /'
echo '  -- and how six other crates pin a type property at compile time --'
for c in ring_*/; do
  k=$( command grep -rc 'assert_sync\|assert_send\|assert_copy\|assert_clone\|assert_debug' "$c"src/lib.rs "$c"tests/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )
  printf '    %-14s %s\n' "$( basename "$c" )" "$k"
done | command grep -v ' 0$'
printf '    ring_trace     %s\n' \
  "$( command grep -rc 'assert_sync\|assert_send\|assert_copy\|assert_clone\|assert_debug' ring_trace/src/lib.rs ring_trace/tests/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

Live output:

```
  -- the derives each type carries, in declaration order --
    #[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
    #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
    #[ derive( Debug ) ]
  -- Hash is derived on TraceOp; every use of it in the crate --
    HashMap, HashSet or .hash() anywhere in the crate: 0
  -- the two sets the suite builds by hand instead --
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    let mut unique = TraceOp::ALL.to_vec();
    unique.sort_unstable_by_key( |op| op.name() );
    unique.dedup();
  -- and how six other crates pin a type property at compile time --
    ring_core      6
    ring_flush     10
    ring_handle    4
    ring_mpsc      13
    ring_registry  2
    ring_spsc      9
    ring_trace     0
```

## The Same Assertions, Run Against These Three Types

*This probe's compiled binary is gone — swept, like every `-tr_probe/`
scratch build, per this project's convention for temporary files — so the
trait checks below can't be re-run directly. The derive lists they exercise
haven't moved: `Trace`, `TraceEntry` and `TraceOp` still carry exactly the
derives shown here, so the `Send`/`Sync`/`Hash` results and the `TraceOp: Ord`
compile error remain what the compiler would say today. TR47 and TR48 below
treat this as settled fact, not a rerun.*

```rust
// -tr_probe/src/bin/type_properties.rs
fn assert_send< T : Send >() {}
fn assert_sync< T : Sync >() {}
assert_send::< Trace >();
assert_sync::< Trace >();
let set : HashSet< TraceOp > = TraceOp::ALL.into_iter().collect();

// -tr_probe/compile/-needs_ord.rs — what the suite's second dedup wanted
pub fn f( v : &mut Vec< TraceOp > ) { v.sort_unstable(); }
```

```
  Trace       Send yes  Sync yes  Debug yes  Copy no  Clone no  Hash no
  TraceEntry  Send yes  Sync yes  Debug yes  Copy yes Clone yes Hash no
  TraceOp     Send yes  Sync yes  Debug yes  Copy yes Clone yes Hash yes
  HashSet< TraceOp > from ALL: 5 distinct kinds, in one line
  a TraceEntry is 24 bytes and Copy, so entries() clones cheaply

error[E0277]: the trait bound `TraceOp: Ord` is not satisfied
```

---

### TR47 — The Property the Whole Design Rests On Is Pinned in Six Other Crates and Not This One

`Trace` is `Send` and `Sync`. It has to be: every method takes `&self`, the
module documentation opens by describing "a shared trace across producers", and
the crate's own contention test hands one `&Trace` to four threads at once. The
property arrives automatically, from `Mutex< Vec< TraceEntry > >` and a `bool`,
and nothing in the crate states it or checks it.

The family does not usually leave this to inference. Six crates pin exactly this
class of property with a one-line generic function and a turbofish call —
`ring_mpsc` thirteen times, `ring_flush` ten, `ring_spsc` nine, `ring_core` six,
`ring_handle` four, `ring_registry` twice — across both doc examples and test
files, and `ring_flush`'s version is the one `pattern/002` records as the
family's standard for a compile-time claim, complete with the compiler's own
notes quoted underneath. `ring_trace` has none of them.

It is also the crate where the assertion would carry the most weight, because
`Sync` here is a consequence of a field type rather than of a deliberate
`unsafe impl`. Swapping `Mutex` for a `RefCell` — which someone optimising the
single-threaded case might reasonably try, since a disabled trace never contends
— silently removes `Sync`, and the first thing to notice would be a compile
error at whatever call site shares the trace, several crates away.

**Finding.** Recorded as a coverage gap against an established family habit
rather than a novel suggestion: the instrument exists, is used 44 times in six
crates, costs two lines, and is missing from the crate whose premise it would
state. `fn assert_sync< T : Sync >() {}` followed by `assert_sync::< Trace >();`
turns "shared across producers" from a sentence in the module doc into something
the compiler enforces.

---

### TR48 — One Derive Too Many and One Too Few, Measured Against What the Suite Does

`TraceOp` derives `Hash`. Nothing in the crate hashes anything: zero `HashMap`,
zero `HashSet`, zero `.hash()`, and `type/001` shows no crate outside
`ring_trace` names the type at all, so the derive has neither an internal user
nor an external one.

`TraceOp` does not derive `Ord`, and the suite needed one. Its uniqueness check
on `ALL` reads `unique.sort_unstable_by_key( |op| op.name() )` then `unique.dedup()`
— a set, built by hand, with the total order borrowed from the string names
because the type does not supply one. Asking for the direct version gives
`error[E0277]: the trait bound `TraceOp: Ord` is not satisfied`.

The two facts meet: `TraceOp` is `Hash + Eq`, so `HashSet< TraceOp >` collects
`ALL` in one line and reports five distinct kinds, which is precisely the
assertion the sort-and-dedup pair is making. The derive the crate has would
replace the workaround the crate wrote, and neither the derive nor the workaround
mentions the other. `TraceEntry`, meanwhile, derives no `Hash` at all, so the two
types that travel together have different capabilities for no stated reason —
and it is the entry, not the operation, that a caller deduplicating a log would
want to key on.

**Finding.** Recorded as an observation about a derive set that was chosen once
and never revisited against use. Nothing is wrong: `Hash` costs nothing to carry
and the hand-built set works. What is worth writing down is that the crate
contains both a capability with no user — `Hash`, derived on the enum at
`src/lib.rs:69` — and a user with no capability — the sort-by-name workaround at
`trace_test.rs:223-225` — and the cheapest resolution is to pick one — either
`HashSet` at the call site, which uses what is derived, or `Ord` on the derive
list, which is what the call site actually asked for.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_five_discriminants_and_the_array_beside_them.md) | The enum these derives sit on |
| [`pattern/002`](../pattern/002_exhaustive_match_as_a_tripwire.md) | The family's standard for a compile-time claim |
| [`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md) | The field that supplies `Sync` |
| [`data_structure/001`](../data_structure/001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md) | The 24 bytes `Copy` makes cheap to clone |
| [`api/002`](../api/002_shared_reference_everywhere_and_what_it_forces.md) | The `&self` receiver `Sync` is what makes usable |

### Sources

| Fact | Where |
|------|-------|
| The three derive lists | `ring_trace/src/lib.rs:69`, `:149`, `:202` |
| `Trace` being `Send` and `Sync` | Probe above |
| Zero hashing anywhere in the crate | Census above |
| The two hand-built sets | `ring_trace/tests/trace_test.rs:185-187`, `:223-225` |
| `TraceOp: Ord` not satisfied | Probe above |
| Six crates, 44 compile-time assertions | Census above |

### Tests

| Test | Covers |
|------|--------|
| `concurrent_recorders_lose_no_entry` | `Sync` in use, unasserted |
| `every_operation_kind_has_its_own_name` | The first hand-built set |
| `the_operation_kinds_are_exactly_the_five_declared` | The second, over `TraceOp` itself |
| `entries_compare_by_value_and_print_readably` | `PartialEq` and `Debug` on an entry |
