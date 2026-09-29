# ring_event — manual testing plan

`tests/event_test.rs` drives both slot shapes through one generic body and
asserts they round-trip. That establishes the bodies *compile* for both shapes.
What it cannot establish is the stronger reading of
`docs/feature/182_typed_slot_and_bytes_slot.md` — "both round-trip through the
**identical** claim/publish/drain path" — because a function could branch
internally on a shape and still satisfy every assertion in that file.

So this plan is a source reading: **the three shared functions must not name a
slot shape at all**, and the shape-specific knowledge must live only in the
`impl` blocks, where adding a shape is additive rather than a new branch.

Run from the workspace root.

## M1 — the three shared functions have no branch and no shape

```bash
sed -n '/^pub fn publish_into/,/^}/p;/^pub fn drain_from/,/^}/p;/^pub fn recycle/,/^}/p' \
  ring_event/src/lib.rs
```

**Expected:** each body is a single delegating expression — `payload.fill(
slot )`, `slot.peek()`, `slot.clear()`. No `match`, no `if`, no `TypedSlot`, no
`BytesSlot`.

## M2 — the shape names appear only where a shape is being implemented

```bash
grep -nE "TypedSlot|BytesSlot" ring_event/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** the `use` line and the four `impl` headers (plus their `fill`
signatures), and nothing else. A shape name in a free function would mean the
path forked.

## M3 — adding a payload kind requires no change to any slot type

```bash
grep -n -B 8 "^pub trait Fill" ring_event/src/lib.rs
grep -n "^impl.*Fill<" ring_event/src/lib.rs
```

**Expected:** `Fill` is implemented **on the payload**, not on the slot — so a
new payload kind is a new `impl` in the caller's own crate and touches nothing
here. The doc should say so, since the alternative arrangement is the obvious
one and a later reader will otherwise "fix" it.

## M4 — the read half's associated type is justified, not incidental

A GAT is a real cost in readability. It has to buy something.

```bash
grep -n -B 10 "^pub trait Peek" ring_event/src/lib.rs
```

**Expected:** the documentation states what flattening `Out` to one concrete
type would cost — a copy out of the slot, which is the allocation the family
exists to avoid.

## M5 — the empty-versus-unpublished ambiguity is written down, not hidden

A zero-length byte payload is indistinguishable from an untouched slot, because
a `BytesSlot` records only a length. That is a genuine limitation and the test
suite asserts it; a caller needs to know it, in the crate, not by reading tests.

```bash
grep -rn "zero.length\|zero-length" ring_event/src ring_event/tests
```

**Expected:** the behaviour is stated somewhere a caller will meet it, and the
statement says where the distinction must be carried instead — the handshake,
not the slot.

## M6 — the doc examples are the API's first reader

```bash
cargo test -p ring_event --doc
```

**Expected:** every example passes, and the `publish_into` example shows *both*
shapes going through the same two calls — that example is the feature.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | Three single-expression bodies: `payload.fill( slot )`, `slot.peek()`, `slot.clear()`. No branch of any kind. |
| 2026-08-28 | M2 | ✅ | 7 hits: the `use` at line 32, and the `impl`/signature pairs at 62/64, 71/73, 107, 117. None in a free function. |
| 2026-08-28 | M3 | ✅ | `impl< T > Fill< TypedSlot< T > > for T` and `impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]` — implemented on the payload. The trait doc states the reason. |
| 2026-08-28 | M4 | ✅ | Module doc's "Why the read half is a GAT" section names the copy that flattening would cost. |
| 2026-08-28 | M5 | ✅ *(after fix)* | First run found 3 hits, **all in `tests/`** and none in `src/` — the limitation was asserted but never stated where a caller reads. `Peek::peek` gained an "A `BytesSlot` cannot distinguish empty from zero-length" section naming the handshake as where the distinction lives and `TypedSlot<()>` as the cheaper signal. |
| 2026-08-28 | M6 | ✅ | 5 doc tests pass; `publish_into`'s example drives both shapes through the same two calls. |

M5 is the check that earned this plan. Both halves of the ambiguity were known —
the test file names it, asserts it, and explains it — but a limitation recorded
only in a test is a limitation the caller finds by hitting it. The grep's answer
("3 hits, all under `tests/`") is exactly the shape of that failure.

M2 was first drafted as a *count* of shape names appearing after the shared
functions begin, and reported 4 — reading as a violation. All four were inside
those functions' own doc examples, which necessarily name a concrete shape in
order to demonstrate anything. Counting is the wrong instrument here; listing
the hits with their line kind is the right one, and is what the command above
now does.
