# workaround

Two external constraints this crate absorbs on behalf of its consumers. Both are
real, both are absorbed the right way, and neither is written down anywhere in
the source — which is what turns a temporary accommodation into a permanent one.

The first is a toolchain constraint: `Index` and `IndexMut` are not yet
const-stable, so two functions written in the clear form cannot be `const`. The
crate's response was to keep the clear form and drop the keyword — defensible,
and indistinguishable from never having considered it, since nothing marks these
two as waiting on anything. The second is a dependency constraint: `write` needs
an error and `ring_types` has no variant for it, so it borrows one whose field
documentation and rendered message both describe a different concept entirely.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Four Functions That Could Be `const`](001_four_functions_that_could_be_const.md) | SL49, SL50 — two free to mark today, two blocked by an operator rather than by what they do |
| 002 | [`BatchTooLarge` Borrowed for a Different Shape](002_batchtoolarge_borrowed_for_a_different_shape.md) | SL51, SL52 — slot-denominated fields filled with bytes, and why borrowing is still right |

### The `const` Split Is Not a Capability Boundary

| Function | Blocked by | Cost to mark `const` |
|----------|------------|----------------------|
| `set` (`:90`) | *nothing* — `Option::replace` is const-stable | Add the keyword |
| `take` (`:117`) | *nothing* — `Option::take` is const-stable | Add the keyword |
| `write` (`:243`) | `IndexMut` not const-stable | Rewrite one expression via `split_at_mut` |
| `read` (`:263`) | `Index` not const-stable | Rewrite one expression via `split_at` |

Verified by const-evaluating all four on rustc 1.97.1, the toolchain the
workspace builds with: `set` and `take` run in a real `const` block verbatim, and
`read`/`write` run once the index operator is replaced. Six `const fn` beside
four plain `pub fn` reads like a considered boundary and is not one — for two of
the four the inference is simply wrong.

Not one doc comment on the ten functions explains a `const` choice. So when
`Index` does stabilise as a const trait, nothing in the crate will tell a
maintainer that two of these were waiting on exactly that: the constraint lifts
silently and the workaround outlives it, which is the ordinary way a workaround
becomes permanent.

### One Variant, Two Units

| Crate | `requested` | `capacity` | Matches the field docs? |
|-------|-------------|------------|:-----------------------:|
| `ring_batch:318` | slot `count` | ring capacity | ✔ |
| `ring_claim:425` | slot count | ring capacity | ✔ |
| `ring_gating:269` | slot `count` | ring capacity | ✔ |
| **`ring_slot:351`** | **`payload.len()` — bytes** | **`N` — one slot's width** | ✘ |

`RingError::BatchTooLarge` documents its fields as "Slots asked for" and "Slots
the ring has in total", and the `Display` arm renders
`batch of {requested} exceeds ring capacity {capacity}`. Nothing in the value
says which crate built it, so a twenty-byte payload refused by a
`BytesSlot< 16 >` reports `batch of 20 exceeds ring capacity 16` — naming a batch
that does not exist and a ring capacity that is not involved.

The borrow is still correct. `ring_slot` sits at Tier 1 above `ring_types` alone;
the alternatives were a crate-local error type (fragmenting the family's single
error type for one condition) or a new variant in a Tier 0 crate (a breaking
change to every exhaustive `match` on `RingError`). It also inherits
`is_configuration() == true` / `is_transient() == false`, which is right for a
compile-time-fixed `N` — more strongly right than for the ring capacity the
classification was written for.

What is missing is two sentences: one at the construction site saying the variant
is borrowed and the message will name a batch, and one on the variant's own
declaration saying its fields are slots *or* bytes depending on the constructor —
so the next person tightening `RingError` does not narrow the field docs to slots
and make this use a lie in the type system as well as the prose.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# six const, four plain — and which four
printf '  const fn : %d\n' "$( grep -c '^[[:space:]]*pub const fn ' ring_slot/src/lib.rs )"
printf '  plain fn : %d\n' "$( grep -c '^[[:space:]]*pub fn ' ring_slot/src/lib.rs )"
grep -n '^[[:space:]]*pub fn ' ring_slot/src/lib.rs

# the two index expressions that are the whole obstacle
sed -n '/^    self\.bytes\[ \.\.payload\.len() ]\.copy_from_slice( payload );$/p;/^    &self\.bytes\[ \.\.self\.len ]$/p' ring_slot/src/lib.rs

# and no comment anywhere explaining a const choice
grep -nE '^[[:space:]]*//[/!].*\bconst' ring_slot/src/lib.rs | grep -viE 'constraint|construct' \
  || echo '  no doc comment anywhere explains a const choice'

# the variant as ring_types declares it, and the message it renders
awk '/^  NameUnknown,$/{ n1 = NR } n1 && NR >= n1 + 1 && NR <= n1 + 9 { print } /^      Self::BatchTooLarge \{ requested, capacity \} =>$/{ n2 = NR } n2 && NR >= n2 && NR <= n2 + 3 { print }' ring_types/src/error.rs

# every crate that constructs it — three mean slots, one means bytes
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'RingError::BatchTooLarge' ring_*/src/*.rs | grep -vE ':[[:space:]]*//' \
  | grep -v '^ring_types/' | sort

# the classification the borrow inherits
command grep -m1 -B1 -A2 -F '      | Self::CapacityNotPowerOfTwo( _ )' ring_types/src/error.rs
```

Const-evaluation results, the two operator errors, and the rendered message all
come from probes on rustc 1.97.1; the output is quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL49 | `ring_slot` | n/a — observation | `set` and `take` are non-`const` for no reason the code contains — both bodies const-evaluate verbatim on the workspace's toolchain |
| SL50 | `ring_slot` | n/a — doc gap | `read` and `write` are blocked by `Index`/`IndexMut` not being const-stable, not by what they do; nothing marks them as waiting, so the constraint will lift silently |
| SL51 | `ring_slot` | **misleading doc** | `write` fills slot-denominated fields with byte counts, so a refused write renders `batch of 20 exceeds ring capacity 16` — a batch that does not exist and a ring capacity that is not involved |
| SL52 | `ring_types` | n/a — doc gap | The borrow is the correct call at this tier and inherits the right classification; neither the construction site nor the variant's declaration records that its fields carry two different units |
