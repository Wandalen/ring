# ring_publish manual testing plan

`tests/publish_test.rs` covers publication, and `tests/handshake_test.rs` runs
the whole claim → publish → available → commit handshake as two test suites that
check different things (see that file's own header on why neither subsumes the
other).

The gap automation leaves here is the memory ordering. Every assertion in the
threaded suite passes on x86 with `PUBLISH` weakened to `Relaxed`, because the
hardware supplies the ordering the code failed to ask for. So the suite that
looks like it covers publication does not cover the one thing publication is
*for*. The `loom` model closes that gap, and P1 records that it does.

Run from the workspace root. The code-line filter is `^[[:space:]]*//`, which
drops `///`, `//!` and plain `//` alike. This crate's module documentation argues
at length about `Relaxed`, bitmaps and `WaitKind`, and a filter keeping ordinary
comments would report those rejected designs as implementations.

## P1. The loom model detects the bugs it is written against

**This is a mutation check: it edits a file, runs the model, and restores.**
Take copies first; the restore is the whole procedure, not a formality.

A passing concurrency model proves nothing until someone deliberately breaks the
implementation and watches the model fail. Two mutations, because the model is
written against two distinct failure modes. One is in the library; the other is
in the ordering of the producer's own steps.

```bash
cp ring_publish/src/lib.rs /tmp/-publish_orig.rs
cp ring_publish/tests/handshake_test.rs /tmp/-handshake_orig.rs

# Mutation 1 — the library asks for a weaker ordering than it needs.
sed -i 's/Ordering::Release;$/Ordering::Relaxed;/' ring_publish/src/lib.rs
RUSTFLAGS="--cfg loom" cargo test -p ring_publish --test handshake_test
cp /tmp/-publish_orig.rs ring_publish/src/lib.rs

# Mutation 2 — the producer publishes the slot before writing it.
# In `mod exhaustive`, swap the two lines:
#   slot.store( WRITTEN, Ordering::Release );
#   publisher.publish( claim.start(), claim.len() );
RUSTFLAGS="--cfg loom" cargo test -p ring_publish --test handshake_test
cp /tmp/-handshake_orig.rs ring_publish/tests/handshake_test.rs

RUSTFLAGS="--cfg loom" cargo test -p ring_publish --test handshake_test
cargo nextest run -p ring_publish --all-features
```

**Expected:** both mutations fail
`a_claimed_slot_is_invisible_until_published_over_every_interleaving` at the
`available offered a slot the producer had claimed but not written` assertion,
with `left: 0`. That is the slot's initial value, meaning `available` handed the
consumer a sequence whose payload had not yet become visible to it. Then all
pass after the restore.

The second half of this check is the part worth keeping. **Run mutation 1
without `--cfg loom`.** Every threaded test still passes. That measures,
rather than asserts, the whole argument for the loom model existing. It is also
why a future edit that "simplifies away" the `--cfg loom` gate would delete the
only coverage this crate has of its central requirement.

## P2. Only compare-exchange advances the published cursor

This is the structural form of the invariant that `try_publish`'s contract
rests on. A publication that can be *assigned* rather than exchanged can move
backwards, and a published cursor moving backwards un-publishes slots a
consumer may already be reading.

```bash
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -nE "fetch_add|compare_exchange|\.store\("
```

**Expected:** exactly one hit, the `compare_exchange` in `try_publish`. No
`fetch_add` and no `store`. `publish` must not have its own advance. It is a
retry loop around `try_publish` and nothing else, so there is exactly one place
in the crate where the cursor moves.

## P3. The ordering is named once, and it is `Release`

```bash
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -nE "Ordering::|GATING|PUBLISH"
```

**Expected:** these lines and no others: the `use` importing `GATING`, the
`const PUBLISH` binding to `Ordering::Release`, `GATING` in `published()`'s
load, and the one exchange taking both (`PUBLISH` on success, `GATING` on
failure). No inline `Ordering::` at any call site. Nobody looking for why the
ring races will find an ordering chosen at the point of use.

The asymmetry in that last line is the point. A failed exchange published
nothing, so it needs no release. But it did read the cursor, and the value it
returns is what the caller retries against.

P1's mutation 1 is the behavioural form of this reading. This one is free.

## P4. Publication is refused, never reordered

The module documentation rejects both alternatives, highest-contiguous
publication and a per-slot bitmap, and defers them to `ring_mpsc`. The
rejection is only real if the code contains neither.

```bash
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -niE "bitmap|contiguous|while|for |max\(|highest"
```

**Expected:** no output. One `loop` in `publish` is the crate's only repetition,
and it retries the *same* exchange rather than scanning for a reorderable
position. Any hit here means the crate took on `ring_mpsc`'s problem, which
would make it untestable without a second producer. That is the exact reason
the problem was kept out.

## P5. Every declared dependency is used

Two sections, two consumers. One command for both would report the
`[dev-dependencies]` as unused, because the library does not use them and must
not.

```bash
# The library's own dependencies, against the library.
comm -23 \
  <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_publish/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )

# The dev-dependencies, against the tests.
comm -23 \
  <( awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f' ring_publish/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -hvE "^[[:space:]]*//" ring_publish/tests/*.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output from either. Publishing computes no distances and no
minimum, so `ring_seqno` does not belong in the library section.

The second command matters more than it looks. The dev-dependencies exist only
because `tests/handshake_test.rs` runs the whole four-operation handshake. If a
future edit narrows that test, they become the kind of dependency edge that
makes a family look more tangled than it is, and this command is what notices.

## P6. The two test suites are both wired, and mutually exclusive

`tests/handshake_test.rs` is one file containing two test suites that must never
both compile. `mod exhaustive` needs loom's atomics, `mod threaded` needs real
ones, and a build where both are live would run loom's model on real threads.

```bash
grep -nE "^#\[ cfg\( (not\( )?loom" ring_publish/tests/handshake_test.rs
grep -nE "^\[target|^loom" ring_publish/Cargo.toml ring_atomic/Cargo.toml
```

**Expected:** exactly two `cfg` gates on the test file, `cfg( loom )` on
`mod exhaustive` and `cfg( not( loom ) )` on `mod threaded`. `loom` is declared
only under `[target.'cfg(loom)']` in both manifests, as a dev-dependency here
and a real one in `ring_atomic`. An ordinary `cargo build` must never resolve
loom at all; if it appears under a plain `[dependencies]`, loom has leaked into
the shipped crate.

---

## Run Record

| Date | Checks | Result |
| ---- | ------ | ------ |
| 2026-08-28 | P1–P6 | 6/6 as expected. P1 measured both mutations failing at the slot assertion (`left: 0`), 2/2 after restore, and mutation 1 passing all 8 threaded tests un-`loom`ed |
