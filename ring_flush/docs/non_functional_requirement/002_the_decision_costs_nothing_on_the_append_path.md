# Non-Functional Requirement: The Decision Costs Nothing on the Append Path

### Scope

- **Purpose**: State the cost ceiling the acceptance table does not state — a policy consulted per record must not undo the property `ring_tls` was built to provide — and give it a measurement method and a threshold.
- **Responsibility**: Fix the quality attribute, the statement, how it is measured, and where the measurement lives.
- **In Scope**: Per-append consultation cost; the zero-atomic and zero-allocation constraints inherited from `ring_tls`'s append-path guarantee.
- **Out of Scope**: The flush path's own cost, which is the cold path (→ [Sequencing Seal, Drain and Reset](../algorithm/002_sequencing_seal_drain_reset.md)); the trade being tuned.

### Quality Attribute

**Non-interference** — that adding a policy to a staging buffer does not
degrade the property the staging buffer exists for.

**This requirement is not in the acceptance table**, and its absence is the
reason to write it down. This crate's own behavioural criterion grades
behaviour; nothing there grades cost. But `ring_tls`'s criterion grades cost
precisely —

> A `TlsBuffer` accumulates `N` items with **zero atomic operations** (asserted
> by a counting allocator/atomic shim)

— and a policy consulted on that same path can break that criterion from
outside the crate that owns it. **`ring_tls`'s test would go red for a defect
introduced in `ring_flush`.**

### Statement

**Consulting a `FlushPolicy` on the append path performs zero atomic
operations, zero allocations, zero locks, and no system calls, for all three
variants.**

Three independent claims:

| # | Claim | Broken by |
|---|-------|-----------|
| P1 | Zero atomics per consultation | A shared counter; an `Arc`; any `Ordering` |
| P2 | Zero allocations per consultation | Logging evaluations; a boxed policy; a `Vec` of anything |
| P3 | Zero locks and no clock reads | A `Mutex` around the counter; an `OnInterval` variant |

**P3's clock clause is what forecloses the obvious fourth policy.** A
time-based flush is the natural next variant and it would put a clock read on a
path constrained to arithmetic. The design's answer is that time-based flushing
is expressible as `OnBarrier` driven from a timer — the cost moves to the
caller's cadence, where it is already being paid
(→ [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)'s
forbidden table).

### The requirement is met more strongly than it was written

**As built there is no consultation on the append path at all.**
`Flusher::append` is one `push` into the staging buffer — no policy read, no
comparison, no branch on the variant. The policy is evaluated on `drive`, which
runs at the caller's cadence rather than per record.

That makes P1–P3 true vacuously rather than by careful arithmetic, which is a
better position to be in: there is no code on that path for a future change to
make expensive without the change being obvious.

