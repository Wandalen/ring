# Non-Functional Requirement: What the Read Path Costs

### Scope

**Purpose:** Establish what `ring_consume` is required to *be* rather than to do
— allocation-free, non-blocking, `no_std`-clean, `unsafe`-free — and measure each
claim against the crate and the eight-crate chain it pulls in, rather than
against the one file that is easiest to grep.

**Responsibility:** The four negative properties, each stated as a claim over a
named scope, each checked over the whole of that scope.

**In Scope:** `ring_consume/src/lib.rs` and all seven crates
`cargo tree -p ring_consume --edges normal` reports; heap allocation measured
at runtime with a counting `GlobalAlloc`; the workspace lint table.

**Out of Scope:** Throughput, latency, and contention cost — the crate is
single-consumer, so there is no contention to measure. Memory layout, which is
[`data_structure/001`](../data_structure/001_sixteen_and_twenty_four.md).

---

## The Scope of a Claim Is Where It Has to Be Checked

This document exists in the shape it does because its sibling in `ring_claim`
did not. That file claimed "nothing allocates" over a scope of seven crates and
verified it with a `grep` over one, and the claim was false — one heap
allocation per `claim`, two crates down, found only by measuring
(`ring_claim` CL55). The same shape of error is available here, and the same
allocation site is in this crate's chain, so every claim below is checked over
the scope it is stated at.

```sh
cd "$(git rev-parse --show-toplevel)"

# the three negative properties across the whole chain, not just this crate
for c in $( cargo tree -p ring_consume --edges normal --prefix none 2>/dev/null \
            | awk '{print $1}' | grep '^ring_' | sort -u ); do
  s=$( grep -vE "^[[:space:]]*//" ring/$c/src/*.rs 2>/dev/null )
  printf '  %-14s alloc:%-2s std::%-2s block:%s\n' "$c" \
    "$( echo "$s" | grep -cE 'Vec<|Box<|String|alloc::|\.collect\(\)' )" \
    "$( echo "$s" | grep -cE '\bstd::' )" \
    "$( echo "$s" | grep -cE '\bloop\b|park|spin_loop|yield_now' )"
done
echo

# and the same for ring_claim's chain, for contrast
for c in $( cargo tree -p ring_claim --edges normal --prefix none 2>/dev/null \
            | awk '{print $1}' | grep '^ring_' | sort -u ); do
  s=$( grep -vE "^[[:space:]]*//" ring/$c/src/*.rs 2>/dev/null )
  printf '  %-14s alloc:%-2s std::%-2s block:%s\n' "$c" \
    "$( echo "$s" | grep -cE 'Vec<|Box<|String|alloc::|\.collect\(\)' )" \
    "$( echo "$s" | grep -cE '\bstd::' )" \
    "$( echo "$s" | grep -cE '\bloop\b|park|spin_loop|yield_now' )"
done
```

Live output:

```
  ring_align     alloc:0  std::0  block:0
  ring_atomic    alloc:0  std::0  block:0
  ring_barrier   alloc:0  std::0  block:0
  ring_consume   alloc:0  std::0  block:0
  ring_cursor    alloc:0  std::0  block:0
  ring_seqno       alloc:0  std::0  block:0
  ring_types     alloc:0  std::0  block:0
  ring_wait      alloc:0  std::2  block:2

  ring_align     alloc:0  std::0  block:0
  ring_atomic    alloc:0  std::0  block:0
  ring_claim     alloc:0  std::0  block:0
  ring_cursor    alloc:0  std::0  block:0
  ring_gating    alloc:1  std::0  block:0
  ring_seqno       alloc:0  std::0  block:0
  ring_types     alloc:0  std::0  block:0
```

`ring_consume`'s own source is clean on all three, and so — since commit
`b7e075ca` — is its chain's allocation column. It was not when this section was
written: `ring_cursor` read `alloc:1` and every barrier-consulting call in this
crate paid for it (CN34). The one hit left in either chain is `ring_gating`'s
`cursors : Vec< PaddedCursor >`, an owned field allocated once at construction
and never per call, which is the shape a static scan cannot tell apart from the
one that was removed — the whole reason CN34 is a measurement and not a grep.
The `std::` and blocking columns are unchanged, and the two Tier 5 siblings are
still not clean in the same places.

---

### CN34 — Every Read of the Available Range Allocated, Two Crates Down, and Now Nothing Does

The static scan used to put one allocation site in the chain, `ring_cursor`,
and it no longer does. Whether a `Consumer` call reached it while it was there
is a question a `grep` cannot answer, so it was
measured — and the measurement was a scratch binary pasted into this document,
run once by hand, and deleted by its own last line. It was right, and it left
nothing behind. Commit `b7e075ca` then removed the allocation from
`ring_cursor::slowest` without touching one document that quoted its cost, and
every number in this section became false at once with nothing reporting it.

