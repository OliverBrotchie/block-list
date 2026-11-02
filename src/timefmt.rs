/// Local wall clock time, from the libc locale.
fn local_time(epoch: u64) -> libc::tm {
    let time = epoch as libc::time_t;
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    unsafe {
        libc::localtime_r(&time, &mut local);
    }
    local
}

/// Local `HH:MM`.
pub fn local_hhmm(epoch: u64) -> String {
    let local = local_time(epoch);
    format!("{:02}:{:02}", local.tm_hour, local.tm_min)
}

/// Local `YYYY-MM-DD HH:MM`.
pub fn local_datetime(epoch: u64) -> String {
    let local = local_time(epoch);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        local.tm_year + 1900,
        local.tm_mon + 1,
        local.tm_mday,
        local.tm_hour,
        local.tm_min
    )
}

/// A short duration: `45m` or `1h30m`.
pub fn human_duration(seconds: u64) -> String {
    let minutes = seconds.div_ceil(60);
    if minutes >= 60 {
        format!("{}h{}m", minutes / 60, minutes % 60)
    } else {
        format!("{minutes}m")
    }
}
