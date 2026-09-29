# non_functional_requirement

Two properties this crate has, both load-bearing for the family it belongs to,
neither stated and neither checked. It costs nothing at runtime to publish
through it rather than around it — measurably nothing, and then, in the emitted
assembly, literally nothing. And its entire dependency closure is three crates
that never touch `std` or an allocator.

Both instances follow the same arc: measure the property, find it holds, then ask
what would happen if it stopped holding. In both cases the answer is that nothing
in the workspace would notice, and in one of them the crate that exists to
measure the family says something false about it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_indirection_that_is_not_there.md) | The Indirection That Is Not There | Paired timing, the linker alias, and the benchmark that skips this path |
| [002](002_a_core_only_crate_that_does_not_say_so.md) | A Core-Only Crate That Does Not Say So | The three-crate closure, the missing declaration, and the missing target |

## Measured at One

Two independent release runs, nine paired repetitions each, two million publishes
per variant, both variants back-to-back inside every repetition: the typed ratio
lands at 1.000× and 1.002×, the byte ratio at 1.012× and 1.000×. There is no
overhead to find.

The compiler says why. Two `extern "C"` functions differing only in whether they
route through `publish_into` produce one body, and the second symbol is emitted
as an alias for the first — `through_the_crate = straight_to_the_slot`. Nothing
in the crate's documentation mentions this, in a family whose stated purpose is
avoiding allocation and copying on a hot path.

## Unguarded on Both Counts

