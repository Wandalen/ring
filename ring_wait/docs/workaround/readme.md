# workaround

Two capabilities this crate needs and cannot have, each substituted with the
cheapest thing that preserves the property that mattered, each with the cost it
imposes and the condition that deletes it.

The first is inside the crate: `WaitKind::Park` cannot park, because parking is
half of a rendezvous and this crate has no way to learn who the other half is.
The second is outside it: the family cannot express "no tick-path crate may
transitively reach a blocking call", so it matches the literal text `"ring_wait"`
instead — a ban imposed on this crate's consumers, on this crate's account, with
none of the machinery living here.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Sleep Where a Park Belongs](001_a_sleep_where_a_park_belongs.md) | The `Park` arm's sleep, the registration relationship that forces it, the 2.2–2.4× gap between requested and delivered, and the two crates computing against the requested figure |
| 002 | [A Crate Name as a Load-Bearing String](002_a_crate_name_as_a_load_bearing_string.md) | WT10 and WT11 — `PARKING_CRATES`, the manifest scan, the two crates that reach a parking operation without appearing on either list, and the entry that is a self-reference |

### The Two, Side by Side

| | Sleep-for-park (001) | Name-as-ban (002) |
|--|----------------------|-------------------|
| Missing capability | a handle registry, to unpark a waiter | a transitive dependency ban Cargo can express |
| Owned by | `ring_handle` — who-knows-whom | nobody; Cargo has no such key |
| Substituted with | `sleep( 50 µs )` | the string `"ring_wait"` |
| Lives in | `src/lib.rs:141` | `ring_poll` and `ring_handle` — **not here** |
| Preserves | the cost profile (idle, not spinning) | the guarantee, for the three declared tick-path crates |
| Loses | wake-on-publication; 2.2–2.4× overshoot | transitivity — two crates reach it unlisted |
| Guarded by | W4 — the explaining comment must survive | the manifest scan, which cannot drift |
| Deletable when | a publisher can reach a waiter's handle | a dependency ban becomes a rule instead of text |
| Would a rename break it? | no | **yes, and misleadingly** |

The pairing is not incidental. The second workaround exists because the first
one does: this crate blocks, so the tick path must not reach it, so the family
needs a ban it cannot write.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# 001 — the substitution, and the absence it stands in for
grep -r --include=*.rs "thread::park\|::unpark" */ 
grep -r "50µs\|50 µs" */tests/*.rs 
command grep -r "from_micros" */tests/*.rs 

# 002 — the three load-bearing strings, and none of them here
grep -r "PARKING_CRATES" */src/*.rs 
grep -r "PARKING_CRATES\|FORBIDDEN" ring_wait/ --include=*.rs
grep -l "ring_wait" ring_*/Cargo.toml | sed 's|/Cargo.toml||'
```

Live output:

```
ring_wait/src/lib.rs:      // Sleeping rather than `thread::park` on purpose. Parking requires the
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/input_timeline/tests/timeline_test.rs:    "event 1 is stamped 50µs, before its predecessor at 100µs",
ring_handle/tests/handle_test.rs:    "10 000 empty drains took {elapsed:?}; a 50µs park each would be ~500ms"
ring_poll/tests/poll_test.rs:/// The arithmetic is the assertion. `ring_wait`'s `Park` variant sleeps 50µs
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_live_capture/tests/capture_test.rs:  std::thread::sleep( Duration::from_micros( 50 ) );
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_live_capture/tests/capture_test.rs:  std::thread::sleep( Duration::from_micros( 50 ) );
ring_poll/src/lib.rs://!    [`PARKING_CRATES`] and `docs/invariant/001`.
ring_poll/src/lib.rs:/// assert!( ring_poll::PARKING_CRATES.contains( &"ring_wait" ) );
ring_poll/src/lib.rs:/// assert!( !ring_poll::PARKING_CRATES.contains( &"ring_handle" ) );
ring_poll/src/lib.rs:pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
ring_barrier
ring_shutdown
ring_wait
```

| | Value |
|--|------:|
| `thread::park` calls in the family | 0 |
| `thread::park` mentions in the family | 1 — the comment at `src/lib.rs:134` |
| Requested sleep | 50 µs |
| Delivered sleep | 112–119 µs (2.2–2.4×) |
| Tests asserting the 50 µs value | **0** |
| Tests computing against it | 2 — `ring_poll`, `ring_handle` |
| Places `"ring_wait"` is load-bearing text | 3 |
| Of those, inside this crate | **0** |
| Manifests containing the string | 3 — 2 dependency edges, 1 self-reference |
| Crates whose closure reaches `ring_wait` | 4 |
| Declared on `PARKING_CRATES` | 3 |
| Tick-path crates reaching it | 0 — verified by closure, not by substring |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT10 | `ring_poll` | n/a — unenforced | `ring_poll::PARKING_CRATES` is declared as "crates from which a parking operation is reachable" and measured as "manifests containing the text"; `ring_consume` reaches one through `ring_barrier` and `ring_testkit` through `ring_shutdown`, and neither appears on either list |
| WT11 | `ring_poll` | n/a — unenforced | Of the three entries the manifest scan does produce, two are dependency edges and the third is `ring_wait`'s own `name =` field; the rule is "the text appears", which reaches the right answer for a reason unrelated to the question |
| WT51 | `ring_wait` | n/a — observation | The crate exports one public constant and has two tunables: `DEFAULT_SPINS` is documented, doctested, and overridable by argument at every call site; the 50 µs sleep is an unnamed literal reachable through no parameter, and is the one two other crates transcribe rather than reference (WT7) |
| WT52 | `ring_poll` | n/a — observation | `PARKING_CRATES` is a fixed-size array in the enforcing crate, so a crate that legitimately needs to wait cannot declare that for itself — it adds the dependency, `ring_poll`'s suite fails, and the fix is an edit to `ring_poll`; the permission lives with the enforcer, which is the correct trade for a rule Cargo cannot express |
