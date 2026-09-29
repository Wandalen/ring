# Workaround: The `unsafe_code` Opt-Out and What Bounds It

### Scope

- **Purpose**: Record the ten `unsafe` lines this crate carries, the three private functions they live in, and the compile-fail tests that pin the argument bounding them.
- **Responsibility**: The opt-out's sites, its cost, and its deletion condition.
- **In Scope**: `#![ allow( unsafe_code ) ]` and every `unsafe` token in `src/lib.rs`.
- **Out of Scope**: The `Sync` impl specifically (→ [`002`](002_an_unsafe_impl_sync_on_a_type_whose_ends_are_not_sync.md)); the workspace lint table itself, which is a workspace-level decision, not this crate's own.

### The Constraint

Rust has one way to mutate through a shared reference — `UnsafeCell` — and no
safe primitive for *two threads, one array, provably disjoint index sets*. The
disjointness here is real and checkable: the producer writes only the slot at
its own cursor, the consumer reads only slots strictly below it, and the ranges
cannot intersect. What is missing is a way to state that to the compiler.

### The Sites

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -E 'unsafe'
```

Live output:

```
#![ allow( unsafe_code ) ]
unsafe impl< S : Send > Sync for Ring< S > {}
  unsafe fn slot( &self, seq : Seq ) -> &S
    unsafe { &*self.slots.at( seq ).get() }
  unsafe fn slot_mut( &self, seq : Seq ) -> &mut S
    unsafe { &mut *self.slots.at( seq ).get() }
    unsafe { self.ring.slot_mut( self.seq ) }
    unsafe { self.ring.slot_mut( self.seq ) }
    Some( unsafe { self.ring.slot( self.start.advanced_by( offset as u64 ) ) } )
    Some( unsafe { self.ring.slot_mut( self.start.advanced_by( offset as u64 ) ) } )
```

Ten lines: the crate attribute, one `unsafe impl Sync`, two `unsafe fn`
declarations with one `unsafe` block each, and four call sites.

**Every one of the four call sites is inside a guard type.** Two are
`Reservation`'s `Deref`/`DerefMut`, two are `Batch`'s `get`/`get_mut`. Neither
guard can be constructed except by a `claim` or a `drain` that has already
established the precondition, so no caller-writable expression reaches an
`unsafe` operation.

### What Pins the Argument

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'compile_fail doc tests:  '; grep -c 'compile_fail' src/lib.rs
```

Live output:

```
compile_fail doc tests:  6
```

The bounding argument is a set of negatives — no `Clone` on either end, no
`Sync` on either end, no second live split, no batch outliving its commit — and
negatives cannot be asserted by a passing test. `compile_fail` doc tests are the
only mechanism that checks them, and they run under `cargo test --doc`.

### Cost and Deletion

| Field | Value |
|-------|-------|
| Cost | Ten `unsafe` lines, each carrying a `SAFETY` comment the workspace's `undocumented_unsafe_blocks = "deny"` enforces |
| Ruled by | This crate's own design: the unsafe is sited here rather than in `ring_store` or `ring_slot` |
| Deletion condition | A safe API below this crate encapsulating the whole invariant — none exists, because the invariant is stated in cursors a storage-only crate does not hold |

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | Every site counted here |
| `readme.md` | The Overview this instance details |
| `../../../bench_harness/gate/declared/ring/unsafe_allowlist.txt` | Names this crate |

### SP50 — Ten Unsafe Lines, the Same Count as the Sibling

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/; do
  n=$( cat ${c}src/*.rs 2>/dev/null | grep -vE '^\s*(//|///|//!)' | grep -c 'unsafe' )
  [ "$n" -gt 0 ] && printf '%-14s %d\n' "${c%/}" "$n"
done
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
ring_mpsc      10
ring_spsc      10
```

Two crates, equal counts, same four kinds of site: the attribute, one
`unsafe impl Sync`, two `unsafe fn` accessors, four call sites inside guards.

The symmetry is the evidence for the siting rule — the unsafe tracks
"holds storage *and* the cursors bounding it", which is true of exactly these
two.

### SP51 — The Bounding Argument Is Six Negatives and Only `compile_fail` Can Check Them

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'compile_fail doc tests:  '; grep -c 'compile_fail' src/lib.rs
printf 'run under:               '; echo 'cargo test --doc'
```

Live output:

```
compile_fail doc tests:  6
run under:               cargo test --doc
```

Six of them. A passing test can only demonstrate that something *is* possible;
the safety argument here rests entirely on things being impossible, and
`compile_fail` is the only mechanism in the language that checks an
impossibility.

**They run under `cargo test --doc`, which is a separate invocation from the
ordinary suite.** A verification pass that runs only `cargo nextest run` checks
none of them.
