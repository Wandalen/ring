# Non-Functional Requirement: Absent Unless Called

### Scope

- **Purpose**: State the requirement that separates this crate from the alternative it exists instead of — a check on the family's hot path — and give each constraint a way to be measured.
- **Responsibility**: The constraints, their measurement commands, and their current readings.
- **In Scope**: Dependency direction, per-call cost, allocation, the absence of any implicit invocation.
- **Out of Scope**: Why a hot-path check would be the wrong trade (→ [`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md)'s P4); what the checks do (→ [`api/001`](../api/001_the_check_surface.md)).

### Requirement

**Nothing in the family pays for this crate unless it calls it.** The obvious
alternative — making `ring_seqno`'s arithmetic defensive, or gating claims on a
validity check — puts the cost of a diagnostic on the family's most-executed
path, to catch a state a correct program never reaches. This crate is the other
choice, and this instance is the set of constraints that keep it honest.

### Constraints

| # | Constraint | Measurement | Reading (2026-08-28) |
|---|---|---|---|
| C1 | No family crate depends on `ring_debug` | `command grep -rln ring_debug */Cargo.toml ` | Only its own manifest |
| C2 | No family `src/` mentions `ring_debug` | `command grep -rn --include=*.rs ring_debug */src/ \| command grep -v '^ring_debug/'` | 2 lines — `ring_spsc` doc-comment citations, not a dependency |
| C3 | Every check is O(1) and allocation-free | Inspection — `check`, `Watch::new` and `Watch::observe` are two atomic loads and four comparisons each; `check_ends`'s cost is different and backend-dependent, and is not what this count covers | Holds |
| C4 | Nothing runs implicitly | No `Drop`, no constructor hook, no `static` initialiser in `src/lib.rs` | Holds |
| C5 | Checks do not perturb the ring | `checking_leaves_both_cursors_where_they_were` | Passes |

**C1 and C2 are the load-bearing pair, and both are mechanically checkable.**
Together they say the dependency arrow points only one way: `ring_debug` knows
about the family, and the family does not know about `ring_debug`. As long as
that holds, a build that never mentions this crate compiles and runs exactly as
if it did not exist.

**C2 is the one that could erode quietly.** A well-meant `debug_assert!(
ring_debug::check( &self.cursors ).is_ok() )` inside `ring_spsc`'s claim path
looks like a free improvement, and is not one. `debug_assert!` expands to
`if cfg!( debug_assertions ) { assert!( .. ) }`, so the expression is *compiled*
in every profile and *executed* only where `debug_assertions` is on. Two things
follow, and they are the two C1 and C2 exist to prevent:

- The dependency edge is real in all profiles — `ring_spsc` would depend on
  `ring_debug`, inverting the arrow, whatever the optimiser does with the branch.
- Every claim in a dev or test build would pay a pair of `Acquire` loads. Release
  builds would not, because the branch folds away; but the family's own tests and
  its `--cfg loom` runs are precisely dev builds, so the paths most sensitive to
  extra ordering are the ones that would pay.

The grep is the guard, and it catches the edge rather than the cost — which is
the right thing to catch, since the edge is what makes the cost possible.

### Why this is stated rather than assumed

A crate whose whole purpose is checking is under standing pressure to be invoked
automatically — that is what "checking" usually means, and an opt-in checker
looks like an unfinished automatic one. The requirement records that the opt-in
is the design and not a stage of it.

The trade is explicit: **this crate catches nothing that nobody runs it against.**
[`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md)
already narrows where it can be run at all, and this instance narrows when. Both
are real costs, accepted so that the family's claim path stays free of them.

### Verification

```bash
# C1 and C2 — the dependency arrow points one way only.
command grep -rln 'ring_debug' */Cargo.toml 
command grep -rn --include=*.rs 'ring_debug' */src/ | command grep -v '^ring_debug/'

# C5, and the rest of the suite.
cargo nextest run -p ring_debug --all-features
```

C1 should list exactly one file; C2 currently prints two lines — `ring_spsc`
doc comments citing `ring_debug`'s docs by path (`ring_debug/docs/invariant/002`,
`ring_debug/docs/pattern/001`), not a `use` or a call. C1 is what guarantees no
functional dependency exists; C2 as written cannot distinguish a citation from
a real one, so "prints nothing" is not the invariant it looks like.

### What is not measured here

