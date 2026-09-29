# Workaround: The Only `std` Type in a Public Signature

### Scope

- **Purpose**: Record that `drain` returned `std::vec::Drain` — the family's only `std` type in a public signature — until TL51's disposition removed it, and state what that pinned while it held.
- **Responsibility**: The constraint, its cost, and the disposition that removed it.
- **In Scope**: The former signature, `pub fn drain( &mut self ) -> std::vec::Drain< '_, T >`, and the current one, `pub fn drain( &mut self ) -> impl Iterator< Item = T > + '_`.
- **Out of Scope**: What `drain` is for (→ [`../api/002`](../api/002_consolidator_read_surface.md), which specifies a different read surface); the fused alternative (→ [`../algorithm/002`](../algorithm/002_the_fused_claim_and_drain.md)).

### The Constraint

Rust has no way to return "an iterator that empties this buffer" without either
naming the concrete iterator or reaching for `impl Trait` in return position.
`drain` named the concrete one, until TL51's disposition (below) switched it to
the second option.

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'public signatures naming a std type, whole family:\n'
grep -rE '^\s*pub (const )?fn .*std::' --include='*.rs' ring_*/src/
printf 'crates declaring no_std: '
grep -rl 'no_std' --include='*.rs' ring_*/src/ | wc -l
```

Live output:

```
public signatures naming a std type, whole family:
crates declaring no_std: 3
```

One signature in thirty-three crates used to name a `std` type — `ring_tls`'s
`drain`. TL51's disposition removed it, so the `grep -rE` line above now
matches nothing anywhere in the family: the posture the other thirty-two
crates already kept is unanimous rather than thirty-two against one.

### Cost

`Vec` was part of the public contract for as long as `drain` named it —
changing `TlsBuffer`'s storage would have been a breaking change to `drain`'s
return type even though `drain`'s *meaning* would not have changed. TL51's
disposition below removed that cost: `impl Iterator< Item = T > + '_` does not
name the storage, so the type that had leaked is no longer part of the
contract. The storage question itself is unaffected — it is still the one
[`../data_structure/002`](../data_structure/002_a_vec_and_a_limit.md) records as
the crate's central open question, just no longer pinned by this signature.

The signature also used to block `no_std` for the whole crate, which for a
per-thread staging buffer with no allocation in its steady state was a
plausible ask. `impl Iterator` carries no such restriction.

### Deletion Condition

`-> impl Iterator< Item = T > + '_`. The language feature existed all along;
the reason it had not been used was never recorded anywhere in the source,
which is what made this a workaround rather than a decision — a decision would
have had a rationale. TL51's disposition below applied exactly this signature.

`ring_batch` already returned `impl Iterator< Item = Seq > + use< >` from
`BatchClaim::sequences`, in this same workstream, so the idiom was present in a
crate this one depends on before this crate adopted it too.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | The signature |
| `../../../ring_batch/src/lib.rs` | Returns an opaque iterator for the same class of thing |

### TL50 — One Signature in Thirty-Three Crates Names a `std` Type

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '^\\s*pub (const )?fn .*std::' --include='*.rs' ring_*/src/
printf 'crates declaring no_std: %s\n' \
  "$( grep -rl 'no_std' --include='*.rs' ring_*/src/ | wc -l )"
```

Live output:

```
crates declaring no_std: 3
```

Three crates declare `no_std` — `ring_overflow`, `ring_stats` and `ring_types` —
and nothing broke, because none of them can reach this signature. They form a
closed chain: `ring_overflow` depends on `ring_stats` and `ring_types`,
`ring_stats` on `ring_types`, and `ring_types` on nothing. `ring_tls`'s own
dependents are `ring_bench`, `ring_factory`, `ring_flush` and `ring_testkit`, so
the leaked `std` type never enters a `no_std` closure. The other thirty-two
crates kept the posture anyway, which is what made this one a deviation rather
than a non-issue — TL51's disposition below closed it.

**Correction (2026-09-20):** this paragraph read "No crate declares `no_std`, so
nothing broke" while the recipe two lines above it printed `crates declaring
no_std: 3`. The conclusion is unchanged but its reason is not: "nothing broke"
was recorded as vacuous — no declaring crate existed to break — and is now a
dependency-graph fact, true only while no declaring crate depends on `ring_tls`.
Should one ever do so, this signature becomes a real breakage and not a
deviation.

### TL51 — The Leaked Type Is the One the Crate May Have to Change

The storage question is
[`../data_structure/002`](../data_structure/002_a_vec_and_a_limit.md)'s open
one, and reading 3 of
[`../decisions/001`](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md)
would change it. The signature that leaked is the signature over the thing most
likely to move.

`-> impl Iterator< Item = T > + '_` closes it. `ring_batch::sequences` already
returns an opaque iterator, in a crate this one depends on, so the idiom is one
import away and no rationale for skipping it is recorded anywhere.

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pub fn drain' ring_tls/src/lib.rs
```

Live output:

```
  pub fn drain( &mut self ) -> impl Iterator< Item = T > + '_
```

**Disposition:** applied — `TlsBuffer::drain`'s return type in
`ring_tls/src/lib.rs` no longer names `std::vec::Drain`; it returns
`impl Iterator< Item = T > + '_`, matching `ring_batch::sequences`'s idiom.
Purely a signature change — the body still calls `self.items.drain( .. )`,
and no caller in the workspace named the concrete type (`ring_testkit`,
`ring_bench`, `ring_flush` and `smoke_ring_write_path` all just call
`.drain()` and chain `.collect()` or iterate), so nothing else moved.
Verified via `cargo test -p ring_tls --all-features`, 2026-09-04 — ring_tls's
22 unit tests plus 7 doctests all pass.
Now prints: `  pub fn drain( &mut self ) -> impl Iterator< Item = T > + '_`
