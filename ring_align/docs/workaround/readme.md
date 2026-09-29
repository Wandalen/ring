# workaround

External constraints `ring_align` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

Both entries here are the same kind of constraint — a Rust language limit that
the crate cannot route around and absorbs on its consumers' behalf — and both
are recorded with a compiler probe that decides, in three lines, whether the
workaround is still needed.

### Overview Table

| ID | Name | Constraint | Cost | Status |
|----|------|------------|------|--------|
| 001 | [The Alignment Literal Cannot Be the Constant](001_the_alignment_literal_cannot_be_the_constant.md) | `E0693` — `#[ repr( align( … ) ) ]` takes a literal, never a `const` | The number is written twice inside the crate that owns it | 🔄 |
| 002 | [`into_inner` Cannot Be `const`](002_into_inner_cannot_be_const.md) | `E0493` — a `const fn` cannot drop the remainder of a destructured generic | One of four associated functions is not `const`; zero callers affected | 🔄 |

**001 costs something and 002 does not.** 001 puts a second literal `64`
seventeen lines from the constant that exists to be the only one, which is a
real dent in [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md)'s
claim and a second edit site on every platform port. 002 is a footnote: no
caller in the family unwraps a `CacheAligned`, and the friction lands on the
one operation that discards the crate's guarantee anyway.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
probe()
{
  rustc --crate-name wa_probe --edition 2021 --emit=metadata -o ./-wa_probe.rmeta ./-wa_probe.rs 2>&1 \
  | command grep -oE 'error\[E[0-9]+\]' | head -1
  rm -f ./-wa_probe.rmeta ./-wa_probe.rs
}
echo '  -- E0693: the attribute will not take the constant --'
cat > ./-wa_probe.rs <<'EOF'
const CACHE_LINE : usize = 64;
#[ repr( align( CACHE_LINE ) ) ]
struct A( u64 );
fn main() { let _ = A( 0 ); }
EOF
probe
echo '  -- E0493: the unwrap will not be const --'
cat > ./-wa_probe.rs <<'EOF'
struct W< T >( T );
impl< T > W< T >
{
  pub const fn into_inner( self ) -> T { self.0 }
}
fn main() { let _ = W( 0u8 ).into_inner(); }
EOF
probe
echo '  -- control: both shapes compile clean once the workaround is applied --'
cat > ./-wa_probe.rs <<'EOF'
const CACHE_LINE : usize = 64;
#[ repr( align( 64 ) ) ]
struct W< T >( T );
impl< T > W< T >
{
  pub fn into_inner( self ) -> T { self.0 }
}
fn main() { let _ = CACHE_LINE; let _ = W( 0u8 ).into_inner(); }
EOF
rustc --crate-name wa_probe --edition 2021 --emit=metadata -o ./-wa_probe.rmeta ./-wa_probe.rs 2>&1 | command grep -cE '^error' || true
rm -f ./-wa_probe.rmeta ./-wa_probe.rs
echo '  -- crates whose code carries the name into_inner --'
command grep -rl 'into_inner' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null | cut -d/ -f2 | sort -u | paste -sd' ' - | sed 's:^:    :'
echo '  -- files outside this crate that name CacheAligned, and their into_inner count --'
for f in $( command grep -rl 'CacheAligned' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null | command grep -v '^ring_align/' ); do
  printf '    %s %s\n' "$f" "$( command grep -c 'into_inner' "$f" || true )"
done
```

Live output:

```
  -- E0693: the attribute will not take the constant --
error[E0693]
  -- E0493: the unwrap will not be const --
error[E0493]
  -- control: both shapes compile clean once the workaround is applied --
0
  -- crates whose code carries the name into_inner --
    ring_align ring_bench ring_mpsc ring_shutdown ring_trace
  -- files outside this crate that name CacheAligned, and their into_inner count --
    ring_cursor/src/lib.rs 0
```

The third command is the control the first two need and could not each carry
alone: both workarounds are *expected failures*, so a compiler that rejected
everything would produce the same two error codes. The clean compile of both
shapes with the workaround applied is what separates "the language refuses
this" from "the probe is broken".

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL49 | Both workarounds | n/a — unenforced | Each is decidable in three lines by the compiler, and nothing runs either probe: they live in a document, so the day `E0693` or `E0493` is lifted the crate keeps both workarounds and nobody is told. The deletion condition is written down and unwatched, which is the same shape as [`decisions`](../decisions/readme.md) AL16 |
| AL50 | `into_inner` | n/a — observation | The one file outside this crate that names `CacheAligned` — `ring_cursor/src/lib.rs` — calls `into_inner` zero times, so 002's cost is measured at zero rather than asserted. The five crates carrying the *spelling* are a coincidence of vocabulary: `Mutex`, `PoisonError` and `ring_shutdown`'s own producer each have one |
| AL51 | `src/lib.rs:68` | n/a — observation | 001's cost is smaller than this readme says: the duplicated literal is caught by `tests/align_test.rs:55`, which compares `size_of` against `CACHE_LINE`. The prose above calls it "a second edit site on every platform port" — true, and the port cannot ship with the two out of step |
| AL52 | `ring_shutdown` | n/a — observation | The same `E0493` constraint bites a second crate — `ring_shutdown/src/lib.rs:505` declares its own non-`const` `into_inner`, and `tests/shutdown_test.rs:273` documents the limitation on purpose. Two crates absorbed one language limit independently, and neither workaround document mentions the other |
