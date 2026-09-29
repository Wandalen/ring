# Type: Two Derives, and the Ones That Are Absent

### Scope

- **Purpose**: Account for `Publisher`'s trait surface — the two traits it derives, the several it cannot, and the two it gets without asking.
- **Responsibility**: Show why each absent trait is absent for a reason one level down, place the two derives against the family's 89, and record which guarantees are held by composition rather than by declaration.
- **In Scope**: `#[ derive( Debug, Default ) ]` at `src/lib.rs:84`, and every trait not on that line.
- **Out of Scope**: The concrete types in the signatures — see [`type/001`](001_a_seq_a_usize_and_the_one_cast.md).

### What Is on the Line

```rust
#[ derive( Debug, Default ) ]
pub struct Publisher
{
  cursor : PaddedCursor,
}
```

Two traits, and each earns its place differently:

| Trait | Why | Could it be hand-written? |
|-------|-----|---------------------------|
| `Debug` | required in practice — `[workspace.lints.rust]` sets `missing_debug_implementations = "warn"` | yes, and worse |
| `Default` | `new()` is `Self::default()` (`src/lib.rs:99-103`), so the derive **is** the constructor | yes, identically |

`Default` deriving down to `PaddedCursor::default()` — itself
`#[ derive( Debug, Default ) ]` at `ring_cursor:142` — is what makes "a publisher
with nothing published" mean `Seq::ZERO` without this crate writing the zero
anywhere. The starting frontier is a fact about `AtomicSeq`, three types down,
and `Publisher` never restates it.

That is also why `new()` and `default()` cannot drift: there is one expression,
and `tests/publish_test.rs:36-43` asserts the two agree anyway
([`item/002`](../item/002_the_two_publications.md)).

