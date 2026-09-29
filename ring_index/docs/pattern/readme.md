# pattern

Two patterns govern this crate, and they operate at different levels. The first
is a rule about *where* one piece of arithmetic may live — one owner for the
fold, everyone else reaches it. The second is a rule about *what shape* a crate
holding only arithmetic takes — no type, no state, free functions over borrowed
values.

Both are recorded here with the same question asked of each: what enforces it. In
both cases the answer is convention rather than the compiler, and in both cases
the family's history shows what that costs. The one-owner rule has one violation
in 33 crates, documented in the violating crate's own module comment. The
stateless-shape rule has one exception among eight functions, and the family
routed around it rather than fixing it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_owner_for_one_arithmetic_fact.md) | One Owner for One Arithmetic Fact | The stated rule, its unaudited enforcement, and the narrower rule the code actually follows |
| [002](002_stateless_arithmetic_over_borrowed_types.md) | Stateless Arithmetic Over Borrowed Types | The shape `ring_index` shares with `ring_seqno`, and the one function of eight that breaks it |

## The Rule That Predicts the Violation

"One owner for the fold" is the rule everybody states; it does not predict where
the one violation happened. "The fold travels with the container" does.
`ring_mpsc` holds two ring-shaped storages in one struct — `slots` wrapped in a
`Buffer`, and `stamps` as a bare `Box< [ AtomicSeq ] >` seven lines below. The
wrapped one inherits the fold. The bare one needed a fold nothing supplied and
got a hand-written mask.

`ring_spsc` holds one storage, wrapped, and has no hand-written fold anywhere.
`ring_store` is the wrapper. `ring_batch` holds no storage and imports `of`
directly. The narrower rule sorts all four crates correctly; the stated rule
sorts none of them, because all four intend to obey it.

## Convention Where the Compiler Was Available

Both patterns could have been mechanized and neither was. `Capacity::mask` is
`pub const fn`, so any crate depending on `ring_types` can fold by hand and
compile; nothing marks `ring_index` as the sanctioned route. No test, lint, or CI
step runs the census that finds a second implementation — the census exists only
in this corpus.

The contrast that makes it worth recording is one level down. `Capacity`'s
power-of-two invariant *is* mechanized: the field is private, so breaking it is a
compile error. The one-owner rule sits directly on top of that invariant, is
stated in the same declarative register, and is enforced by nobody.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- ring_mpsc, two storages in one struct --'
sed -n '/^  slots : Buffer< UnsafeCell< S > >,$/p;/^  stamps : Box< \[ AtomicSeq ] >,$/p' ring_mpsc/src/lib.rs
echo '  -- crates writing their own mask, outside ring_index --'
command grep -rn 'capacity()\.mask()\|capacity\.mask()' --include=*.rs */src/lib.rs | command grep -v 'ring_index/' | command grep -v 'ring_types/'
echo '  -- the two stateless crates --'
printf '    ring_index %s pub fn / %s struct\n' "$( command grep -cE '^pub (const )?fn ' ring_index/src/lib.rs )" "$( command grep -cE '^\s*(pub )?struct ' ring_index/src/lib.rs )"
printf '    ring_seqno   %s pub fn / %s struct\n' "$( command grep -cE '^pub (const )?fn ' ring_seqno/src/lib.rs )" "$( command grep -cE '^\s*(pub )?struct ' ring_seqno/src/lib.rs )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX41 | `ring_index` | n/a — unenforced | The one-owner pattern has no lint, test, or visibility restriction behind it; the census that detects a violation exists only in this corpus |
| IX42 | `ring_mpsc` | n/a — observation | "The fold travels with the container" predicts the one violation exactly: `ring_mpsc` wraps `slots` in a `Buffer` and holds `stamps` bare seven lines below |
| IX43 | `ring_index` | n/a — observation | `ring_index` and `ring_seqno` are the only crates with zero structs, traits, statics and zero occurrences of `mut`; eight total functions over `ring_types` values |
| IX44 | `ring_index` | n/a — observation | One of the eight allocates and it is the one with no callers; `ring_batch` declined the container rather than the arithmetic |
