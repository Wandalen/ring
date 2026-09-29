# Non Functional Requirement Doc Definition

### Scope

- **Purpose**: State the two requirements this crate is held to that are not about what it computes — that its claim be decidable by a test, and that it add nothing to the running program.
- **Responsibility**: For each, state the requirement, the evidence it is met, and the part that is deferred or paid for elsewhere.
- **In Scope**: Verifiability of the padding claim; runtime and memory cost.
- **Out of Scope**: The throughput claim itself, which belongs to a separate staged benchmark plan; correctness of the constant, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Structural Claim Is Testable](001_the_structural_claim_is_testable.md) | This crate makes two claims; it delivers the one a test can decide and defers the other explicitly, with a four-layer stack behind the half it does deliver | 🔄 |
| 002 | [The Crate Costs Nothing at Runtime](002_the_crate_costs_nothing_at_runtime.md) | No allocation, no branch, no instruction — and a memory cost of 56 bytes per wrapped word that is paid whether or not it buys anything | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- payload bytes against wrapper bytes, for a cursor-sized word --'
cat > ./-nfr_probe.rs <<'EOF'
#[ repr( align( 64 ) ) ]
struct CacheAligned< T >( #[ allow( dead_code ) ] T );
fn main()
{
  use core::mem::size_of;
  let p = size_of::< u64 >();
  let w = size_of::< CacheAligned< u64 > >();
  println!( "    u64 payload {p}, wrapper {w}, padding {} bytes per wrapped word", w - p );
}
EOF
rustc --crate-name nfr_probe --edition 2021 -o ./-nfr_probe ./-nfr_probe.rs 2>/dev/null && ./-nfr_probe
rm -f ./-nfr_probe ./-nfr_probe.rs
echo '  -- branches in the crate body --'
sed 's://.*::' ring_align/src/lib.rs | command grep -cE '\bif\b|\bmatch\b' || true
echo '  -- control: branches exist in the crate one tier down --'
sed 's://.*::' ring_types/src/capacity.rs | command grep -cE '\bif\b|\bmatch\b' || true
echo '  -- where the deferred half is sent --'
command grep 'workstream' ring_align/src/lib.rs
[ -e docs/workstream/008_ring_write_path.md ] && echo '    the cited path exists' || echo '    the cited path does not exist'
[ -d docs/workstream/008_ring_write_path ] && echo '    a directory of that name, without the .md, does'
```

Live output (re-run 2026-09-29, after `ring` was extracted into its own
repository — see the AL31 row's note below):

```
  -- payload bytes against wrapper bytes, for a cursor-sized word --
    u64 payload 8, wrapper 64, padding 56 bytes per wrapped word
  -- branches in the crate body --
0
  -- control: branches exist in the crate one tier down --
2
  -- where the deferred half is sent --
    the cited path does not exist
```

The grep at "where the deferred half is sent" now matches nothing — `src/lib.rs`
no longer cites `docs/workstream/` at all, so there is no line for it to print.
The directory check is also silent: `docs/workstream/` never existed inside
`ring`'s own tree, only in the private monorepo `ring` was extracted out of.

**The branch count carries a control** because a zero is the claim: a pattern
that matched nothing anywhere would produce the same zero as a crate with no
branches. `ring_types/src/capacity.rs` is measured identically and reports two,
so the zero above is the crate and not the grep.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL29 | The padding | **measured cost** | 56 bytes of every 64 are padding for a cursor-sized payload — the crate's memory cost is seven-eighths waste by construction, and it is paid unconditionally: on a machine whose line is 128 the same 56 bytes buy no separation at all, and on a struct with one wrapped field they buy nothing either way |
| AL30 | The runtime cost | n/a — observation | Zero branches in the crate body against two in `ring_types/src/capacity.rs` measured the same way, and every entry point but `into_inner` is `const fn` — the "costs nothing at runtime" claim is about instructions, and the claim that is *not* made is about memory, which AL29 measures |
| AL31 | `src/lib.rs:4` | **wrong doc** | The deferred half of the requirement is sent to a path that does not exist; the material is in a *directory* of that name without the extension. The scope line of this very definition inherits the deferral, so a reader following the requirement to its resolution lands on nothing, one character away from four documents |
| AL32 | The split itself | n/a — observation | This definition holds a requirement that is met and a requirement that is deferred, and only the met half has evidence in the crate. Nothing in the repository watches the deferred half — no gate, no test and no task refers back to it — so "deferred" and "dropped" are, from inside this crate, the same state |
