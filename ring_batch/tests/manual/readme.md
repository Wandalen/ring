# ring_batch manual testing plan

`tests/batch_test.rs` asserts that a claim of 64 costs one operation, using
`ring_atomic::CountingSeq`. That assertion is only as good as the shim, so this
plan does what the shim cannot: **read the claim path and count the atomic
operations by eye**, and check that the two refusals a caller must tell apart
are documented as different kinds rather than only returning different
variants.

Run from the workspace root.

## M1. `claim` performs exactly one atomic operation, visibly

```bash
sed -n '/^pub fn claim</,/^}/p' ring_batch/src/lib.rs
```

**Expected:** a single-expression body containing one `fetch_add` and no loop,
no retry, no second cursor read. The single operation should be apparent from
the body alone, not only from the counting assertion.

## M2. `claim_gated` adds loads, not a second advance

```bash
sed -n '/^pub fn claim_gated</,/^}/p' ring_batch/src/lib.rs \
  | grep -nE "producer\.|consumer\.|claim\("
```

**Expected:** two `load` calls (producer position, consumer barrier) and exactly
one call to `claim`. Two advances would double-claim; a `fetch_add` used as the
gate read would advance the cursor on a refused claim.

## M3. The gating loads' fixed ordering is explained

M2 shows the two gating loads hardcode `Ordering::Acquire` while `order` governs
only the advance. A caller reading the signature sees one ordering parameter and
may reasonably assume it governs everything the function does.

```bash
grep -n -B 4 -A 12 "The gated form" ring_batch/src/lib.rs
```

**Expected:** the doc states that `order` governs the advance only, that the
gating reads are always `Acquire`, and *why* that is not a caller's choice. A
`Relaxed` barrier read would let a producer act on a stale consumer position and
overwrite a slot still in use.

## M4. The two refusals are documented as different kinds, not just different variants

```bash
grep -n -A 12 "^/// # Errors" ring_batch/src/lib.rs
```

**Expected:** `BatchTooLarge` is named a configuration error the caller must not
retry, `Full` is named back-pressure the caller should retry, and the doc says
what goes wrong if they are collapsed: a retry loop spinning forever on an
impossible request.

## M5. The fold is `ring_index`'s and not a local copy

Two commands, because one pattern cannot say both "no local fold" and "the real
fold is called". An empty result would be ambiguous between the two.

```bash
# a fold of its own — must produce nothing
grep -nE "% *capacity|% *cap|& *[a-z_]*mask|\.mask\(\)" ring_batch/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
# ring_index's fold, imported and actually called — must produce both
grep -nE "use ring_index|[^a-z_]of\( " ring_batch/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** the first is empty, the second shows the import *and* a call site.
A second implementation of the fold is the duplication `ring_index` exists to
prevent; an import with no call would mean the fold is happening somewhere the
first command did not look.

## M6. The doc examples are the API's first reader

```bash
cargo test -p ring_batch --doc
```

**Expected:** every example passes; the `claim` example shows the operation
count, since that is the feature.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | One line: `BatchClaim::new( cursor.fetch_add( count as u64, order ), count )`. One operation, visible without the shim. |
| 2026-08-28 | M2 | ✅ | Two `load` calls and one `claim(...)`, in that order. |
| 2026-08-28 | M3 | ✅ *(after fix)* | The first run found the gap: the doc said "the barrier read is a load, not a fence on the claim path" and never said the load's ordering is fixed at `Acquire` regardless of `order`. Doc extended to state it and to say why it is not a caller's choice. |
| 2026-08-28 | M4 | ✅ | Both variants documented with their kind and the consequence of collapsing them. |
| 2026-08-28 | M5 | ✅ *(check corrected)* | First command empty, so no local fold. Second shows `use ring_index::of;` at line 31 and the call at line 313 in `drain_order`. |
| 2026-08-28 | M6 | ✅ | 10 doc tests pass. |

M3 is the check that earned this plan. Nothing in the test suite could have
found it. `claim_gated` behaves correctly with the hardcoded `Acquire`, so
every assertion passes. What was wrong was the *contract*: a signature offering
one ordering parameter, silently applying it to one of the three atomic
operations it performs.

M5 was drafted as one combined pattern and found only the import, because the
call site reads `of( seq, capacity )` with no qualifying prefix. It was split
into two commands with opposite expected outcomes. The finding was in the
check, not the crate.
