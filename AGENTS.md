# TimeKit — agent notes

- Zero dependencies; the engine is pure logic. Zone data is GENERATED (tools/gen_zones.py -> src/zones.rs) from IANA tzdata — never hand-edit zones.rs; regenerate and note the tzdata version.
- The two documented caveats are contractual honesty, not bugs: POSIX rules = current rule set only (no pre-change history), and Morocco's two zones follow decree transitions their POSIX strings don't express. Full TZif tables (ROADMAP #1) are the fix for both — do not paper over them with special cases.
- Correctness bar: any engine change must re-run the differential check against Python zoneinfo (sampled instants across representative zones incl. southern hemisphere, half/quarter-hour offsets, and Lord Howe's 30-minute DST) in addition to unit tests.
- POSIX offset signs are inverted vs UTC offsets (CST6 = UTC-6); the parser's sign handling has a regression test via Asia/Tokyo (JST-9) — keep it.
- Gate: WinDevKit tools/gate.sh (fmt + clippy both targets + full tests). Check/clippy do not link — CI's real build is the link gate.
