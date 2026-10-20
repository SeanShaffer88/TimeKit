//! C ABI surface — everything declared in include/timekit.h.
//! Conventions (house standard): string inputs are const char* UTF-8;
//! outputs fill caller-provided structs (TkInfo / TkCivil) whose layout
//! matches the header exactly; `*mut c_char` results are caller-owned
//! (free with tk_free_string); failures return an ERR_* code + a
//! thread-local last-error message; nothing unwinds across the boundary.

// The exported functions ARE the raw-pointer C ABI declared in
// include/timekit.h; pointer validity is the documented contract there.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::catch_unwind;

use crate::{Civil, ZoneInfo, ERR_INTERNAL, ERR_INVALID_ARG, TK_OK};

#[repr(C)]
pub struct TkInfo {
    pub offset_secs: i64,
    pub is_dst: c_int,
    pub abbr: [c_char; 16],
}

#[repr(C)]
pub struct TkCivil {
    pub year: c_int,
    pub month: c_int,
    pub day: c_int,
    pub hour: c_int,
    pub minute: c_int,
    pub second: c_int,
    pub offset_secs: i64,
    pub is_dst: c_int,
    pub abbr: [c_char; 16],
}

fn fill_abbr(dst: &mut [c_char; 16], abbr: &str) {
    for slot in dst.iter_mut() {
        *slot = 0;
    }
    for (i, byte) in abbr.bytes().take(15).enumerate() {
        dst[i] = byte as c_char;
    }
}

fn fill_info(out: *mut TkInfo, zone: &crate::Zone, info: ZoneInfo) {
    if out.is_null() {
        return;
    }
    unsafe {
        (*out).offset_secs = info.offset;
        (*out).is_dst = if info.is_dst { 1 } else { 0 };
        let abbr = if info.is_dst {
            zone.dst
                .as_ref()
                .map(|d| d.abbr.as_str())
                .unwrap_or(&zone.std_abbr)
        } else {
            &zone.std_abbr
        };
        fill_abbr(&mut (*out).abbr, abbr);
    }
}

thread_local! {
    static LAST_ERROR: RefCell<(i32, CString)> =
        RefCell::new((TK_OK, CString::new("").unwrap()));
}

fn set_error(code: i32, msg: &str) {
    let clean: String = msg.chars().filter(|&c| c != '\0').collect();
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = (
            code,
            CString::new(clean).unwrap_or_else(|_| CString::new("error").unwrap()),
        );
    });
}

fn clear_error() {
    set_error(TK_OK, "");
}

fn guard<T, F: FnOnce() -> T>(f: F) -> std::thread::Result<T> {
    catch_unwind(std::panic::AssertUnwindSafe(f))
}

fn panic_code() -> c_int {
    set_error(ERR_INTERNAL, "internal panic caught at the FFI boundary");
    ERR_INTERNAL
}

fn zone_input(ptr: *const c_char) -> Result<crate::Zone, c_int> {
    if ptr.is_null() {
        set_error(ERR_INVALID_ARG, "zone must not be null");
        return Err(ERR_INVALID_ARG);
    }
    let name = match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(ERR_INVALID_ARG, "zone is not valid UTF-8");
            return Err(ERR_INVALID_ARG);
        }
    };
    crate::zone_by_name(name).map_err(|(c, m)| {
        set_error(c, &m);
        c
    })
}

fn code(r: crate::TResult<()>) -> c_int {
    match r {
        Ok(()) => {
            clear_error();
            TK_OK
        }
        Err((c, m)) => {
            set_error(c, &m);
            c
        }
    }
}

#[no_mangle]
pub extern "C" fn tk_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn tk_tzdata_version() -> *const c_char {
    static TZV: std::sync::OnceLock<CString> = std::sync::OnceLock::new();
    TZV.get_or_init(|| CString::new(crate::zones::TZDATA_VERSION).unwrap())
        .as_ptr()
}

#[no_mangle]
pub extern "C" fn tk_zone_count() -> c_int {
    crate::zones::ZONES.len() as c_int
}