The probe is now a test. It measures the same six call shapes, in the same
order this table lists them, and fails the suite when the answer changes:

```sh
cd "$(git rev-parse --show-toplevel)"

echo '  -- the fold every barrier-consulting row below reaches, two crates down --'
awk '/^pub fn slowest\( cursors : &\[ PaddedCursor \] \)/{ f = 1 } f { print } f && /^\}$/{ exit }' ring_cursor/src/lib.rs
echo '  -- every call shape this crate asserts allocation-free, from the test itself --'
tr '\n' ' ' < ring_consume/tests/allocation_test.rs | tr -s ' ' \
  | command grep -oE '\( 0, 0 \), "[^"]+"' | sed 's/( 0, 0 ), /    /'
printf '    call shapes asserted allocation-free: %s\n' \
  "$( tr '\n' ' ' < ring_consume/tests/allocation_test.rs | tr -s ' ' \
      | command grep -oE '\( 0, 0 \), "[^"]+"' | wc -l )"
command grep -c 'control_calls >= 1' ring_consume/tests/allocation_test.rs \
  | sed 's/^/    assertions that a deliberate allocation is seen: /'
```

Live output:

```
  -- the fold every barrier-consulting row below reaches, two crates down --
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}
  -- every call shape this crate asserts allocation-free, from the test itself --
    "position() ×1000"
    "available() ×1000"
    "available_up_to( 8 ) ×1000"
    "commit_available() ×1000"
    "available() on an empty barrier — zero before the fix too, and therefore no evidence"
    "commit_available() on an empty barrier — zero before the fix too, and therefore no evidence"
    call shapes asserted allocation-free: 6
    assertions that a deliberate allocation is seen: 1
```

| Call | Allocations per 1000, before `b7e075ca` | Allocations per 1000, now |
|------|---------------------:|---------------------:|
| `Consumer::position()` | 0 | 0 |
| `Consumer::available()` | **1000** | 0 |
| `Consumer::available_up_to( 8 )` | **1000** | 0 |
| `Consumer::commit_available()` | **1000** | 0 |
| `Consumer::available()`, empty barrier | 0 | 0 |
| `Consumer::commit_available()`, empty barrier | 0 | 0 |

The before column is not quoted from memory. Restoring the `collect()` body to
`ring_cursor::slowest` and re-running `no_read_of_the_available_range_allocates`
fails on its second measured row with `left: (1000, 8000)` — one allocation of
one `Seq` per call, a thousand times, for exactly the reason the removed `Vec`
existed:

```rust
// ring_cursor::slowest in ring_cursor/src/lib.rs, before commit b7e075ca
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
  ring_seqno::slowest( &positions )
}
```

The `Vec` existed only to adapt `&[ PaddedCursor ]` to the `&[ Seq ]` that
`ring_seqno::slowest` takes. Both were folds over the same iterator; neither
needed the intermediate. This was the identical site and identical cause as
`ring_claim` CL55, reached by a different path — the read half of the handshake
allocated for the same reason the write half did, and one four-line change
closed both at once. That is also why it was declined here twice: the site was
somebody else's, and a `ring_consume`-only pass could see the cost without
being able to remove it.

Three details made it worse than the bare number suggested. The first is gone
with the allocation; the other two are properties of this crate and are still
true.

**It allocated when there was nothing to read.** The measurement ran
`commit_available()` a thousand times; the first call advanced the cursor to the
frontier and the remaining 999 had an empty run to commit. All 1000 allocated.
The work was unconditional — the `Vec` was built before anything learned the
answer was zero. The replacement test keeps that exact shape, so a `Vec`
reintroduced anywhere on the path is caught by the 999 calls that do no work.

**The barrier is still the hot path.** `position()`, which reads the consumer's
own cursor directly, never allocated. Every method that has to look at the
barrier did. A consumer polling for work calls exactly those methods, in a loop,
and that loop is the crate's entire intended usage — which is why the guard has
to sit on those calls specifically rather than on the crate.

**The zero cases are why nobody noticed, and two of them are still in the
table.** An empty `Barrier` gives `slowest` an empty slice, and a zero-length
`Vec` never reaches the allocator. Any test that constructs a consumer with no
dependencies measured zero truthfully, about a case no real ring is ever in.
Both empty-barrier rows read zero in both columns: they are kept, and labelled
as such in the test's own assertion messages, because a row that cannot
distinguish the two states is worth naming rather than deleting
([`pitfall/002`](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md)).

