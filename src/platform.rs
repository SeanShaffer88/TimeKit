//! Platform layer: just the clock. The timezone engine in lib.rs is
//! pure logic on every OS.

/// Current Unix time in seconds.
#[cfg(windows)]
pub fn now_unix() -> i64 {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemTimeAsFileTime(ft: *mut u64);
    }
    let mut ft: u64 = 0;
    unsafe { GetSystemTimeAsFileTime(&mut ft) };
    const EPOCH_DIFF_100NS: u64 = 116_444_736_000_000_000;
    if ft < EPOCH_DIFF_100NS {
        return 0;
    }
    ((ft - EPOCH_DIFF_100NS) / 10_000_000) as i64
}

/// Current Unix time in seconds.
#[cfg(not(windows))]
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
