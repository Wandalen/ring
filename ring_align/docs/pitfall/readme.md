# Pitfall Doc Definition

### Scope

- **Purpose**: Record the two traps this crate's shape sets — a constant whose wrongness is silent, and a proof that proves the wrong thing.
- **Responsibility**: For each, state the scope, the trap, the failure it produces, and the mitigation.
- **In Scope**: A `CACHE_LINE` too small for the host; `size_of` mistaken for evidence about addresses.
- **Out of Scope**: Traps in the cursors that wear the padding, which are [`ring_cursor`](../../../ring_cursor/readme.md)'s; the cost of over-padding, ruled on in [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [A Constant Too Small Buys Nothing](001_a_constant_too_small_buys_nothing.md) | Every assertion still passes while the padding does nothing — and one crate in the family has already grown a second copy of the number | 🔄 |
| 002 | [`size_of` Proves Nothing About Addresses](002_size_of_proves_nothing_about_addresses.md) | The structural assertion and the property are different claims; the negative control is what tells them apart | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- assertions that decide by size, and by address --'
printf '    size_of %s, from_ref %s\n' \
  "$( command grep -c 'size_of' ring_align/tests/align_test.rs || true )" \
  "$( command grep -c 'from_ref' ring_align/tests/align_test.rs || true )"
echo '  -- the negative control that tells the two apart --'
command grep 'fn two_unwrapped_fields_share_a_line' ring_align/tests/align_test.rs
echo '  -- the declared line size against the one this host reports --'
printf '    declared %s, host %s\n' \
  "$( command grep 'pub const CACHE_LINE' ring_align/src/lib.rs | tr -dc 0-9 )" \
  "$( getconf LEVEL1_DCACHE_LINESIZE )"
echo '  -- where in the family that comparison is made --'
command grep -r 'LEVEL1_DCACHE' ring_*/src ring_*/tests 2>/dev/null
```

Live output:

```
  -- assertions that decide by size, and by address --
    size_of 5, from_ref 4
  -- the negative control that tells the two apart --
fn two_unwrapped_fields_share_a_line()
  -- the declared line size against the one this host reports --
    declared 64, host 64
  -- where in the family that comparison is made --
ring_align/tests/manual/readme.md:getconf LEVEL1_DCACHE_LINESIZE
ring_align/tests/manual/readme.md:| 2026-08-28 | M1 | ✅ | `getconf LEVEL1_DCACHE_LINESIZE` reports 64 on this host — the constant is right here. Still wrong for Apple Silicon (128); the module doc says so and says a port raises the constant rather than making it per-crate conditional. |
```

The third command is the pitfall executing rather than being described: it is
the comparison the crate never makes for itself, and on a host where the two
numbers differ its output changes while every test in the suite keeps passing.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL41 | The declared constant | n/a — coverage | The one place the family compares its constant to the machine is `tests/manual/readme.md` — a procedure a person runs, whose last result is dated `2026-08-28` and which will keep reading ✅ on a machine nobody has re-run it on. The check exists, is written down, and is not a gate |
| AL42 | The suite | **latent hazard** | Every assertion in `align_test.rs` compares the layout against `CACHE_LINE` rather than against the host, so the suite is self-consistent at any value the constant takes. A wrong constant is invisible to the tests by construction, which is why AL41's manual step is the only detector and why its being manual matters |
| AL43 | Size against address | n/a — observation | Five assertions decide by `size_of` and four by `from_ref`, and only the second kind can see the property the crate exists for — the split is close to even, which is the healthy shape, and the crate's own doc at `tests/align_test.rs:48` says so in as many words |
| AL44 | `two_unwrapped_fields_share_a_line` | n/a — observation | The negative control is what separates a real address measurement from one that would pass on any pair of fields: it asserts two *unwrapped* fields do share a line, so a change that broke the address arithmetic would have to break both tests in opposite directions to stay hidden |
