# lifecycle

A slot is allocated once and reused for the ring's whole life, so its lifecycle
is two nested loops: what happens within one publish, and what survives from one
lap to the next. The crate's own justification for the fixed array — "a ring's
slots are allocated once, so a slot that could grow would defeat the allocation
behaviour the ring was chosen for" — is what makes both loops consequences of the
design rather than defects in it.

The first instance finds the family running two incompatible publish models, and
the safer of the two is the narrower one: `ring_mpsc` and `ring_spsc` drain with
`TypedSlot::take`, which empties the slot as a side effect and cannot be
forgotten, while `ring_event` drains through a borrow and needs a third call that
can. The second follows what is left behind: one shape releases a lap's payload
at `clear` and the other retains all `N` bytes until some later lap overwrites
them — a window nothing bounds and nothing documents.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Slot Across One Publish](001_a_slot_across_one_publish.md) | SL29, SL30 — a mandatory third call that fails silently, and the two-step model only one shape can use |
| 002 | [A Slot Across a Ring's Laps](002_a_slot_across_a_rings_laps.md) | SL31, SL32 — one `clear` that frees and one that forgets, and residue with no upper bound |

### Two Models for One Publish

| Model | Steps | Empties on drain? | Works for | Failure if a step is missed |
|-------|:-----:|:-----------------:|-----------|-----------------------------|
| `ring_event` — `publish_into` / `drain_from` / `recycle` | 3 | ✘ | Both shapes | Slot reads occupied forever; silent |
| `ring_mpsc`/`ring_spsc` — `push` / `get_mut` + `take` | 2 | ✔ | `TypedSlot` only | Not possible |

`Peek` returns a borrow for both shapes — `self.get()` and `self.read()`, the two
`&self` accessors — so a `ring_event` drain never mutates. The cycle is
`empty → occupied → occupied-and-read → empty` and the last transition is a call
a caller must remember: forget it and the ring keeps working, the slot keeps
reporting non-empty, and `all_empty()` reads `false` on a fully-drained ring.

`BytesSlot` cannot adopt the two-step model. Taking a `[ u8; N ]` by value would
copy `N` bytes out of a slot the ring owns and will overwrite anyway — precisely
the cost the shape exists to avoid. The difference is forced by what the payload
*is*, neither crate is wrong, and `read`'s doc never says that its sibling
`TypedSlot::take` leaves the slot empty where it does not.

### One Word, Two Guarantees

| | `TypedSlot< T >` | `BytesSlot< N >` |
|---|---|---|
| `clear` does | Drops `T` | Sets `len = 0` |
| Payload memory after `clear` | Freed | Resident, all `N` bytes |
| Observable through the API? | No — `get()` is `None` | No — `read()` is `[]` |
| Observable through `Debug`? | No | **Yes, in full** |
| Retention window | Until `clear` | Until overwritten, or the ring dies |

Measured with a drop counter, a `TypedSlot` releases its payload's heap
allocation at `clear`. Measured with `Debug`, a `BytesSlot` cleared of
`lap 1: secret456` still holds all sixteen bytes, and a five-byte lap-2 write
leaves eleven of them intact.

### Nothing Bounds the Window

`ring_store::Buffer::clear` is the one bulk reset in the family and no shipped
code calls it — only its own definition and its own test. No ring clears its
slots on shutdown and none clears them on drop; the boxed slice is freed as it
stands. So the retention window is not one lap, it is *until a longer write lands
on that exact slot, or the process ends*.

The justification is honest — per-lap zeroing costs `N` bytes of writes per
publish on the hot path, which is the cost the fixed-array shape exists to avoid.
The gap is that nothing states the window at all, while the module comment's
"reads back exactly what was written and nothing else" invites the opposite
conclusion.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the three named operations, and their three receivers
grep -n 'pub fn publish_into\|pub fn drain_from\|pub fn recycle' ring_event/src/lib.rs

# both Peek impls return a borrow, so a drain never mutates
command grep -m1 -A18 -F 'impl< T > Peek for TypedSlot< T >' ring_event/src/lib.rs

# the same word, two bodies
awk '/^    self\.0\.is_none\(\)$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 6 { print } /^    Self::is_empty\( self \)$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 6 { print }' ring_slot/src/lib.rs

# the premise the whole lifecycle rests on
command grep -m1 -A3 -F '/// The shape for traffic arriving from outside the process, decoded later by' ring_slot/src/lib.rs

# the one bulk reset, and its complete absence from shipped code
grep -rn '\.clear()' ring_*/src/*.rs | grep -vE ':[[:space:]]*///?' \
  | grep -vE 'entries|items|sink|log|slot\.clear' || echo '  none — no crate resets a whole buffer'
```

State observations — drop counts and two-lap byte retention — come from a release
probe; the figures are quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL29 | `ring_event` | **latent hazard** | A `ring_event` drain never empties the slot, so `recycle` is mandatory and separate; omitting it is neither a compile error nor a runtime error and leaves a drained ring reading non-empty |
| SL30 | `ring_slot` | n/a — doc gap | The family runs two publish models and the safer, self-clearing one works only for `TypedSlot`; `read`'s doc never notes that it borrows where `take` would empty |
| SL31 | `ring_slot` | n/a — observation | The two `clear` bodies agree on everything the API can see and disagree underneath — one drops the payload, the other moves a counter |
| SL32 | `ring_slot` | **latent hazard** | A cleared `BytesSlot` retains all `N` bytes until a longer write lands on that slot or the ring dies; no bulk reset is wired to any lifecycle event and no doc states the window |