**Cost:** was reachable, and is now zero. One small allocation, amortised over
the batch for a consumer draining large batches; one allocation per poll,
forever, for a consumer polling an idle ring, to compute a number that is
usually zero.

**Disposition:** applied — the `Vec` is gone from `ring_cursor/src/lib.rs`
and this crate's share of the guarantee is a test rather than a paragraph:
`ring_consume/tests/allocation_test.rs` asserts all six of the rows above
at `( 0, 0 )`, with a control arm that fails if the counting allocator is not
installed, so a zero means *nothing allocated* rather than *nothing was
watching*. Now prints: `call shapes asserted allocation-free: 6`

---

### CN35 — The Chain Is Neither `no_std`-Clean Nor Non-Blocking, and `ring_claim`'s Is Both

The two Tier 5 siblings hold different properties, and nothing in either crate
says so:

| Property, over the whole chain | `ring_claim` (7 crates) | `ring_consume` (8 crates) |
|--------------------------------|:-----------------------:|:-------------------------:|
| Zero `std::` paths | ✔ | ✘ — 2, in `ring_wait` |
| Zero blocking constructs | ✔ | ✘ — 2, in `ring_wait` |
| Zero allocation sites | ✘ — 1 site, at construction | ✔ — since `b7e075ca` |

The allocation row reversed after this table was written and is left in to
show which way: the property `ring_consume` lacked is the one it now holds, and
the sibling it was measured against still carries a site. The remaining
difference is `ring_wait`, and
[`integration/002`](../integration/002_eight_methods_and_the_one_that_is_called.md)
establishes how it arrives: `ring_consume` depends on `ring_barrier`, which
depends on `ring_wait` for exactly one of its nine methods — `wait_for` — which
`ring_consume` never calls and which has zero library callers anywhere in the
family.

The three lines that cost the chain both properties:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E '\bstd::|\bloop\b|park|spin_loop|yield_now'
```

Live output:

```
        core::hint::spin_loop();
      std::thread::yield_now();
      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
```

These are correct lines in the crate that owns them — `ring_wait` is the
family's blocking primitive and blocking is its job. The finding is not about
`ring_wait`. It is that a crate whose entire design is *not to block* cannot
state that property over its own dependency chain, and cannot be `no_std`,
because of a transitive edge it does not use.

A `#[ cfg( feature = "wait" ) ]` on `ring_barrier::wait_for` with `ring_wait`
made an optional dependency would restore both properties to `ring_consume` at
the cost of one attribute and one manifest line. Nothing in the family proposes
it, because nothing in the family measures either property.

**Cost:** reachable, and asymmetric in an unrecorded way. A reader who checks
`ring_claim`'s chain, finds it clean, and generalises to "the Tier 5 primitives
are `no_std`-clean" is wrong about half of them.

**Disposition:** declined — the fix this section names
(`#[ cfg( feature = "wait" ) ]` gating `ring_barrier::wait_for` behind an
optional `ring_wait`) would touch `ring_barrier/Cargo.toml`, six-plus tests in
`ring_barrier/tests/barrier_test.rs`, and ripples into 27 files under
`ring_barrier/docs/` — including `ring_barrier/docs/api/001_nine_methods_over_one_borrowed_slice.md`'s
own method count — all out of scope for a `ring_consume`-only pass.

---

