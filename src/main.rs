// dormire - A modern replacement for `sleep`
// Copyright (C) 2026  anoter43 <74D756F4B3EAF32E6F1294928D214D55C4F7479D>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDateTime, NaiveTime, TimeZone};
use indicatif::{ProgressBar, ProgressStyle};

const USAGE: &str = "Usage:
  dormire <duration>...          Sleep like GNU sleep (e.g. 1.5 2s 3m 1h 1d)
  dormire --until <time>         Sleep until a time (HH:MM[:SS] or \"YYYY-MM-DD HH:MM[:SS]\")
  dormire --pid <pid>            Sleep until the given process exits";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("{USAGE}");
        return ExitCode::from(if args.is_empty() { 1 } else { 0 });
    }

    let code = match args[0].as_str() {
        "--until" => {
            let Some(t) = args.get(1) else {
                eprintln!("--until requires a time argument\n{USAGE}");
                return ExitCode::from(1);
            };
            match until_time(t) {
                Ok(d) => sleep_with_progress(d, &format!("until {t}")),
                Err(e) => {
                    eprintln!("dormire: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "--pid" => {
            let Some(p) = args.get(1) else {
                eprintln!("--pid requires a PID argument\n{USAGE}");
                return ExitCode::from(1);
            };
            match p.parse::<libc::pid_t>() {
                Ok(pid) if pid > 0 => wait_for_pid(pid),
                _ => {
                    eprintln!("dormire: invalid PID: {p}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            let mut total = Duration::ZERO;
            for a in &args {
                match parse_duration(a) {
                    Ok(d) => total += d,
                    Err(e) => {
                        eprintln!("dormire: {e}");
                        return ExitCode::from(1);
                    }
                }
            }
            sleep_with_progress(total, &format_duration(total))
        }
    };
    code
}

/// GNU-sleep style duration: number with optional suffix s/m/h/d (default s).
fn parse_duration(s: &str) -> Result<Duration, String> {
    let (num, mult) = match s.chars().last() {
        Some('s') => (&s[..s.len() - 1], 1.0),
        Some('m') => (&s[..s.len() - 1], 60.0),
        Some('h') => (&s[..s.len() - 1], 3600.0),
        Some('d') => (&s[..s.len() - 1], 86400.0),
        _ => (s, 1.0),
    };
    let secs: f64 = num
        .parse()
        .map_err(|_| format!("invalid time interval '{s}'"))?;
    if secs < 0.0 || !secs.is_finite() {
        return Err(format!("invalid time interval '{s}'"));
    }
    Ok(Duration::from_secs_f64(secs * mult))
}

/// Parse "HH:MM[:SS]" (today, or tomorrow if already past) or
/// "YYYY-MM-DD HH:MM[:SS]" local time, and return the wait duration.
fn until_time(s: &str) -> Result<Duration, String> {
    let now = Local::now();
    let target = if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M"))
    {
        let today = now.date_naive().and_time(t);
        let day = if today <= now.naive_local() {
            now.date_naive().succ_opt().unwrap()
        } else {
            now.date_naive()
        };
        day.and_time(t)
    } else if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M"))
    {
        dt
    } else {
        return Err(format!("invalid --until time '{s}' (use HH:MM[:SS] or \"YYYY-MM-DD HH:MM[:SS]\")"));
    };

    let target = Local
        .from_local_datetime(&target)
        .single()
        .ok_or_else(|| format!("time '{s}' is ambiguous (DST transition)"))?;
    let delta = target - now;
    if delta.num_seconds() <= 0 {
        return Err(format!("time '{s}' is in the past"));
    }
    Ok(delta.to_std().unwrap())
}

fn sleep_with_progress(total: Duration, label: &str) -> ExitCode {
    if total.is_zero() {
        return ExitCode::SUCCESS;
    }
    let bar = make_bar(total.as_millis() as u64, label);
    let start = Instant::now();
    let tick = Duration::from_millis(50);
    loop {
        let elapsed = start.elapsed();
        if elapsed >= total {
            break;
        }
        bar.set_position(elapsed.as_millis() as u64);
        sleep(tick.min(total - elapsed));
    }
    bar.finish_with_message(format!("slept {label}"));
    ExitCode::SUCCESS
}

fn wait_for_pid(pid: libc::pid_t) -> ExitCode {
    if !pid_alive(pid) {
        eprintln!("dormire: no such process: {pid}");
        return ExitCode::from(1);
    }
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner} waiting for PID {msg} [{elapsed_precise}]")
            .unwrap(),
    );
    spinner.set_message(pid.to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));
    while pid_alive(pid) {
        sleep(Duration::from_millis(200));
    }
    spinner.finish_with_message(format!("PID {pid} exited"));
    ExitCode::SUCCESS
}

#[cfg(target_os = "macos")]
fn errno() -> i32 {
    unsafe { *libc::__error() }
}

#[cfg(target_os = "linux")]
fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

fn pid_alive(pid: libc::pid_t) -> bool {
    // kill(pid, 0) performs error checking without sending a signal.
    unsafe { libc::kill(pid, 0) == 0 || errno() == libc::EPERM }
}

fn make_bar(len_ms: u64, label: &str) -> ProgressBar {
    let bar = ProgressBar::new(len_ms);
    bar.set_style(
        ProgressStyle::with_template(
            "{bar:40.cyan/blue} {percent:>3}% {elapsed_precise}/{duration_precise} ({msg})",
        )
        .unwrap(),
    );
    bar.set_message(label.to_string());
    bar
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    match secs {
        s if s % 86400 == 0 && s > 0 => format!("{}d", s / 86400),
        s if s % 3600 == 0 && s > 0 => format!("{}h", s / 3600),
        s if s % 60 == 0 && s > 0 => format!("{}m", s / 60),
        _ => format!("{:.1}s", d.as_secs_f64()),
    }
}