**Per-call latency has no number.** C3 asserts O(1) and allocation-free by
inspection, not by benchmark, because a wall-clock figure for two atomic loads
would measure the harness rather than the code, and because nothing depends on
the figure — the checks run in tests, not in a loop that has a budget. If that
changes, the measurement belongs in `ring_bench`, which is where the family's
timing machinery lives.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| Q1 | A family crate takes a dependency on `ring_debug` | C1's grep lists a second manifest |
| Q2 | A `debug_assert!` calling `check` lands in a claim path | C2's grep is non-empty; the dependency edge is real in every profile, and dev/test/loom builds pay two extra `Acquire` loads per claim |
| Q3 | A check gains a `store` | C5 fails |
| Q4 | The crate grows a `Drop` or a lazily-initialised global | `tests/manual/readme.md`'s M3 stage greps for it by hand — the only guard here that is not automated |

**Q4 has a guard, but not an automated one.** `tests/manual/readme.md`'s M3
stage greps `ring_debug/src/lib.rs` for `impl Drop`, `static `,
`lazy_static`, `OnceLock` and `ctor` by hand — the same check a mechanical gate
would run, just not run automatically. A gate that detected implicit invocation
*across the family* would need to reason about initialisation order in every
crate, which is more machinery than the risk warrants for an 8-item crate — but
the single-file check this crate needs already exists as a dated stage of its
own test plan.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- loads this crate issues, per entry point --'
awk '/^pub fn |^  pub fn /{ f = $0; sub( /^ */, "", f ); sub( /\(.*/, "", f ) } /load\( OBSERVE \)/{ print f }' ring_debug/src/lib.rs | uniq -c
echo '-- what check_ends calls instead --'
awk '/pub fn check_ends/{ f = 1 } f && /let (pending|free)/{ sub( /^ */, "" ); print "  " $0 } f && /^}$/{ exit }' ring_debug/src/lib.rs
echo '-- one level down: ring_core dispatches on the backend --'
awk '/pub fn len\( &self \)/{ f = 1 } f && /Inner::/{ sub( /^ */, "" ); print "  " $0 } f && /^  }$/{ exit }' ring_core/src/lib.rs
echo '-- two levels down, on the spsc arm --'
awk '/pub fn available\( &self \)/{ f = 1 } f && /load\(/{ sub( /^ */, "" ); print "  available: " $0 } f && /^  }$/{ exit }' ring_spsc/src/lib.rs
awk '/fn occupancy\( &self \)/{ f = 1 } f && /load\(/{ sub( /^ */, "" ); print "  occupancy: " $0 } f && /^  }$/{ exit }' ring_spsc/src/lib.rs
echo '-- the mpsc arm is a different path again --'
awk '/pub fn available\( &self \)/{ f = 1 } f && /^  }$/{ exit } f && NF { l = $0; sub( /^ */, "", l ); print "  mpsc available:     " l }' ring_mpsc/src/lib.rs
awk '/pub fn free_capacity\( &self \)/{ f = 1 } f && /^  }$/{ exit } f && NF { l = $0; sub( /^ */, "", l ); print "  mpsc free_capacity: " l }' ring_mpsc/src/lib.rs
echo '-- fences in this crate docs, by type --'
printf '  files carrying an sh block:   %s\n' "$( command grep -rl '^```sh$' ring_debug/docs/ | wc -l )"
printf '  files carrying a bash block:  %s\n' "$( command grep -rl '^```bash$' ring_debug/docs/ | wc -l )"
command grep -rl '^```bash$' ring_debug/docs/ | sed 's|^ring_debug/docs/|    bash in: |'
echo '-- the single literal the corpus checker compares each fence against (backticks shown as ~) --'
command grep 'lines\[ i \].rstrip()' bench_harness/gate/corpus/recipes.py | tr '\140' '~' | sed 's/^/  /'
printf '  ring_* doc files carrying a bash block: %s\n' "$( command grep -rl '^```bash$' ring_*/docs/ | wc -l )"
```

Live output:

```
-- loads this crate issues, per entry point --
      2 
-- what check_ends calls instead --
  let pending = consumer.len();
  let free = producer.free_capacity();
-- one level down: ring_core dispatches on the backend --
  ConsumerInner::Spsc( consumer ) => consumer.available(),
  ConsumerInner::Mpsc( consumer ) => consumer.available(),
  ConsumerInner::Crossbeam( queue ) => queue.len(),
-- two levels down, on the spsc arm --
  available: let consumed = self.ring.cursors.consumer().load( OWN );
  available: let produced = self.ring.cursors.producer().load( GATING );
  occupancy: let produced = self.ring.cursors.producer().load( OWN );
  occupancy: let consumed = self.ring.cursors.consumer().load( GATING );
