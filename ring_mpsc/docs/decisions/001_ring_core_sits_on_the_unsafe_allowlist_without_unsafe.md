# Decision: `ring_core` Sits on the Unsafe Allowlist Without Unsafe

**Status:** open, and narrower than it looks. The correct resolution is
probably the third, and it is a change to a declaration ruled by a
repository-level decision, not by this crate. Filed here because this crate is
the one whose opt-out the list governs, and because the mismatch was found
while checking that opt-out.

### Scope

- **Purpose**: Record that the three-name unsafe allowlist ruling this crate's opt-out contains a crate with no `unsafe` in it, which is the exact condition the previous four-name list was cut for.
- **Responsibility**: The measurement, what the repository-level ruling already decided, and the three ways to resolve the mismatch.
- **In Scope**: `gate/declared/ring/unsafe_allowlist.txt` and the `unsafe` census of the three crates it names.
- **Out of Scope**: This crate's own justification for its opt-out (→ [`../workaround/001`](../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md)).

### The Measurement

```sh
cd "$(git rev-parse --show-toplevel)"
for c in $( grep -vE '^\s*(#|$)' bench_harness/gate/declared/ring/unsafe_allowlist.txt ); do
  printf '%-12s allow attr: %d   unsafe in code: %d\n' "$c" \
    "$( grep -rcE '^\s*#!\[ *allow\( *unsafe_code *\) *\]' $c/src/lib.rs )" \
    "$( cat $c/src/*.rs | grep -vE '^\s*(//|///|//!)' | grep -c 'unsafe' )"
done
```

Live output:

```
ring_spsc    allow attr: 1   unsafe in code: 10
ring_mpsc    allow attr: 1   unsafe in code: 10
ring_core    allow attr: 0   unsafe in code: 0
```

**Two of the three named crates carry `unsafe`; `ring_core` carries none, and
does not even write the opt-out attribute.**

### What the Allowlist's Ruling Already Decided

The allowlist's own header states the principle and the precedent:

> Decision/121's four names — `ring_align`, `ring_atomic`, `ring_store`,
> `ring_slot` — were removed because a check of all four found no `unsafe` in
> any of them […] An allowlist longer than the set of crates actually using
> unsafe is a gate that cannot fail.

That is the identical test, and `ring_core` fails it. The header also gives the
reason `ring_core` is listed: it "assembles a *complete* ring: slot storage plus
the cursors that bound it", which is a statement about where unsafe *would*
belong, not about where it is.

### Three Readings

| Reading | Consequence |
|---------|-------------|
| The listing is anticipatory | `ring_core` composes this crate and `ring_spsc` rather than writing its own unsafe. The entry reserves a slot for unsafe that never arrived — and an anticipatory entry is precisely what "a gate that cannot fail" means |
| The listing is correct and the census is | If `ring_core` is expected to gain unsafe, the entry is right and the mismatch is temporary. Nothing in the ruling above says so |
| The list should be two | Remove `ring_core` and the allowlist equals the set of crates using unsafe, which is the property the earlier ruling cut four names to restore |

### MP13 — The Allowlist Names Three Crates and Two Carry Unsafe

The measurement is in the instance above. What makes it a finding rather than a
tidiness note is that the allowlist's own header states the test and applies it:
the earlier ruling's four names were cut because "a check of all four found no
`unsafe` in any of them", and the header concludes "an allowlist longer than the
set of crates actually using unsafe is a gate that cannot fail."

**Three names, two users, and the same argument that removed four.**

### MP14 — G6 Could Not Have Caught It, Because G6 Could Not Fail

```sh
cd "$(git rev-parse --show-toplevel)"
cd bench_harness/gate
printf 'crates matching the spaced spelling:   '
grep -rlE '^\s*#!\[ *allow\( *unsafe_code *\) *\]' ../../ring_*/src 2>/dev/null | wc -l
printf 'G6, ring family:                       '
GATE_FAMILY=ring bash g6_unsafe.sh 2>&1 | tail -1
```

Live output:

```
crates matching the spaced spelling:   2
G6, ring family:                       REACHED    G6 — 33 crate(s) inherit unsafe-code=deny; opt-outs confined to the 3 declared and justified
```

The house codestyle writes `#![ allow( unsafe_code ) ]` with spaces. The gate
scanned for `#!\[allow\(unsafe_code\)\]` without them, matched nothing, and
`continue`d past every crate — so it reported "opt-outs confined to the declared
and justified" having verified neither half for anybody.

**The gate anticipated exactly this failure and from the wrong direction.** Its
header warns that checking the allowlist alone would be "a conjunction over an
empty set: nothing declared, nothing found, REACHED". The emptiness arrived
through the scan instead.

The scan is now spacing-tolerant, and the gate immediately failed for a real
reason — `ring_mpsc` had no justification in its `workaround/readme.md`
(→ [`../workaround/001`](../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md)).

**Disposition:** applied — already fixed in `bench_harness/gate/g6_unsafe.sh:34-41`,
whose scan is the spacing-tolerant pattern
`'^\s*#!\[ *allow\( *unsafe_code *\) *\]'` this finding describes, with an
inline comment recording the same bug and fix narrated above. Re-ran both
recipes from this finding's own evidence block to confirm the fix still holds
(`bash g6_unsafe.sh`, 2026-09-04): output matched byte-for-byte. Now prints:
`G6, ring family:                       REACHED    G6 — 33 crate(s) inherit unsafe-code=deny; opt-outs confined to the 3 declared and justified`

### MP15 — The Justification Requirement Had Never Been Met by This Crate

Before this stage, `ring_mpsc/docs/workaround/readme.md` declared **None**, and
supported it by listing seven dependencies — of which two, `ring_publish` and
`ring_consume`, are not dependencies at all, while `ring_atomic`, `ring_slot`
and `ring_types` are and were omitted. It also described `Cargo.toml` as
"currently empty".

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -c 'path = "\.\./ring_' Cargo.toml
grep -oE 'ring_[a-z]+' Cargo.toml | sort -u | tr '\n' ' '; echo
```

Live output:

```
8
ring_atomic ring_store ring_claim ring_config ring_consume ring_cursor ring_gating ring_mpsc ring_publish ring_slot ring_types 
```

**Three wrong statements in the file the allowlist points at**, in a crate that
does carry a workaround and had not recorded it. All three were reachable by
reading `Cargo.toml`, which the file cites as its source.