**It was not built that way on purpose.** The original design had `OnBatch`
maintaining a `usize` incremented per append — the one variant that genuinely
seemed to need per-record work. That counter turned out to duplicate the
buffer's own occupancy exactly, and deleting it removed the last per-record
cost as a side effect (→ [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)'s
deleted Step 3, and `tests/manual/readme.md`'s F3).

**The claims themselves are unchanged and still worth stating**, because they
constrain what may be *added* later. A future `OnInterval`, or an `OnBatch`
variant that counts bytes rather than records, would reintroduce exactly the
per-append work that is currently absent.

### Measurement Method

| # | Measurement | Mechanism | Lives in |
|---|-------------|-----------|----------|
| C1 | Atomic count across `N` appends | The counting shim `ring_tls`'s criterion uses — **but it cannot be attached**, see below | **Nowhere.** No crate can inject a cursor into `ring_core::Producer` |
| C2 | Allocation count across `N` appends | Counting allocator — needs unsafe, blocked by G6. **Proxied** by capacity invariance | `ring_flush/tests/append_cost_test.rs` (proxy); exact form awaits `ring_testkit` |
| C3 | `size_of::< FlushPolicy >() == 16` | Asserted as `2 × size_of::< usize >()` rather than a literal, so it measures the shape rather than re-pinning an observation. Confirmed at 16 independently (`tests/manual/readme.md`'s F2) | `ring_flush/tests/flush_test.rs` |
| C4 | No `Drop` impl on `FlushPolicy` | `!mem::needs_drop::< FlushPolicy >()` — one line, no trybuild dependency, and stronger than checking for an `impl Drop` because it also rejects a variant whose payload has one | `ring_flush/tests/flush_test.rs` |
| C5 | ~~The counter is not shared~~ | **Moot — there is no counter.** `OnBatch` reads the buffer's own occupancy, which is thread-local by construction. A shared counter is now unrepresentable rather than merely reviewed against | — |

**"C1 and C2 cannot live in this crate" was true when written and is now half
wrong, for two different reasons.** The paragraph below is kept because its
first reason has genuinely lapsed and its second turned out to be the real one.

**C2 can live here now.** The original argument was that this crate does not
own the append path — `ring_tls` does. That stopped being true when
`Flusher::new` took the buffer by value: the buffer is unreachable to the
caller, so every record arrives through `Flusher::append`, which is this
crate's own function. Ownership of the path moved, and the measurement follows
it.

**But C2 is blocked anyway, by something this instance did not consider.** A
counting allocator means `unsafe impl GlobalAlloc`; the workspace sets
`unsafe-code = "deny"`; and gate G6 confines the opt-out to four declared
crates, each justifying it in `docs/workaround/readme.md`. `ring_flush` is not
one, and neither is `ring_tls` — **so option 1 in the table below is blocked
for C2 too**, not just here.

**G6 scans `src/` only, so an `#![allow(unsafe_code)]` in `tests/` would pass
the gate.** It would also defeat the gate's stated purpose. Recorded rather than
done: `tests/append_cost_test.rs` asserts the strongest *safe* proxy instead —
that the staging buffer's capacity never changes across `N` appends, since a
`Vec` that did not grow did not reallocate. That catches the realistic defect (a
buffer that silently expands instead of refusing) and misses per-record boxing.

**C1's obstacle is different again, and is not about unsafe.**
`ring_atomic::CountingSeq` counts operations on a cursor the caller supplies.
`TlsBuffer::flush_into` takes such a cursor, which is how `ring_tls`'s own suite
counts atomics — but `Flusher` does not use `flush_into`. It publishes through
`ring_core::Producer`, whose cursors are built privately inside `Ring::new` and
cannot be replaced. **There is no seam, in any crate.** C1 through a bound
policy is therefore not measurable today by any of the three options below.

*Original reasoning, retained:* this crate does not own the append path;
`ring_tls` does. The measurement that would catch a violation therefore has to
run in `ring_tls`'s test suite, against a buffer with one of this crate's
policies bound — which means it exists only if someone writes it there, and
nothing in either crate's acceptance row requires it.

**So the requirement's primary measurement is still unowned — but the reason
changed from *nobody wrote it* to *nobody can*.** `ring_tls`'s test asserts
zero atomics for a bare buffer; this crate's own test asserts behaviour with no
cost assertion at all. A policy introducing an atomic passes both, and would
keep passing however diligent anyone was, until either `ring_core` grows a seam
or `ring_testkit` gets an allowlist entry.

**The distinction matters for what to do next.** An unwritten test is a task; an
unwritable one is a design gap in a *different* crate, and filing it as a task
here would produce something no amount of effort in this crate could close.

| Where the gap could close | Cost |
|---------------------------|------|
| Add a policy-bound case to `ring_tls`'s existing shim test | **Blocked for both.** C1: `Flusher` publishes through `ring_core::Producer`, so there is no cursor to hand `CountingSeq`. C2: a counting allocator needs unsafe, and `ring_tls` is not on G6's allowlist either |
| Add the shim to `ring_flush`'s own test | Same G6 blocker for C2. **A safe proxy was written instead** — `tests/append_cost_test.rs`, capacity-invariance rather than allocation-counting |
| Put the shim in `ring_testkit` and use it from both | **Now the only route for C2, and forced rather than preferred.** Needs `ring_testkit`'s planned shared shim *and* an entry on G6's unsafe allowlist with a justification — a deliberate, reviewable widening rather than an incidental one. Does nothing for C1, which needs a seam in `ring_core` |

**The first option's *stated* obstacle was never the real one, and the fact is
worth keeping even though the option is now blocked for other reasons.** A
dev-dependency from `ring_tls` to `ring_flush` appears to close a cycle, since
`ring_flush` already depends on `ring_tls` — but Cargo permits dev-dependency
cycles specifically because dev-dependencies are not part of the normal build
graph. Confirm it rather than taking it on trust:

```sh
cd "$(git rev-parse --show-toplevel)"
# a two-crate scratch workspace: a → b as a dependency, b → a as a dev-dependency
d=$( mktemp -d )
trap 'rm -rf "$d"' EXIT
mkdir -p "$d/a/src" "$d/b/src" "$d/b/tests"
printf '[workspace]\nresolver = "2"\nmembers = ["a", "b"]\n' > "$d/Cargo.toml"
printf '[package]\nname = "a"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nb = { path = "../b" }\n' > "$d/a/Cargo.toml"
printf 'pub fn f() -> i32 { 42 }\n' > "$d/a/src/lib.rs"
printf '[package]\nname = "b"\nversion = "0.1.0"\nedition = "2024"\n\n[dev-dependencies]\na = { path = "../a" }\n' > "$d/b/Cargo.toml"
printf '// b never uses a; only its dev-dependency does\n' > "$d/b/src/lib.rs"
printf '#[test]\nfn calls_a() { assert_eq!( a::f(), 42 ); }\n' > "$d/b/tests/it.rs"
if ( cd "$d" && cargo check --workspace ) >/dev/null 2>&1
then echo 'cargo check --workspace: succeeds'
else echo 'cargo check --workspace: FAILS'
fi
if ( cd "$d" && cargo test -p b ) >/dev/null 2>&1
then echo 'cargo test -p b: passes'
else echo 'cargo test -p b: FAILS'
fi
```

Live output:

```
cargo check --workspace: succeeds
cargo test -p b: passes
```

**This matters because the intuition points the wrong way**, and it will point
the wrong way again for some other pair of crates. "It would be a cycle" is a
reason such tests do not get written, and it is not a real obstacle.

**It is, however, no longer the operative one here.** Two harder obstacles sit
behind it — no injectable cursor for C1, no unsafe budget for C2 — and neither
is solved by the dev-dependency the cycle worry was blocking. Option three is
now a necessity rather than the preference this paragraph called it, and even
then only for C2.

### Acceptance Threshold

| Measurement | Threshold | Status |
|-------------|-----------|--------|
| C1 | 0 atomics per append, all three policies | **Unmeasurable today** — no crate can inject a counting cursor into `ring_core::Producer`. Mitigated structurally: `append` is one `push` and touches no cursor at all |
| C2 | 0 allocations per append | **Proxied** — `appending_never_grows_the_staging_buffer` asserts capacity invariance across 512 appends for all three policies. The exact form needs `ring_testkit` plus a G6 allowlist entry |
| C3 | Exactly 16 bytes | **Asserted** — `the_policy_is_a_value` |
| C4 | No `Drop` | **Asserted** — `the_policy_is_a_value`, via `!needs_drop` |
| C5 | — | **Moot** — no counter exists to share |

**C3 and C4 are cheap, local, and worth writing immediately** even though they
are the weaker half. They are proxies: a policy that stayed 16 bytes and
`Drop`-free is very unlikely to have gained an atomic. They are not proofs, and
recording them as proxies rather than as the requirement is the honest framing.

**The requirement is therefore currently satisfied by construction and
unverified by test** — which is exactly the state that decays silently, and the
reason it is written down as a requirement with a named gap rather than left as
an implicit design intention.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_evaluating_a_policy_at_an_append.md](../algorithm/001_evaluating_a_policy_at_an_append.md) | The path this ceiling constrains; its forbidden table is P1–P3 restated |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_policy_enum.md](../data_structure/001_the_policy_enum.md) | C3 and C4's subject; its refused shapes are the violations |
| [../data_structure/002_the_flush_log.md](../data_structure/002_the_flush_log.md) | P2's hazard — a structure that allocates, kept off this path deliberately |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_three_triggers_proven_by_a_flush_log.md](001_three_triggers_proven_by_a_flush_log.md) | The stated criterion; this one is the unstated companion |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) | The `ring_tls` seam across which this requirement is enforced and cannot be tested |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | Row 175's shim; row 176's silence on cost |