`ring_bench` declares nine ring crates and not this one, so no benchmark
exercises the path this crate exists to provide. Its manifest explains why it had
to add `ring_slot` — "`ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are
generic over `Slot`, and `TypedSlot< T >` is the only implementor this benchmark
exercises (the other, `BytesSlot< N >`, round-trips through no path measured
here — see ring_event's own non_functional_requirement/001 § EV34)". The
coverage gap is real and unchanged — `ring_slot` declares two implementors and
the benchmark measures one — but the manifest now says so itself, and names this
crate's own instance as the reason.

**Correction (2026-09-20):** this paragraph quoted the manifest as ending at
"`TypedSlot< T >` is the only implementor" and called that second half false,
because `ring_slot` declares two. The manifest has since been corrected: it
carries the qualifier "this benchmark exercises", names `BytesSlot< N >`
explicitly, and cites EV34. The quote here was cut mid-qualifier and the charge
of falsity no longer had a target. What survives is the measurement gap, not the
documentation gap — the benchmark still exercises one of the two implementors,
which is the finding; the manifest no longer hides it, which was the charge.

The portability side is the same shape. Zero `std` or `alloc` references across
`ring_event`, `ring_slot` and `ring_types`; zero external dependencies anywhere
in the closure; 18 of the family's 33 crates at zero references by that measure,
3 of them declaring `#![ no_std ]` — one of which, `ring_types`, is inside this
closure — so 15 remain in exactly this position. No
bare-metal target is installed either, so the
compiler has never been asked the question and could not answer it if it were.

**Correction (2026-09-20):** this paragraph read "17 of the family's 33 crates in
the same position", and the EV35 row below said the same. The position described
is *zero references and no declaration*, so all three declarers come out of the
18, not just `ring_types` — the subtraction was 18−1 where it should have been
18−3. The Regenerate block below is why neither number was checked against
anything, and two failures compounded there. Four of its commands still globbed
`ring_*/`, a path the crates left, so the census printed `crates at zero:
1 of 33` and the declaration count printed `0` while the prose said 17 and 3 —
and the section recorded no output at all, so there was nothing to diff those
wrong values against. A recipe with no quoted output cannot go stale, because
nothing it prints is ever compared to anything. All four commands are retargeted
below, a Live output block is added under them, and EV36's test
denominator is corrected from sixteen to the seventeen the crate now has.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the crate says about the cost of going through it --'
command grep -m1 -A2 -F '/// Deliberately trivial. Its value is not what it does but that there is only' ring_event/src/lib.rs
echo '  -- what the family benchmark declares, and its reason for the slot crate --'
command grep '^ring_' ring_bench/Cargo.toml
command grep -m1 -A3 -F '#   ring_slot  — `ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are generic' ring_bench/Cargo.toml
echo '  -- against every Slot implementor there is --'
command grep -r 'impl.*Slot for' --include=*.rs
echo '  -- the whole dependency closure of this crate --'
for c in ring_event ring_slot ring_types; do
  printf '%-12s ' "$c"
  sed -n '/^\[dependencies\]/,/^\[/p' $c/Cargo.toml | command grep -o '^[a-z_]* =' | tr -d ' =' | tr '\n' ' '
  echo
done
echo '  -- crates with no std or alloc reference outside doc comments --'
for c in ring_*/; do
  n=$( cat "$c"src/*.rs 2>/dev/null | command grep -v '^ *//' | command grep -c 'std::\|String\|Vec<\|Vec::\|Box<\|format!\|println!\|HashMap' || true )
  printf '%-18s %s\n' "$( basename $c )" "$n"
done | sort -k2 -n | awk '$2 == 0 { z++ } END { print "  crates at zero: " z " of 33" }'
echo '  -- and how many declare no_std, against targets that could check --'
command grep -rl 'no_std' --include=*.rs ring_*/src | wc -l
rustup target list --installed
```

Live output:

```
  -- what the crate says about the cost of going through it --
/// Deliberately trivial. Its value is not what it does but that there is only
/// one of it: a ring's publish *path* passes through here — the step where a
/// claimed slot receives its payload — so no slot shape can acquire a publish
  -- what the family benchmark declares, and its reason for the slot crate --
ring_factory = { path = "../ring_factory" }
ring_tls = { path = "../ring_tls" }
ring_flush = { path = "../ring_flush" }
ring_stats = { path = "../ring_stats" }
ring_spsc = { path = "../ring_spsc" }
ring_mpsc = { path = "../ring_mpsc" }
ring_core = { path = "../ring_core" }
ring_slot = { path = "../ring_slot" }
ring_types = { path = "../ring_types" }
#   ring_slot  — `ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are generic
#                over `Slot`, and `TypedSlot< T >` is the only implementor this
#                benchmark exercises (the other, `BytesSlot< N >`, round-trips
#                through no path measured here — see ring_event's own
  -- against every Slot implementor there is --
ring_slot/src/lib.rs:impl< T > Slot for TypedSlot< T >
ring_slot/src/lib.rs:impl< const N : usize > Slot for BytesSlot< N >
  -- the whole dependency closure of this crate --
ring_event   ring_types ring_slot 
ring_slot    ring_types 
ring_types   
  -- crates with no std or alloc reference outside doc comments --
  crates at zero: 18 of 33
  -- and how many declare no_std, against targets that could check --
3
aarch64-unknown-linux-gnu
wasm32-unknown-unknown
wasm32-wasip1
x86_64-unknown-linux-musl
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV33 | `ring_event` | n/a — doc gap | Publishing through this crate costs nothing against calling the slot directly — two independent release runs of nine paired repetitions put the typed ratio at 1.000× and 1.002× and the byte ratio at 1.012× and 1.000× — and the assembly shows there is nothing to measure, because two `extern "C"` functions differing only in whether they route through `publish_into` compile to one body with the second symbol emitted as an alias, `through_the_crate = straight_to_the_slot`; the crate's only performance statement defends the read half's GAT against a copy it would have introduced, while `publish_into`'s own account is architectural ("Deliberately trivial. Its value is not what it does but that there is only one of it"), so the first question a reader of a family built to avoid allocation would ask about a crate sitting between ring and slot is answered by a linker alias and recorded in no doc, test or benchmark |
| EV34 | `ring_bench` | **wrong doc** | The family's benchmark crate declares nine ring crates — `ring_factory`, `ring_tls`, `ring_flush`, `ring_stats`, `ring_spsc`, `ring_mpsc`, `ring_core`, `ring_slot`, `ring_types` — and not `ring_event`, so nothing in the workspace measures the path this crate exists to provide and the zero-overhead property would regress unobserved; worse, the manifest comment justifying `ring_slot`'s inclusion states that "`TypedSlot< T >` is the only implementor" of `Slot`, which is false — `ring_slot` declares two, `TypedSlot< T >` at `:176` and `BytesSlot< N >` at `:385` — and the omitted one is the shape this crate exists to prove parity for, so the benchmark's model of the family has the second slot shape missing from it as a stated fact, consistent with `ring_core` fixing `TypedSlot< T >` eight times and mentioning `BytesSlot` zero |
| EV35 | `ring_event` | n/a — doc gap | The crate's entire dependency closure is `ring_event` → `ring_slot` → `ring_types`, three crates with no third-party dependency anywhere in it, and none of the three references `std` or `alloc` outside a doc comment — everything used is `core`: `Option`, `Result`, a fixed array, a `usize`, two traits and an associated type — so `#![ no_std ]` would compile today and cost one line; it is absent here and from 30 of the 33 crates in the family — `ring_types`, `ring_stats` and `ring_overflow` carry it, and `ring_types` sits inside this crate's own closure — with 18 of the 33 at zero references by the same measure and 15 of them in the same position as this crate, and nothing records whether core-only operation is a supported property or an accident of the code so far, which for the smallest and purest crate in a ring-buffer family is the question an embedded consumer asks first |
| EV36 | `ring_event` | n/a — unenforced | The core-only property rests on source inspection rather than on the compiler, because without `#![ no_std ]` the compiler always links `std` and never has cause to object, and the toolchain has no target that could ask the question — all four installed (`aarch64-unknown-linux-gnu`, `wasm32-unknown-unknown`, `wasm32-wasip1`, `x86_64-unknown-linux-musl`) ship a `std`; the suite cannot close the gap either, since its dev-dependency `ring_store` stores slots in a `Box< [ S ] >` built from a `Vec::with_capacity`, so four of the seventeen tests reach an allocator through storage that the library under test never touches, and until the declaration exists the family's most portable crate is portable by accident rather than by construction |
