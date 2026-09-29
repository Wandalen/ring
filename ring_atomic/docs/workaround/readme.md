# workaround

External constraints `ring_atomic` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

The crate absorbs exactly one, and absorbs it well. `loom`'s instrumented atomics
carry per-execution model state, so they have no `const` constructor and their trait
implementations are loom's own business — two facts that would otherwise leak into
every crate holding a cursor. `ring_atomic` takes both: one `cfg` switch selects the
atomic source, `new` is written twice for each of the two cells, and `Default` is
written out by hand rather than derived. Six `cfg` sites and one duplicated impl, in
exchange for four other crates running loom models against this crate's real cells
instead of a re-implementation. Both instances here are about the edges of that one
seam rather than about a second constraint.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_loom_seam_and_the_manifest_above_it.md) | The Loom Seam and the Manifest Above It | What the seam costs inside the crate, and the `check-cfg` entry it cannot contain |
| [002](002_what_the_seam_does_not_switch.md) | What the Seam Does Not Switch | The third imported name that crosses unswitched, and the suite that compiles but cannot run |

## The Part That Could Not Be Absorbed

A workaround is judged by what it keeps away from its consumers, and this one keeps
away everything except a build flag. `loom` is set by `RUSTFLAGS` rather than by a
feature, so rustc needs a `check-cfg` entry to accept the name — and Cargo does not
let a crate both inherit the workspace lints table and add to it. The entry therefore
lives in the root manifest, shared by all 33 crates, and `ring_atomic` does not build
standalone under `-D warnings`: one error per `cfg` site, six in total. That is the
precise sense in which the module documentation's "no other crate needs to know the
seam exists" is true of siblings and false of whatever builds them.

## Compiling Is Not Running

Both edges recorded here are places where the ordinary signal says fine. `Ordering` is
imported unconditionally from `core` while the atomics switch, which is correct only
because loom re-exports core's type — the loom branch compiles clean, and nothing
records that this was checked rather than assumed. And all 17 tests compile under
`--cfg loom` while every one of them panics on first atomic access, because none opens
a `loom::model`; the family's fifteen models live in four other crates. In both cases
a type-level change would be caught and a behavioural one would not.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the seam, and every site it forces here --'
command grep -c 'cfg( loom )\|cfg( not( loom ) )' ring_atomic/src/lib.rs
echo '  -- the entry it cannot contain, and the count in its own comment --'
# anchored on the entry's own comment, not a line number: the members list
# above it grows with every crate added, and a fixed offset then quotes
# whichever member comment moved into its place
sed -n '/^# .loom. is set by RUSTFLAGS/,/^unexpected_cfgs/p' Cargo.toml
echo '  -- against the crates that actually read the cfg --'
command grep -rl 'cfg( loom )' --include=*.rs . | sed 's|ring/||' | sort
echo '  -- and where the models that exercise the branch live --'
command grep -rlc 'loom::model' --include=*.rs . | sed 's|ring/||' | sort
```

Live output:

```
  -- the seam, and every site it forces here --
6
  -- the entry it cannot contain, and the count in its own comment --
# `loom` is set by RUSTFLAGS, not by any feature, so rustc has no other way to
# learn it is a real cfg. Declared once here rather than per crate: the lints
# table is inherited workspace-wide, and a crate cannot both inherit it and add
# its own. Only ring_atomic, ring_cursor and ring_publish read the cfg — see
# ring_atomic's module documentation on the seam.
unexpected_cfgs = { level = "warn", check-cfg = [ 'cfg(loom)' ] }
  -- against the crates that actually read the cfg --
ring_atomic/src/lib.rs
ring_cursor/src/lib.rs
ring_mpsc/tests/mpsc_test.rs
ring_publish/tests/handshake_test.rs
ring_publish/tests/publish_test.rs
ring_spsc/tests/spsc_test.rs
ring_testkit/tests/exhaustive_test.rs
ring_testkit/tests/testkit_test.rs
  -- and where the models that exercise the branch live --
ring_atomic/tests/atomic_test.rs
ring_barrier/tests/barrier_test.rs
ring_batch/tests/batch_test.rs
ring_bench/tests/bench_test.rs
ring_claim/tests/claim_test.rs
ring_consume/tests/consume_test.rs
ring_core/tests/core_test.rs
ring_cursor/tests/cursor_test.rs
ring_debug/tests/debug_test.rs
ring_factory/tests/factory_test.rs
ring_flush/tests/append_cost_test.rs
ring_flush/tests/flush_test.rs
ring_gating/tests/gating_test.rs
ring_handle/tests/handle_test.rs
ring_mpsc/tests/mpsc_test.rs
ring_poll/tests/poll_test.rs
ring_publish/tests/handshake_test.rs
ring_publish/tests/publish_test.rs
ring_registry/tests/registry_test.rs
ring_shutdown/tests/shutdown_test.rs
ring_spsc/tests/spsc_test.rs
ring_testkit/src/lib.rs
ring_testkit/tests/exhaustive_test.rs
ring_testkit/tests/testkit_test.rs
ring_tls/tests/tls_test.rs
ring_wait/tests/wait_test.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT49 | `ring_atomic` | n/a — observation | The crate absorbs loom's missing `const` constructors and uncertain trait impls at six `cfg` sites, two duplicated constructors and one hand-written `Default`, but cannot absorb the `check-cfg` entry — Cargo forbids a crate from both inheriting the workspace lints table and extending it — so lifted out of the workspace it fails under `-D warnings` with one error per `cfg` site, six in all |
| AT50 | root `Cargo.toml` | **wrong doc** | The `check-cfg` entry's comment says "Only ring_atomic, ring_cursor and ring_publish read the cfg" where six files across six crates do and five declare `loom` as a dependency — the word "Only" would lead a reader to think removing one crate's loom test frees the entry, and this is the fourth never-recomputed count in this crate's neighbourhood |
| AT51 | `ring_atomic` | n/a — doc gap | `AtomicU64` and `AtomicUsize` are switched by `cfg` while `Ordering` is imported unconditionally from `core` one line below, which is correct only because loom re-exports core's type rather than instrumenting its own — verified by building the loom branch clean, and recorded nowhere, so the asymmetry reads as an oversight rather than the deliberate choice it is |
| AT52 | `ring_atomic` | n/a — coverage | All 17 tests compile under `--cfg loom` and every one panics at the first atomic access, because none opens a `loom::model` and the family's fifteen live in four other crates — so `cargo build --tests` reports the loom configuration green, and a behavioural break in the seam's own branch would surface only three or more dependency edges downstream |