### Tests

| File | Relationship |
|------|--------------|
| `tests/append_cost_test.rs` | C2's proxy, both halves — `appending_never_grows_the_staging_buffer` across 512 appends and three policies, and `a_full_staging_buffer_refuses_and_keeps_its_capacity`, which is the half that would actually fail first: capacity staying constant means nothing unless the buffer is genuinely driven to its limit |
| `tests/flush_test.rs` | C3 and C4, both in `the_policy_is_a_value`. C3 is asserted as `2 × size_of::< usize >()` rather than a literal `16`, so it measures the shape instead of re-pinning whatever was observed; the literal was independently confirmed at 16 (`tests/manual/readme.md`'s F2). C4 uses `needs_drop` rather than the trybuild case this row specified — one line, no build dependency, and strictly stronger |
| `ring_tls/tests/tls_test.rs` | Where C1 and C2 belong, reachable today via a dev-dependency on this crate |

### FL35 — The One Block Introduced by "Confirm It Rather Than Taking It on Trust" Is Both Unrun and Unrunnable

The instruction, the block it introduces, and what the corpus gate does with it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the sentence and the fence beneath it --'
awk '/^### FL/{ exit } /taking it on trust|^```bash|^# a two-crate|^cargo /{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md
echo '  -- every bash-fenced block in this crate corpus, by file --'
for f in $( find ring_flush/docs -name '*.md' | sort )
do
  n=$( command grep -c '^```bash' "$f" )
  [ "$n" -gt 0 ] && printf '    %-58s %s\n' "${f#ring_flush/docs/}" "$n"
done
printf '    total bash blocks the recipes gate never opens: %s\n' \
  "$( command grep -rc '^```bash' ring_flush/docs --include='*.md' 2>/dev/null | command grep -v ':0$' | cut -d: -f2 | paste -sd+ | bc )"
echo '  -- and whether the scratch workspace the block needs is ever created --'
printf '    lines creating it: %s\n' \
  "$( awk '/^### FL/{ exit } /mkdir|cargo new|cat >|tempfile|mktemp/' \
       ring_flush/docs/non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md | wc -l )"
```

Live output:

```
  -- the sentence and the fence beneath it --
    142: graph. Confirm it rather than taking it on trust:
    146: # a two-crate scratch workspace: a → b as a dependency, b → a as a dev-dependency
    169: cargo check --workspace: succeeds
    170: cargo test -p b: passes
  -- every bash-fenced block in this crate corpus, by file --
    total bash blocks the recipes gate never opens: 
  -- and whether the scratch workspace the block needs is ever created --
    lines creating it: 2
```

[`workaround/002`](../workaround/002_the_compilation_boundary_that_was_never_built.md)'s
FL52 found a ```` ```bash ```` block whose sentence told the reader to run it,
and recorded the general shape: every such block in the corpus is a check
somebody thought they had. This is the same shape one directory over, and it is
worse in a way worth separating.

**FL52's block was correct and invisible.** `cargo tree --depth 1` would have
printed the right answer; only the fence type stopped it. **This block is
invisible and would not work if it were not.** It names a two-crate scratch
workspace in a comment and never creates one — nothing in the instance makes a
directory, writes a manifest, or reaches a temporary path. Re-fence it ```` ```sh ````
and `cargo check --workspace` runs against *this* workspace, succeeds for
reasons unrelated to the claim, and `cargo test -p b` fails on a package that
does not exist. The gate would then report a failing recipe, which is the right
verdict for the wrong reason.

**The claim it was meant to settle is correct** — Cargo does permit
dev-dependency cycles, and the instance is right that the intuition points the
wrong way. That is exactly why the unrunnable block matters: the paragraph's
whole value is that it corrects a widely-held wrong belief, and it offers a
verification the reader cannot perform. A reader who doubts it is left where
they started.

Nine such blocks exist across this crate's corpus. They are not failing recipes;
they are not recipes at all, and the gate reports zero problems for every one.

**Disposition:** applied — this block is now fenced ```` ```sh ```` and
genuinely creates the two-crate scratch workspace it names (a `mktemp -d`
workspace with both manifests and a real dev-dependency edge, cleaned up via
`trap ... EXIT`), rather than naming one in a comment and never building it.
`cargo check --workspace` and `cargo test -p b` now run for real against that
workspace on every gate pass, and both still confirm the claim: the
dev-dependency cycle is permitted and the test passes. The crate-wide census
above now finds zero remaining ```` ```bash ```` blocks anywhere in this
crate's corpus — the other eight named here (five in `integration/001`, one
in this instance, one in `pitfall/001`'s Trap section, two in
`workaround/readme.md`) have all been re-fenced ```` ```sh ```` with genuine
`Live output:` as part of the same pass.

### FL36 — Both Documents Blocking C2 Cite an Allowlist That Was Deleted for Being Unfailable

The list this instance reasons from, and the list that exists:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what this instance says G6 confines --'
awk '/^### FL/{ exit } /four declared|^crates, each justifying/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md
echo '  -- and what the proxy test it produced says --'
awk -v n1="$( command grep -n -m1 -F '//! is an unsafe trait, the workspace sets `unsafe-code = "deny"`, and gate G6' ring_flush/tests/append_cost_test.rs | cut -d: -f1 )" -v n2="$( command grep -n -m1 -F '//! `docs/workaround/readme.md`.' ring_flush/tests/append_cost_test.rs | cut -d: -f1 )" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_flush/tests/append_cost_test.rs
echo '  -- the allowlist G6 actually reads --'
printf '    path: %s\n' "$( command grep -n 'decl=' bench_harness/gate/g6_unsafe.sh | head -1 )"
printf '    entries: %s\n' \
  "$( command grep -vE '^\s*#|^\s*$' bench_harness/gate/declared/ring/unsafe_allowlist.txt | tr '\n' ' ' )"
echo '  -- why the four names went --'
command grep -E "121's four names|cannot fail" bench_harness/gate/declared/ring/unsafe_allowlist.txt \
  | sed -E 's/^(.{0,116}).*/    \1/'
```

Live output:

```
  -- what this instance says G6 confines --
    94: `unsafe-code = "deny"`; and gate G6 confines the opt-out to four declared
    95: crates, each justifying it in `docs/workaround/readme.md`. `ring_flush` is not
  -- and what the proxy test it produced says --
    8: //! is an unsafe trait, the workspace sets `unsafe-code = "deny"`, and gate G6
    9: //! confines the opt-out to three declared crates — `ring_spsc`, `ring_mpsc`,
    10: //! `ring_core` — each justifying it in its own
    11: //! `docs/workaround/readme.md`.
  -- the allowlist G6 actually reads --
    path: 21:decl="$DECL/unsafe_allowlist.txt"
    entries: ring_spsc ring_mpsc ring_core 
  -- why the four names went --
    # replaced decision/121's four names with these three. Each crate here must
    # Decision/121's four names — ring_align, ring_atomic, ring_store, ring_slot —
    # set of crates actually using unsafe is a gate that cannot fail.
```

The instance argues that C2 is blocked because a counting allocator needs
`unsafe`, the workspace denies it, and G6 confines the opt-out to a list this
crate is not on. **The conclusion is right and the list is the superseded
one** — a later ruling replaced the earlier four-name allowlist with the
current three, and none of the four the instance's own proxy test names survives.
`ring_flush` is not on the new list either, so nothing about the reasoning's
outcome changes; what changes is that the argument no longer describes the gate
it names.

**The reason the four were removed is the part that should not be lost.** A
check of all four found no `unsafe` in any of them — the allowlist header states
the principle outright: "An allowlist longer than the set of crates actually
using unsafe is a gate that cannot fail." Four names were removed precisely
because listing an inert crate makes the confinement vacuous, which is the same
defect [`nfr/001`](001_three_triggers_proven_by_a_flush_log.md)'s FL34 records
about M4 in the next file over. The family found it, ruled on it, and shrank the
list; this instance and its test still quote the pre-ruling version.

**The test is the worse of the two sites.** `append_cost_test.rs` exists *because*
of this blockage — it is the safe proxy written when the real measurement was
ruled unreachable — and its module documentation justifies its own existence by
citing a superseded ruling. A reader checking whether the proxy is still
necessary is sent to a list that no longer contains any of the names given.
