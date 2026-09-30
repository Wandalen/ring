# workaround

External constraints `ring_publish` absorbs on behalf of its consumers, each with
the cost it imposes and the condition under which it can be deleted.

Both entries come from the same source: `tests/handshake_test.rs` is feature
170's reached-test, and it is the whole four-operation handshake rather than this
crate alone. That one decision adds a `[target.'cfg(loom)']` block and four
dev-dependencies to a manifest whose library needs two crates — and both
additions look, to a reader scanning the manifest, like something worse than they
are.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The `loom` Seam and Its Only User](001_the_loom_seam_and_its_only_user.md) | PB43, PB44 — the four-line atomic switch, the six crates that touch it, and the two documents that still say three |
| 002 | [Four Dev-Dependencies That Look Like a Cycle](002_four_dev_dependencies_that_look_like_a_cycle.md) | PB45, PB46 — zero reverse edges anywhere in the workspace, both dependency closures, and the family's three scaffolding corrections |

### What Each Costs and What Would Delete It

| | The loom seam (001) | The dev-dependencies (002) |
|--|---------------------|----------------------------|
| Absorbed from | `loom`'s atomics cannot be obtained any other way | the reached-test's criterion names four operations |
| Where it lives | `ring_atomic:64-67`, four lines | this manifest, `[dev-dependencies]`, four lines |
| Cost to this crate's API | `Publisher::new` cannot be `const` | — |
| Cost to a build | none — an ordinary `cargo build` never resolves `loom` | test closure doubles, 5 crates → 10 |
| Cost to a reader | the seam is invisible from this crate's source | four edges that read like a cycle until the comment is read |
| Guarded by | `§ P6` | `§ P5` |
| Deleted by | a memory model in `core` — i.e. never | moving the reached-test to its own crate |

Neither is deletable in practice, which is what makes them workarounds rather
than debts. The honest position on both is the same: the arrangement is chosen,
argued at the point it appears in the manifest, and watched by a manual check
that runs when someone runs it.

### The Manifest, All Three Blocks

```toml
[dependencies]                              # 2 — what the library needs
ring_types, ring_cursor

[dev-dependencies]                          # 4 — what the reached-test needs      → 002
ring_claim, ring_consume, ring_barrier, ring_gating

[target.'cfg(loom)'.dev-dependencies]       # 1 — what the exhaustive half needs   → 001
loom = "0.7"
```

Seven declarations, of which two are the crate. That ratio is the subject of both
files, and `Cargo.toml:12-14` and `:21` each carry a comment explaining their own
block — which is why the arrangement survives review, and why the two stale
sentences recorded in 001 are notable: the manifest documents itself correctly,
and the *family-level* documents about it drifted.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# every crate that touches the seam, in source, in tests, and in manifests
grep -r 'cfg( *loom' ring_*/src/*.rs ring_*/tests/*.rs
grep -rl 'loom' ring_*/Cargo.toml

# the two sentences that name this crate as the seam's only user
# anchored on the entry's own comment, not a line number: the members list
# above it grows with every crate added, and a fixed offset then quotes
# whichever member comment moved into its place
sed -n '/^# .loom. is set by RUSTFLAGS/,/^unexpected_cfgs/p' Cargo.toml
command grep -m1 -A1 -F '//! `ring_publish/tests/handshake_test.rs` is what uses it, and is run with' ring_atomic/src/lib.rs

# every mention of ring_publish in any manifest in the workspace
grep -r 'ring_publish' */Cargo.toml