### PB41 — 88 of the Family's 89 Derives Include `Debug`; Only Three Are Exactly `Debug, Default`

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rhcE '^#\[ derive\(' ring_*/src/*.rs | paste -sd+ | bc          # 89
grep -rhE  '^#\[ derive\(' ring_*/src/*.rs | grep -c Debug            # 88
grep -rE '^#\[ derive\( Debug, Default \) \]' ring_*/src/*.rs       # 3
```

Live output:

```
89
88
ring_cursor/src/lib.rs:#[ derive( Debug, Default ) ]
ring_publish/src/lib.rs:#[ derive( Debug, Default ) ]
ring_stats/src/lib.rs:#[ derive( Debug, Default ) ]
```

Eighty-nine derive attributes across the 33 crates, eighty-eight of them
including `Debug`; the distribution is still strikingly bimodal:

| Trait | On | Share |
|-------|---:|------:|
| `Debug` | 88 | 99% |
| `Clone` | 47 | 53% |
| `PartialEq` / `Eq` | 42 / 42 | 47% |
| `Copy` | 37 | 42% |
| `Hash` | 13 | 15% |
| `Default` | 11 | 12% |
| `PartialOrd` / `Ord` | 4 / 4 | 4% |

`Debug` at 99% is still driven by the lint, not a convention anyone had to
remember — a struct without it warns. The one exception proves that:
`ring_slot`'s `BytesSlot` derives only `Clone` (`ring_slot/src/lib.rs:236`)
because its `Debug` and `PartialEq` are hand-written to read only the first
`len` bytes rather than the full backing array — a derived pair would print
and compare the uninitialized residue past `len` that `clear()` deliberately
leaves in place. The lint is satisfied by hand instead of by derive; `Debug`
still holds. Thirty-nine of the 89 are `#[ derive( Debug ) ]` and nothing
else, which is what a type holding a lock, an atomic, or a borrow can offer.

The exact list `Publisher` uses appears three times in the whole family:

| Type | Crate | Holds |
|------|-------|-------|
| `PaddedCursor` | `ring_cursor` | `CacheAligned< AtomicSeq >` |
| **`Publisher`** | **`ring_publish`** | **one `PaddedCursor`** |
| `RingStats` | `ring_stats` | `AtomicU64` counters |

Two of the three are this crate and the type it wraps — the derive list is
inherited rather than chosen, which is the same relationship
[`data_structure/001`](../data_structure/001_one_padded_cursor_and_nothing_else.md)
records for the 64-byte layout. `Publisher` adds no state to `PaddedCursor` and
correspondingly adds no capability.

### PB42 — Every Absent Trait Is Absent Because of `AtomicSeq`, Not Because of a Decision Here

None of `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd` or `Ord` is on
`Publisher`, and in every case the reason is the same and lives three types down:

```
Publisher → PaddedCursor → CacheAligned< AtomicSeq > → AtomicU64
   :87          :143              ring_atomic:166       ring_atomic:65/67
```

`AtomicU64` implements none of them — and under `--cfg loom` it is not even the
same `AtomicU64`, since `ring_atomic:64-67` swaps `core`'s for `loom`'s. Neither
one implements any of the seven, so the absences hold identically on both sides
of that seam ([`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)).

| Trait | Why absent | If it were wanted |
|-------|------------|-------------------|
| `Clone` | `AtomicU64` is not `Clone` | would need a manual impl reading the value — and two publishers over one frontier is exactly the bug |
| `Copy` | requires `Clone` | same, worse |
| `PartialEq` / `Eq` | `AtomicU64` is not `PartialEq` | would need a load, so equality would be a *timestamped* comparison, not a property |
| `Hash` | requires `Eq` to be meaningful | a hash of a value that changes under you |
| `PartialOrd` / `Ord` | ordering two publishers is not a question anyone asks | — |

The `Clone` row is worth dwelling on. A derived `Clone` would be impossible, but a
*manual* one would compile — and it would produce a second `Publisher` with a
snapshot of the frontier, so two producers publishing through the two clones
would each advance a private cursor and the consumer would see one of them. That
is a fresh way to reach
[`pitfall/002`](../pitfall/002_conflating_the_two_cursors.md)'s corruption, and
the reason it is unreachable is that nobody wrote the impl — not that anything
prevents it.

The `PartialEq` row generalises the point: for a type whose whole content is an
atomic, every value-level trait is either impossible or a lie about a moving
target. The absences are not restraint; they are the shape of the field.

### The Two Traits Nobody Declared

`Publisher` is `Send` and `Sync`, and neither appears in the source:

| Trait | How | Depends on |
|-------|-----|------------|
| `Send` | auto | `AtomicU64: Send` |
| `Sync` | auto | `AtomicU64: Sync` |

`Sync` is the load-bearing one — `publish` takes `&self`, so every producer holds
a shared reference to one `Publisher` across threads, and that is only sound
because `Sync` holds. It is derived from composition and asserted nowhere.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE 'unsafe impl.*(Send|Sync)' ring_*/src/*.rs
grep -r 'allow( unsafe_code )' ring_*/src/*.rs
```

Live output:

```
ring_mpsc/src/lib.rs://! is the fact [`Ring`]'s `unsafe impl Sync` argument rests on
ring_mpsc/src/lib.rs:unsafe impl< S : Send > Sync for Ring< S > {}
ring_spsc/src/lib.rs:unsafe impl< S : Send > Sync for Ring< S > {}
ring_mpsc/src/lib.rs:#![ allow( unsafe_code ) ]
ring_spsc/src/lib.rs:#![ allow( unsafe_code ) ]
```

Two `unsafe impl … Sync` exist family-wide — `ring_mpsc:353` and
`ring_spsc:282`, both `unsafe impl< S : Send > Sync for Ring< S > {}` — and they
are the only two crates that opt out of the workspace's `unsafe-code = "deny"`
with `#![ allow( unsafe_code ) ]`. They need the impl because a `Ring` holds
`UnsafeCell< Buffer< S > >`, which is not `Sync`, so composition gives them
nothing and the guarantee has to be asserted and argued — `ring_mpsc:340-352` is
a thirteen-line `// SAFETY:` block for one line of `impl`.

`ring_publish` needs no such argument, cannot write one — `unsafe-code = "deny"`
applies to it with no opt-out — and gets the stronger guarantee for free, because
it holds no cell and no raw pointer. The same two crates keep appearing on the
other side of every one of this crate's ledgers: they hold the `Drop` guards
([`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md) § PB36),
they built stamps instead of a published cursor
([`pitfall/002`](../pitfall/002_conflating_the_two_cursors.md) § PB38), and they
are the two that route around this crate entirely
([`integration/002`](../integration/002_the_two_crates_that_declined.md)).

That is the trade stated once: `Publisher` is safe by composition and useless
without a ring; `Ring` owns its slots and pays for them in `unsafe`.

### What Is Not Implemented, and Would Not Be Free

| Trait | Cost if added |
|-------|---------------|
| `Drop` | needs a ring to publish into — the shape [`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md) § PB36 shows this crate cannot hold |
| `Display` | a frontier printed without its ring is a bare integer; `Debug` already gives that |
| `Iterator` | over what? `is_published` answers per-sequence; ranges are `ring_consume`'s |
| `From< Seq >` | a publisher starting mid-history has no meaning — publication begins at zero or resumes a ring this type does not know about |

None is a gap. Each names a capability that would require the crate to know about
something it deliberately does not — which is the same boundary
[`decisions/001`](../decisions/001_refused_rather_than_reordered.md) draws for the
algorithm, applied to the trait surface.

### Where the Trait Surface Is Checked

| Check | What it establishes |
|-------|--------------------|
| `#![ deny( missing_docs ) ]` at `src/lib.rs:55` | a new public trait impl cannot land undocumented |
| `missing_debug_implementations = "warn"` | `Debug` cannot be dropped silently |
| `unsafe-code = "deny"` (inherited, no opt-out) | no `unsafe impl` can be added here without also editing the crate's attributes |
| `tests/publish_test.rs:36-43` | `new()` and `default()` agree |
| `tests/handshake_test.rs` compiling at all | `Sync` holds — four threads share one `&Publisher` |

The last is the only check on `Sync`, and it is a compile-time one obtained as a
side effect: no test asserts `Publisher: Sync`, but the handshake tests could not
build if it did not.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | The field every absence follows from |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | The boundary the trait surface mirrors |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | The two crates that pay in `unsafe` for what this one gets by composition |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_publications.md](../item/002_the_two_publications.md) | `new` and `default`, and the test that they agree |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | Why `Drop` is unavailable here |
| [../pitfall/002_conflating_the_two_cursors.md](../pitfall/002_conflating_the_two_cursors.md) | The corruption a hand-written `Clone` would reach |

### Types

| File | Relationship |
|------|--------------|
| [001_a_seq_a_usize_and_the_one_cast.md](001_a_seq_a_usize_and_the_one_cast.md) | The concrete types these traits would apply to |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:84-88,99-103` | The derive, and the constructor it supplies |
| `ring_cursor/src/lib.rs:143-144` | The same derive list, one level down |
| `ring_mpsc/src/lib.rs:191, 340-353` | The opt-out and the `Sync` argument this crate does not need |
| `ring_spsc/src/lib.rs:169, 269` | The same, for the single-producer design |
| `ring_atomic/src/lib.rs:32,64-67,35-36` | `AtomicSeq`, the loom swap under it, and why no `unsafe` is needed |
| `Cargo.toml` | `missing_debug_implementations = "warn"`, `unsafe-code = "deny"` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:36-43` | `new` and `default` agreeing |
| `tests/handshake_test.rs:497-561` | Three threads over one `&Publisher` — `Sync`, checked by the compiler |
