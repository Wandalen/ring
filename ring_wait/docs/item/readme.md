# item

Seven public items, read line by line. The split is by role rather than by
alphabet: one instance takes the function where all four strategies differ, the
other takes the four that share a single loop and differ only in what they have
already decided for you.

The two together cover the whole surface — `DEFAULT_SPINS` and
`escalation_hint` appear in the second instance's closing table with pointers to
where each is read in full.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Four Arms of the Pause](001_the_four_arms_of_the_pause.md) | `pause` at `:112-146` — each arm's code, cost, and return, WT21 (three arms ignore `attempt`), and the only arm whose comment outweighs its code |
| 002 | [The Loop, the Wrapper, and the Two Questions](002_the_loop_the_wrapper_and_the_two_questions.md) | `wait_until`, `wait`, `for_space`, `for_data` as a body-size ladder, the opposite-error pair, the `count = 0` corner, and WT4 (who keeps the attempt count) |

### The Surface, Measured

```sh
cd "$(git rev-parse --show-toplevel)"

# every public item
grep -E "^pub (const|fn)" ring_wait/src/lib.rs

# body size — first statement through the function's closing brace
for r in "84 91 escalation_hint" "114 145 pause" "183 195 wait_until" \
         "214 215 wait" "241 242 for_space" "268 269 for_data"; do
  set -- $r
  printf '%-16s %2d\n' "$3" "$( awk -v a=$1 -v b=$2 'NR>=a&&NR<=b' \
    ring_wait/src/lib.rs | wc -l )"
done
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
escalation_hint   8
pause            32
wait_until       13
wait              2
for_space         2
for_data          2
```

| Item | Line | Kind | Body | Lines | Read in |
|------|-----:|------|------|------:|---------|
| `DEFAULT_SPINS` | 63 | `const` | — | — | [`data_structure/002`](../data_structure/002_the_budget_and_the_attempt_index.md) |
| `escalation_hint` | 83 | `const fn` | `:84-91` | 8 | [`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) |
| `pause` | 112 | `fn` | `:114-145` | 32 | [001](001_the_four_arms_of_the_pause.md) |
| `wait_until` | 179 | `fn` | `:183-195` | **13** | [002](002_the_loop_the_wrapper_and_the_two_questions.md) |
| `wait` | 210 | `fn` | `:214-215` | 2 | [002](002_the_loop_the_wrapper_and_the_two_questions.md) |
| `for_space` | 239 | `fn` | `:241-242` | 2 | [002](002_the_loop_the_wrapper_and_the_two_questions.md) |
| `for_data` | 265 | `fn` | `:268-269` | 2 | [002](002_the_loop_the_wrapper_and_the_two_questions.md) |

The thirteen lines the rest of this collection keeps referring to are
`wait_until`'s — the loop header at `:183` through the closing brace at `:195`.
The three one-expression wrappers are two lines each only because the closing
brace gets its own.

| | Count |
|--|------:|
| Public items | 7 |
| Items that are one expression | 3 |
| Items with a `match` | 2 |
| Items marked `#[ must_use ]` | 1 |
| Items declared `const fn` | 1 |
| Items with a caller outside this crate | 1 |

### Regenerate the Per-Arm Census

```sh
cd "$(git rev-parse --show-toplevel)"

# comment-to-code ratio per `pause` arm
for r in "116 126 Spin" "127 131 Yield" "132 143 Park" "144 144 None"; do
  set -- $r
  c=$( awk -v a=$1 -v b=$2 'NR>=a&&NR<=b' ring_wait/src/lib.rs \
       | grep -cE "^[[:space:]]*//" )
  t=$(( $2 - $1 + 1 ))
  printf '%-6s total=%2d  comment=%2d  code=%2d\n' "$3" "$t" "$c" "$(( t - c ))"
done

# which arms read the attempt index
awk -v n1="$(( $( command grep -n -m1 -F 'pub fn pause( kind : WaitKind, attempt : usize ) -> bool' ring_wait/src/lib.rs | cut -d: -f1 ) + 2 ))" -v n2="$(( $( command grep -n -m1 -F '    WaitKind::None => false,' ring_wait/src/lib.rs | cut -d: -f1 ) + 1 ))" 'NR>=n1 && NR<=n2' ring_wait/src/lib.rs | grep -n "attempt" | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'

# any dispatch below `pause` — expected: none
awk -v n1="$(( $( command grep -n -m1 -F '    WaitKind::None => false,' ring_wait/src/lib.rs | cut -d: -f1 ) + 2 ))" 'NR>n1' ring_wait/src/lib.rs | grep -vE "^[[:space:]]*//" \
  | grep -nE "match|if[[:space:]]+kind|WaitKind::" \
  || echo '(no dispatch below pause)' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
# control: the identical expression above it, where the dispatch is
awk -v n1="$(( $( command grep -n -m1 -F '    WaitKind::None => false,' ring_wait/src/lib.rs | cut -d: -f1 ) + 2 ))" 'NR<=n1' ring_wait/src/lib.rs | grep -vE "^[[:space:]]*//" \
  | grep -cE "match|if[[:space:]]+kind|WaitKind::"
```

Live output:

```
Spin   total=11  comment= 3  code= 8
Yield  total= 5  comment= 0  code= 5
Park   total=12  comment= 7  code= 5
None   total= 1  comment= 0  code= 1
      for _ in 0..=( attempt % 8 )
(no dispatch below pause)
9
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT4 | family | n/a — observation | Two of the three production call sites discard the attempt count, both because they had something better to return, so the crate's return type was designed for the consumer it does not have |
| WT21 | `ring_wait` | n/a — observation | Three of `pause`'s four arms take an `attempt` parameter they never read, and the fourth reads it through `% 8`, so the parameter's consumed information content is three bits delivered to one arm |
| WT37 | `ring_wait` | n/a — observation | `escalation_hint` is the crate's only `const fn`, and the five items callers actually reach are excluded from const-ness by the two `std` calls in `pause` — so the keyword marks the one function with nothing observable to do and no caller to do it for |
| WT38 | `ring_wait` | n/a — coverage | `pause`'s doctest exercises `Spin` and `None`, the two arms that stay inside `core`; the two that reach `std` — the ones that make the crate un-`no_std`, trigger `ring_handle`'s guard, and carry the crate's largest decision — have no executable example |