-- the mpsc arm is a different path again --
  mpsc available:     pub fn available( &self ) -> usize
  mpsc available:     {
  mpsc available:     let from = self.position();
  mpsc available:     let end = self.ring.contiguous_end( from, self.ring.capacity().get() );
  mpsc available:     from.distance_to( end ) as usize
  mpsc free_capacity: pub fn free_capacity( &self ) -> usize
  mpsc free_capacity: {
  mpsc free_capacity: self.claimer.headroom()
-- fences in this crate docs, by type --
  files carrying an sh block:   41
  files carrying a bash block:  1
    bash in: non_functional_requirement/001_absent_unless_called.md
-- the single literal the corpus checker compares each fence against (backticks shown as ~) --
      fence = lines[ i ].rstrip()
  ring_* doc files carrying a bash block: 11
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | A1 — C5 as a guarantee; the 8 items C3 counts |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_checking_a_pair_without_touching_it.md](../algorithm/001_checking_a_pair_without_touching_it.md) | C3's complexity table, and N4 — the `store` C5 forbids |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_reaching_the_cursors_of_a_live_ring.md](../integration/001_reaching_the_cursors_of_a_live_ring.md) | The other half of the trade — where the crate can be called, as against when |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | P4 — the hot-path fix this requirement exists instead of |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The four dependency edges, and nothing depending back |
| [`src/lib.rs`](../../src/lib.rs) | The 8 public items; no `Drop`, no `static` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/debug_test.rs` | C5 — `checking_leaves_both_cursors_where_they_were`. C1, C2 and C4 are not testable from inside the crate: a test here cannot observe what other crates depend on, which is why they are greps in a manual plan rather than assertions |

### DB45 — the cost model counts the loads three entry points issue and none of the loads the fourth does

C3 prices the crate as *"8 public items over two atomic loads and four
comparisons."* Two loads is exact for `check`, for `Watch::new` and for
`Watch::observe`: six `load( OBSERVE )` sites, two apiece.

`check_ends` has none. Its cost is `Consumer::len()` plus
`Producer::free_capacity()`, which `ring_core` dispatches on a backend enum. On the
SPSC arm that resolves to `available()` and `occupancy()`, **four loads — each
cursor read twice, once at `OWN` and once at `GATING`.** On the MPSC arm it
resolves to `position()`, `contiguous_end()` and `Claimer::headroom()`, a different
path with a different count.

Three consequences follow, and the third is why this is recorded here rather than
filed as a typo in a table.

- **The crate's declared ordering does not reach its only Contract-reachable entry
  point.** `const OBSERVE : Ordering = Ordering::Acquire` is argued at length in
  [`algorithm/001`](../algorithm/001_checking_a_pair_without_touching_it.md) and
  `check_ends` never uses it.
- **The skew window is twice as wide as documented.**
  [`api/001`](../api/001_the_check_surface.md)'s B1 asks the caller to keep the ring
  quiescent across two loads; for `check_ends` it is four, taken at four moments.
- **The cost is not a property of this crate.** C3 says the reading is obtained by
  inspection, and no amount of inspecting `ring_debug` produces it — the number
  lives two crates away and changes with a `ring_core` enum arm this crate cannot
  see.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F '| C3 |' ring_debug/docs/non_functional_requirement/001_absent_unless_called.md
```

Live output:

```
| C3 | Every check is O(1) and allocation-free | Inspection — `check`, `Watch::new` and `Watch::observe` are two atomic loads and four comparisons each; `check_ends`'s cost is different and backend-dependent, and is not what this count covers | Holds |
```

**Disposition:** applied — C3's Measurement cell no longer prices the whole
crate as "8 public items over two atomic loads and four comparisons"; it now
scopes that count to `check`, `Watch::new` and `Watch::observe` specifically
and states that `check_ends`'s cost is different, backend-dependent, and not
covered by this inspection. Now prints: `and is not what this count covers`

### DB46 — the constraints called mechanically checkable sit in the one fence the corpus checker does not read

The Verification block giving C1 and C2 their commands is opened with a `bash`
fence. `recipes.py` compares each opening fence against a single literal — an `sh`
fence — so a `bash` block is not merely unchecked, it is invisible: never executed,
never exit-checked, never compared against a recorded output. It is the only file
in this crate's docs carrying one.

Both greps are correct, and both still pass — measured independently in
[`integration/002`](../integration/002_the_edges_that_were_never_drawn.md), which
finds the reverse-dependency count at zero in every manifest section. So nothing is
currently wrong. What is missing is the property this document claims for itself:
*"C1 and C2 are the load-bearing pair, and both are mechanically checkable"*, and
*"The grep is the guard."*

**A guard nothing executes is a guard whose next failure is silent**, which is
exactly the shape of the risk C2 is written to catch — a well-meant
`debug_assert!` landing in a claim path and inverting the dependency arrow. The
document names that scenario, supplies the command that would catch it, and puts
the command where the machine does not look.

The fence choice is not unique to this crate: the recipe counts the `ring_*` doc
files carrying a `bash` block family-wide, and it is 11.
