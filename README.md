# TimeKit

**Timezones are where date code goes to be wrong quietly.** TimeKit does them once, correctly, behind a small C ABI: all 497 canonical IANA zones, DST transitions, and local↔UTC conversion with explicit answers for the two cases everyone else hand-waves — the spring-forward **gap** (a local time that doesn't exist) and the fall-back **overlap** (a local time that happens twice).

```c
TkCivil c;  tk_to_local("America/Chicago", unix, &c);
int64_t t; int flags;
tk_from_local("America/Chicago", &wall, TK_EARLIER, &t, &flags); // flags tells you if it was gap/overlap
tk_add_calendar(zone, t, 0, 1, 0, 0, &out);   // +1 month, clamped, zone-aware
tk_next_transition(zone, t, &when, &after, &found);
tk_format(zone, t, "%Y-%m-%d %H:%M %Z %z");
tk_parse_iso("2026-10-20T15:15:30+02:00", &t);
```

## How it works

Zone data is the IANA Time Zone Database (**tzdata 2026c**, see `tk_tzdata_version()`), compiled in at build time by `tools/gen_zones.py`: each zone's POSIX rule string (the rule set in force) is embedded — 497 zones deduplicate to 95 distinct rules, a few KB total. No OS timezone dependency, no files to ship, identical answers on every machine.

## The honest caveats (read these)

- **Rules are the current rule set.** POSIX rules project correctly into the future and recent past. An instant from before a rule change (e.g. US dates before the 2007 DST change) is evaluated by *today's* rules. Full historical transition tables are the top roadmap item.
- **Morocco is decree-based.** Africa/Casablanca and Africa/El_Aaiun observe Ramadan suspensions set by decree; even their IANA POSIX strings (`<+00>0`) don't describe actual practice (+01 with Ramadan dips). Only full TZif tables capture them. TimeKit matches the published rule string; treat those two zones as approximate until the TZif table work lands.

## Verification status

Unit tests cover Chicago's 2026 DST boundaries to the second, gap/overlap policy outcomes, southern-hemisphere seasons, calendar clamping, ISO parsing, and formatting. Beyond that, the shipped engine was **differentially tested against Python's `zoneinfo`** (an independent implementation): 6,600 sampled instants across 22 zones, 2024–2028 — exact offset/DST agreement everywhere except the two documented Morocco zones (whose mismatch is the IANA footer's own, described above).

Building: `cargo build --release` → `timekit.dll` / `libtimekit.so`; `cargo test --locked`. Windows CI runs the full gate plus a real MSVC link.

License: MIT (the zone data is the IANA public-domain compilation). See `AGENTS.md` for build discipline.
