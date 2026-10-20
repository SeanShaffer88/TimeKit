# TimeKit Roadmap

## v0.1 (current)
All 497 canonical zones from tzdata 2026c POSIX rule strings; UTC↔local with explicit gap/overlap policies; calendar arithmetic; next-transition; ISO-8601 parse; strftime-subset format; differentially verified against Python zoneinfo.

## Next, in order
1. **Full TZif transition tables** — removes both v0.1 caveats at once: correct historical instants (pre-rule-change dates) and correct Morocco (decree transitions are in the tables). Data cost is a few hundred KB compiled; the generator already reads TZif, so this extends it to emit transition lists with the POSIX rule as the tail behavior.
2. **tzdata refresh tooling**: one command to regenerate zones.rs from a new tzdata release + a diff report (zones added/changed) so updates are reviewable.
3. **Week-of-year / day-of-year formatting** (%j, %U/%W) and 12-hour tokens, if a consumer asks — the formatter grows by demand, not speculation.
4. **Interval type**: (start, end) pairs with overlap/contains/duration helpers in zone-aware terms — the natural companion for scheduling consumers (pairs with next-transition for "fire at local 09:00 daily" style jobs).

## Explicitly not planned
- Leap-second smearing or TAI: consumers who need them need a dedicated library, and pretending otherwise would be worse than saying no.
- Locale month/day *names* (%B/%A): names are a localization concern; numeric + abbreviation formatting stays locale-free on purpose.