/// Zone name by index (0..tk_zone_count()-1); caller frees. Null when
/// out of range.
#[no_mangle]
pub extern "C" fn tk_zone_name(index: c_int) -> *mut c_char {
    match guard(|| {
        if index < 0 || index as usize >= crate::zones::ZONES.len() {
            set_error(ERR_INVALID_ARG, "zone index out of range");
            return std::ptr::null_mut();
        }
        clear_error();
        CString::new(crate::zones::ZONES[index as usize].0)
            .unwrap()
            .into_raw()
    }) {
        Ok(p) => p,
        Err(_) => {
            let _ = panic_code();
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn tk_offset_at(zone: *const c_char, unix: i64, out: *mut TkInfo) -> c_int {
    match guard(|| {
        let z = match zone_input(zone) {
            Ok(z) => z,
            Err(c) => return c,
        };
        if out.is_null() {
            set_error(ERR_INVALID_ARG, "out must not be null");
            return ERR_INVALID_ARG;
        }
        let info = z.info_at(unix);
        fill_info(out, &z, info);
        clear_error();
        TK_OK
    }) {
        Ok(c) => c,
        Err(_) => panic_code(),
    }
}

#[no_mangle]
pub extern "C" fn tk_to_local(zone: *const c_char, unix: i64, out: *mut TkCivil) -> c_int {
    match guard(|| {
        let z = match zone_input(zone) {
            Ok(z) => z,
            Err(c) => return c,
        };
        if out.is_null() {
            set_error(ERR_INVALID_ARG, "out must not be null");
            return ERR_INVALID_ARG;
        }
        let (c, info, abbr) = z.to_local(unix);
        unsafe {
            (*out).year = c.year as c_int;
            (*out).month = c.month as c_int;
            (*out).day = c.day as c_int;
            (*out).hour = c.hour as c_int;
            (*out).minute = c.minute as c_int;
            (*out).second = c.second as c_int;
            (*out).offset_secs = info.offset;
            (*out).is_dst = if info.is_dst { 1 } else { 0 };
            fill_abbr(&mut (*out).abbr, &abbr);
        }
        clear_error();
        TK_OK
    }) {
        Ok(c) => c,
        Err(_) => panic_code(),
    }
}

/// policy: 0 earlier, 1 later, 2 reject (gap/overlap become errors).
/// flags_out (may be null): 0 unique, 1 gap-shifted, 2 ambiguous-chosen.
#[no_mangle]
pub extern "C" fn tk_from_local(
    zone: *const c_char,
    in_civil: *const TkCivil,
    policy: c_int,
    unix_out: *mut i64,
    flags_out: *mut c_int,
) -> c_int {
    match guard(|| {
        let z = match zone_input(zone) {
            Ok(z) => z,
            Err(c) => return c,
        };
        if in_civil.is_null() || unix_out.is_null() {
            set_error(ERR_INVALID_ARG, "civil/unix_out must not be null");
            return ERR_INVALID_ARG;
        }
        let c = unsafe {
            Civil {
                year: (*in_civil).year as i64,
                month: (*in_civil).month as u32,
                day: (*in_civil).day as u32,
                hour: (*in_civil).hour as u32,
                minute: (*in_civil).minute as u32,
                second: (*in_civil).second as u32,
            }
        };
        match z.from_local(c, policy) {
            Ok((unix, flags)) => {
                unsafe {
                    *unix_out = unix;
                    if !flags_out.is_null() {
                        *flags_out = flags;
                    }
                }
                clear_error();
                TK_OK
            }
            Err((cc, m)) => {
                set_error(cc, &m);
                cc
            }
        }
    }) {
        Ok(c) => c,
        Err(_) => panic_code(),
    }
}

#[no_mangle]
pub extern "C" fn tk_add_calendar(
    zone: *const c_char,
    unix: i64,
    years: i64,
    months: i64,
    days: i64,
    secs: i64,
    out: *mut i64,
) -> c_int {
    match guard(|| {
        let z = match zone_input(zone) {
            Ok(z) => z,
            Err(c) => return c,
        };
        if out.is_null() {
            set_error(ERR_INVALID_ARG, "out must not be null");
            return ERR_INVALID_ARG;
        }
        code(
            crate::add_calendar(&z, unix, years, months, days, secs).map(|v| {
                unsafe { *out = v };
            }),
        )
    }) {
        Ok(c) => c,
        Err(_) => panic_code(),
    }
}

/// Next transition after `unix`. On TK_OK, *found_out is 1 and
/// unix_out/info_out describe the transition and the state after it;
/// *found_out is 0 for fixed zones (no transitions exist).
#[no_mangle]
pub extern "C" fn tk_next_transition(
    zone: *const c_char,
    unix: i64,
    unix_out: *mut i64,
    info_out: *mut TkInfo,
    found_out: *mut c_int,
) -> c_int {
    match guard(|| {
        let z = match zone_input(zone) {
            Ok(z) => z,
            Err(c) => return c,
        };
        if unix_out.is_null() || found_out.is_null() {
            set_error(ERR_INVALID_ARG, "unix_out/found_out must not be null");
            return ERR_INVALID_ARG;
        }
        match z.next_transition(unix) {
            Some((t, info)) => {
                unsafe {
                    *unix_out = t;
                    *found_out = 1;
                }
                fill_info(info_out, &z, info);
                clear_error();
                TK_OK
            }
            None => {
                unsafe { *found_out = 0 };
                clear_error();
                TK_OK
            }
        }
    }) {
        Ok(c) => c,
        Err(_) => panic_code(),
    }
}

/// Format `unix` in `zone` with the subset: %Y %m %d %H %M %S %z %Z %s %%.
#[no_mangle]
pub extern "C" fn tk_format(zone: *const c_char, unix: i64, fmt: *const c_char) -> *mut c_char {
    match guard(|| {
        let z = match zone_input(zone) {
            Ok(z) => z,
            Err(_) => return std::ptr::null_mut(),
        };
        if fmt.is_null() {
            set_error(ERR_INVALID_ARG, "fmt must not be null");
            return std::ptr::null_mut();
        }
        let f = match unsafe { CStr::from_ptr(fmt) }.to_str() {
            Ok(s) => s,
            Err(_) => {
                set_error(ERR_INVALID_ARG, "fmt is not valid UTF-8");
                return std::ptr::null_mut();
            }
        };
        match CString::new(crate::format(&z, unix, f)) {
            Ok(c) => {
                clear_error();
                c.into_raw()
            }
            Err(_) => {
                set_error(ERR_INTERNAL, "format result contained a NUL byte");
                std::ptr::null_mut()
            }
        }
    }) {
        Ok(p) => p,
        Err(_) => {
            let _ = panic_code();
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn tk_parse_iso(text: *const c_char, unix_out: *mut i64) -> c_int {
    match guard(|| {
        if text.is_null() || unix_out.is_null() {
            set_error(ERR_INVALID_ARG, "text/unix_out must not be null");
            return ERR_INVALID_ARG;
        }
        let s = match unsafe { CStr::from_ptr(text) }.to_str() {
            Ok(s) => s,
            Err(_) => {
                set_error(ERR_INVALID_ARG, "text is not valid UTF-8");
                return ERR_INVALID_ARG;
            }
        };
        code(crate::parse_iso(s).map(|v| unsafe { *unix_out = v }))
    }) {
        Ok(c) => c,
        Err(_) => panic_code(),
    }
}

#[no_mangle]
pub extern "C" fn tk_now_utc() -> i64 {
    crate::platform::now_unix()
}

#[no_mangle]
pub extern "C" fn tk_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = guard(|| unsafe {
            drop(CString::from_raw(ptr));
        });
    }
}

#[no_mangle]
pub extern "C" fn tk_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().1.as_ptr())
}

#[no_mangle]
pub extern "C" fn tk_last_error_code() -> c_int {
    LAST_ERROR.with(|e| e.borrow().0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    fn cs(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    #[test]
    fn ffi_contract() {
        assert!(!tk_version().is_null());
        assert!(tk_zone_count() > 400);
        let name = tk_zone_name(0);
        assert!(!name.is_null());
        tk_free_string(name);
        assert!(tk_zone_name(999_999).is_null());

        let mut info = TkInfo {
            offset_secs: 0,
            is_dst: 0,
            abbr: [0; 16],
        };
        assert_eq!(
            tk_offset_at(cs("America/Chicago").as_ptr(), 0, &mut info),
            TK_OK
        );
        assert_eq!(info.offset_secs, -6 * 3600); // Jan 1970: CST
        assert_eq!(
            tk_offset_at(cs("Nope/Zone").as_ptr(), 0, &mut info),
            crate::ERR_UNKNOWN_ZONE
        );
        assert_eq!(tk_offset_at(ptr::null(), 0, &mut info), ERR_INVALID_ARG);

        let mut out: i64 = 0;
        assert_eq!(
            tk_parse_iso(cs("1970-01-01T00:00:00Z").as_ptr(), &mut out),
            TK_OK
        );
        assert_eq!(out, 0);

        let mut civil = TkCivil {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            offset_secs: 0,
            is_dst: 0,
            abbr: [0; 16],
        };
        assert_eq!(tk_to_local(cs("UTC").as_ptr(), 0, &mut civil), TK_OK);
        assert_eq!((civil.year, civil.month, civil.day), (1970, 1, 1));

        // next_transition: TK_OK both ways; found flag carries the answer.
        let mut t: i64 = 0;
        let mut found: c_int = -1;
        let mut inf2 = TkInfo {
            offset_secs: 0,
            is_dst: 0,
            abbr: [0; 16],
        };
        assert_eq!(
            tk_next_transition(cs("Asia/Tokyo").as_ptr(), 0, &mut t, &mut inf2, &mut found),
            TK_OK
        );
        assert_eq!(found, 0);
        assert_eq!(
            tk_next_transition(
                cs("America/Chicago").as_ptr(),
                0,
                &mut t,
                &mut inf2,
                &mut found
            ),
            TK_OK
        );
        assert_eq!(found, 1);
        assert!(t > 0);
        tk_free_string(ptr::null_mut());
    }
}