# how many crates carry a dependency-usage check like § P5
grep -rl 'every declared dependency' ring_*/tests/manual/readme.md | wc -l
ls -d ring_*/tests/manual/readme.md | wc -l
```

Live output:

```
ring_atomic/src/lib.rs:#[cfg(loom)]
ring_atomic/src/lib.rs:    #[cfg(loom)]
ring_atomic/src/lib.rs:    #[cfg(loom)]
ring_cursor/src/lib.rs:    #[cfg(loom)]
ring_cursor/src/lib.rs:    #[cfg(loom)]
ring_testkit/src/lib.rs://! and it is already a `cfg(loom)` dev-dependency of `ring_atomic`,
ring_mpsc/tests/mpsc_test.rs:#[cfg(loom)]
ring_publish/tests/handshake_test.rs:#[cfg(loom)]
ring_publish/tests/publish_test.rs:// `#![ cfg( loom ) ]` its `exhaustive` module carries. `--cfg loom` swaps
ring_spsc/tests/spsc_test.rs://! simply narrow enough that sampling it 100 000 times does not open it. The `#[ cfg( loom ) ] mod exhaustive` at the bottom
ring_spsc/tests/spsc_test.rs:#[cfg(loom)]
ring_testkit/tests/exhaustive_test.rs://! **The whole file is `cfg( loom )`.** Under `--cfg loom`, `ring_atomic`
ring_testkit/tests/exhaustive_test.rs:#![cfg(loom)]
ring_testkit/tests/testkit_test.rs:// The inverse of `exhaustive_test.rs`'s `#![ cfg( loom ) ]`. `--cfg loom` swaps
ring_atomic/Cargo.toml
ring_mpsc/Cargo.toml
ring_publish/Cargo.toml
ring_spsc/Cargo.toml
ring_testkit/Cargo.toml
# `loom` is set by RUSTFLAGS, not by any feature, so rustc has no other way to
# learn it is a real cfg. Declared once here rather than per crate: the lints
# table is inherited workspace-wide, and a crate cannot both inherit it and add
# its own. Only ring_atomic, ring_cursor and ring_publish read the cfg — see
# ring_atomic's module documentation on the seam.
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(loom)'] }
//! `ring_publish/tests/handshake_test.rs` is what uses it, and is run with
//! `RUSTFLAGS="--cfg loom" cargo test -p ring_publish --test handshake_test`.
ring_mpsc/Cargo.toml:# Eight, not the seven this manifest was scaffolded with. `ring_publish` and
ring_publish/Cargo.toml:name = "ring_publish"
9
33
```

| | Value |
|--|------:|
| Crates reading `cfg(loom)` in `src/` | 2 — `ring_atomic`, `ring_cursor` |
| Crates reading it in `tests/` | 4 — `ring_publish`, `ring_mpsc`, `ring_spsc`, `ring_testkit` |
| Crates declaring `loom` | 5 |
| …as a real `[dependencies]` entry | 1 — `ring_atomic` |
| Crates the two documents say read the cfg | **3** |
| `cfg(loom)` sites in `ring_publish/src/` | **0** |
| Manifests in the workspace declaring `ring_publish` | **0** |
| Manifests mentioning it at all | 1 — `ring_mpsc`, recording its removal |
| Library dependency closure | 5 of 33 |
| Test dependency closure | 10 of 33 |
| Dev-dependencies declared | 4 |
| …strictly required for resolution | 2 |
| …required to write `use` | 4 |
| Manual plans in the family | 33 |
| …carrying a dependency-usage check | 9 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB43 | `ring_atomic` | **misleading doc** | Two documents name this crate as the seam's only user and both are stale by three crates: `Cargo.toml:226` lists three readers where six touch it, and `ring_atomic:59-60` names `handshake_test.rs` as *"what uses it"* where four loom models now exist |
| PB44 | family | n/a — unenforced | `§ P6` checks two manifests and five declare `loom`, so the property *"loom never reaches a shipped build"* is established family-wide by nothing; widening the check would be the wrong fix |
| PB45 | family | n/a — observation | Not one manifest in the workspace declares `ring_publish`; the only manifest that ever did records its removal and the reason, so the acyclicity claim is a present fact with zero candidates rather than a promise |
| PB46 | family | n/a — observation | Three manifests were scaffolded before their implementations and corrected after; two of the three corrections remove `ring_seqno`, for the same reason each time |