### CN36 — One of Four Properties Has Enforcement, and It Is the One Nobody Would Break

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[workspace.lints/,/^\[[^w]/p' Cargo.toml
grep -c 'unsafe' ring_consume/src/lib.rs
ls ring_consume/tests/
```

Live output:

```
[workspace.lints.rust]
rust_2018_idioms = { level = "warn", priority = -1 }
future_incompatible = { level = "warn", priority = -1 }
missing_docs = "warn"
missing_debug_implementations = "warn"
unsafe-code = "deny"
unsafe_op_in_unsafe_fn = "deny"
unreachable_pub = "warn"
meta_variable_misuse = "warn"
redundant_lifetimes = "warn"
unit_bindings = "warn"
unused_lifetimes = "warn"
unused_macro_rules = "warn"
# `loom` is set by RUSTFLAGS, not by any feature, so rustc has no other way to
# learn it is a real cfg. Declared once here rather than per crate: the lints
# table is inherited workspace-wide, and a crate cannot both inherit it and add
# its own. Only ring_atomic, ring_cursor and ring_publish read the cfg — see
# ring_atomic's module documentation on the seam.
unexpected_cfgs = { level = "warn", check-cfg = [ 'cfg(loom)' ] }

[workspace.lints.clippy]
# Unsafe: one operation per block, each with its own `// SAFETY:`.
undocumented_unsafe_blocks = "deny"
multiple_unsafe_ops_per_block = "deny"
unnecessary_safety_comment = "warn"
unnecessary_safety_doc = "warn"
cast_ptr_alignment = "warn"
ptr_as_ptr = "warn"
ptr_cast_constness = "warn"
mem_forget = "warn"
# Concurrency: locks are what this family exists to avoid; refcount bumps stay visible.
mutex_atomic = "warn"
mutex_integer = "warn"
rc_mutex = "warn"
clone_on_ref_ptr = "warn"
# Determinism: hash iteration order would leak into the delivery order.
iter_over_hash_type = "warn"
# Sequence and index arithmetic.
cast_sign_loss = "warn"
precedence_bits = "warn"
# Hygiene.
dbg_macro = "warn"
exit = "warn"
infinite_loop = "warn"
large_stack_frames = "warn"
todo = "warn"
unimplemented = "warn"
unused_result_ok = "warn"

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"
private_intra_doc_links = "deny"
bare_urls = "warn"
missing_crate_level_docs = "warn"
unescaped_backticks = "warn"
0
allocation_test.rs
consume_test.rs
manual
```

| Property | True today | Enforced by | Breaks silently |
|----------|:----------:|-------------|:---------------:|
| No `unsafe` | ✔ | `unsafe-code = "deny"` + `undocumented_unsafe_blocks = "deny"` | **no** |
| No allocation (chain) | ✔ — zero per read, measured | `tests/allocation_test.rs` | **no** |
| Never blocks (chain) | ✘ — `ring_wait` is in it | nothing | **it already did** |
| `no_std`-clean (chain) | ✘ — `ring_wait` is in it | nothing — and declared in only 3 of 33 | **it already did** |

Two lints guard `unsafe` in a crate whose implementation is four comparisons, a
subtraction and two stores — code that would never have reached for `unsafe`.
Of the three properties a plausible change actually threatens, one now has a
guard and two still have none, and those two are still false at the scope this
document states them at. All three became false without a single test failing,
warning firing, or check reporting, because at the time no test, warning or
check looked.

The allocation row is the one that changed, and it is worth being exact about
why. It did not change because the property was enforced — nothing enforced it.
It changed because the site was removed for unrelated reasons two crates away,
and this document went on asserting the old number until somebody read it. The
guard now in the row is what makes the reverse impossible: the row reads ✔
because `tests/allocation_test.rs` fails when it stops being true, not because
the current answer happens to be zero.

Unlike `ring_claim`, this crate has no `tests/manual/readme.md` whose grep could
be widened to cover the remaining two — the cheapest fix available to its
sibling does not exist here. The equivalent would be a new manual check, or a
`deny` on `clippy::disallowed_types` naming `Vec` for the chain's leaf crates.

**Cost:** reachable. Two of four stated properties are false and nothing reports
it; of the two that hold, one is enforced twice and the other once.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| non_functional_requirement | [002](002_eleven_constants_and_the_one_that_is_shared.md) | the ordering constants, the other property nothing checks |
| integration | [002](../integration/002_eight_methods_and_the_one_that_is_called.md) | how `ring_wait` enters the chain and why nothing calls it |
| algorithm | [001](../algorithm/001_position_frontier_pending.md) | the call sequence CN34 measures |
| pitfall | [002](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) | the empty-barrier case that hid CN34's allocation |
| workaround | [001](../workaround/001_five_functions_none_const.md) | the `ring_seqno` surface `slowest` adapts to, and what the adapter costs |

### Sources

| What | Where |
|------|-------|
| The allocation site, while it existed | `ring_cursor::slowest`, `ring_cursor/src/lib.rs` |
| The test that now pins its absence | `ring_consume/tests/allocation_test.rs` |
| The `ring_wait` lines | `ring_wait/src/lib.rs:28,34,39` |
| The one method that reaches them | `ring_barrier/src/lib.rs:285` |
| The read path | `ring_consume/src/lib.rs:336,365,425` |
| The workspace lints | `Cargo.toml`, `[workspace.lints]` |

### Tests

| Claim | Verified by |
|-------|-------------|
| Zero allocations on every barrier-consulting call | `no_read_of_the_available_range_allocates`, six rows with a control arm |
| That the same test reports the old number when the `Vec` is restored | `left: (1000, 8000)` on its second measured row |
| Zero on `position()` and on an empty barrier | the same test, rows 1, 5 and 6 |
| `ring_wait` is the chain's only `std::`/blocking crate | the per-crate scan over `cargo tree` output |
| `ring_claim`'s chain has neither | the same scan run against `ring_claim` |
| Nothing enforces three of the four | no test, lint or manual check in the crate names them |
