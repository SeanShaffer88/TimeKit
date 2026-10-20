/* TimeKit.dll — IANA timezones and civil time, done once, correctly.
 *
 * 497 zones from the IANA Time Zone Database (see tk_tzdata_version),
 * evaluated from the zones' current POSIX rule sets: UTC<->local with
 * explicit DST gap/overlap policy, calendar arithmetic, transitions,
 * ISO-8601 parsing, formatting. Strings are UTF-8; returned char* are
 * caller-owned (free with tk_free_string). Struct outputs use the two
 * layouts below. Errors set thread-local tk_last_error(). */
#ifndef TIMEKIT_H
#define TIMEKIT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32) && defined(TK_BUILD)
#define TK_API __declspec(dllexport)
#elif defined(_WIN32)
#define TK_API __declspec(dllimport)
#else
#define TK_API
#endif

enum {
    TK_OK = 0,
    TK_ERR_INVALID_ARG = 1,
    TK_ERR_UNKNOWN_ZONE = 2,
    TK_ERR_GAP = 3,       /* local time does not exist (policy REJECT) */
    TK_ERR_AMBIGUOUS = 4, /* local time occurs twice (policy REJECT) */
    TK_ERR_INTERNAL = 6
};

/* from_local policies */
enum { TK_EARLIER = 0, TK_LATER = 1, TK_REJECT = 2 };
/* from_local flags: 0 unique, 1 gap-shifted, 2 ambiguous-chosen */

typedef struct {
    int64_t offset_secs;
    int is_dst;
    char abbr[16];
} TkInfo;

typedef struct {
    int year, month, day, hour, minute, second;
    int64_t offset_secs;
    int is_dst;
    char abbr[16];
} TkCivil;

/* "0.1.0" and e.g. "2026c" — static strings, do not free. */
TK_API const char *tk_version(void);
TK_API const char *tk_tzdata_version(void);

TK_API int tk_zone_count(void);
TK_API char *tk_zone_name(int index);

TK_API int tk_offset_at(const char *zone, int64_t unix, TkInfo *out);
TK_API int tk_to_local(const char *zone, int64_t unix, TkCivil *out);
TK_API int tk_from_local(const char *zone, const TkCivil *in, int policy,
                         int64_t *unix_out, int *flags_out);
TK_API int tk_add_calendar(const char *zone, int64_t unix, int64_t years,
                           int64_t months, int64_t days, int64_t secs,
                           int64_t *out);
TK_API int tk_next_transition(const char *zone, int64_t unix,
                              int64_t *unix_out, TkInfo *info_out,
                              int *found_out);
TK_API char *tk_format(const char *zone, int64_t unix, const char *fmt);
TK_API int tk_parse_iso(const char *text, int64_t *unix_out);
TK_API int64_t tk_now_utc(void);

TK_API void tk_free_string(char *ptr);
TK_API const char *tk_last_error(void);
TK_API int tk_last_error_code(void);

#ifdef __cplusplus
}
#endif
#endif /* TIMEKIT_H */
