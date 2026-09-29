# api

One type and twelve functions, eleven of which are `pub const fn` carrying
`#[ must_use ]`. The twelfth is the constructor, and it is the only fallible one,
the only non-`const` one, and the only one without an explicit mark — three
exceptions that are all the same function and all correct for different reasons.

The declarations are uniform; what they are used for is not. Ten production reads
exist outside this crate and they reach three of the seven readers. The other
four — `wait()`, `producers()`, `batch()`, `is_tick_safe()` — have no caller in
any `src/` in the workspace, and a neighbouring crate's suite has already measured
the behavioural half of the same result.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_twelve_functions_eleven_of_them_const.md) | Twelve Functions, Eleven of Them `const` | Every declaration, its attributes, and the return-type asymmetry among the readers |
| [002](002_three_readers_used_and_four_with_no_caller.md) | Three Readers Used, and Four With No Caller | The production and test call census, and the suite that measured it from the other side |

## Eleven Identical Prefixes and One Exception

Twelve functions, eleven `const`, eleven `must_use`, and it is the same eleven both
times. `new` is neither, because `Result` is already `#[ must_use ]` in `core` and
because a `?` cannot appear in a const context.

Both absences are right and neither is stated. A reader scanning the file sees
eleven identical two-line prefixes and one function with none, and the difference
that produces that — one `?`, one line into the body — is not visible at the
declaration. `ring_overflow` has the same shape for the same second reason
([§ OV5](../../../ring_overflow/docs/api/001_four_declarations_three_of_them_const.md)),
with the difference that its blocker is an atomic and cannot be removed, while
this one can.

## The Half of the Surface Nobody Reads

`capacity()` is read six times outside the crate, `overflow()` three,
`is_multi_producer()` once. `wait()`, `producers()`, `batch()` and
`is_tick_safe()` are read zero times in any `src/`, and outside this crate's own
tests only `producers()` appears at all — once, in a `ring_factory` assertion
whose purpose is to establish that two configs differ before showing that the
rings they build do not.

That test, `only_two_of_five_config_fields_are_observable_through_the_factory`,
reaches the same conclusion by measurement rather than by census, and names a
distinct reason for each silent field. The two methods agree exactly on
`producers`, `wait` and `batch`. Neither crate records the joined result, and each
knows something the other does not: `ring_factory` scopes its claim to "through
the factory", while the census shows there is no other route either.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every public declaration, with the attribute above it --'
command grep -n -B1 'pub struct \|pub fn \|pub const fn ' ring_config/src/lib.rs | command grep -v '^--$'
echo '  -- totals: functions, const fns, must_use marks --'
command grep -c 'pub fn \|pub const fn ' ring_config/src/lib.rs || true
command grep -c 'pub const fn ' ring_config/src/lib.rs || true
command grep -c 'must_use' ring_config/src/lib.rs || true
echo '  -- every RingConfig accessor call in src/ outside this crate --'
command grep -rn 'config\.\(capacity\|wait\|overflow\|producers\|batch\|is_multi_producer\|is_tick_safe\)()' --include=*.rs */src | command grep -v '^ring_config/' | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
echo '  -- and the measured half of the same result --'
command grep -m1 -B1 -A5 -F '/// The criterion'"'"'s real extent: **two of five fields**, and each of the other' ring_factory/tests/factory_test.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC5 | `ring_config` | n/a — doc gap | Twelve functions, eleven `pub const fn`, eleven `#[ must_use ]`, and it is the same eleven both times — `new` is the sole exception on both counts, correctly, because `Result` is already `#[ must_use ]` in `core` and because its `?` cannot appear in a const context, and neither reason is stated at the declaration where the asymmetry is visible |
| RC6 | `ring_config` | n/a — observation | `capacity()` is the one reader returning a validated newtype where the other four return the field's own type, and of the four non-doctest production reads outside this crate two call `.get()` on it immediately (`ring_bench:278`, `ring_core:189`) and two pass the `Capacity` on whole — an even split that leaves the wrapper neither consistently carried nor consistently unwrapped, with no `capacity_slots() -> usize` for the callers that want the number |
| RC7 | `ring_config` | n/a — unadopted | Ten production reads exist outside this crate and reach three readers — `capacity()` six, `overflow()` three, `is_multi_producer()` once — while `wait()`, `producers()`, `batch()` and `is_tick_safe()` have no caller in any `src/` in the workspace and, outside this crate's own tests, only `producers()` is touched at all, once, so three of five stored fields and one of two derived readings have no consumer of any kind |
| RC8 | `ring_config` | n/a — doc gap | `ring_factory`'s `only_two_of_five_config_fields_are_observable_through_the_factory` reaches the same conclusion by measurement — asserting that configs differing only in `producers`, `wait` or `batch` build indistinguishable rings — and agrees exactly with the call census on those three fields, but scopes its claim to "through the factory" where the census shows no other route exists, and neither crate records the joined result |
