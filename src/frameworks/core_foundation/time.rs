
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Time things including `CFAbsoluteTime`.

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::CFTypeRef;
use crate::frameworks::foundation::NSTimeInterval;
use crate::libc::time::{time_t, timestamp_to_calendar_date};
use crate::mem::SafeRead;
use crate::objc::{id, nil, retain};
use crate::msg;
use crate::{impl_GuestRet_for_large_struct, Environment};
use std::ops::Add;
use std::time::{Duration, SystemTime};

/// Seconds between Unix and Apple's epochs
pub const SECS_FROM_UNIX_TO_APPLE_EPOCHS: u64 = 978_307_200;

/// The absolute reference date is 1 Jan 2001 00:00:00 GMT
pub fn apple_epoch() -> SystemTime {
    SystemTime::UNIX_EPOCH.add(Duration::from_secs(SECS_FROM_UNIX_TO_APPLE_EPOCHS))
}

pub type CFTimeInterval = NSTimeInterval;
pub type CFAbsoluteTime = CFTimeInterval;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct CFGregorianDate {
    pub year: i32,    // SInt32
    pub month: i8,    // SInt8
    pub day: i8,      // SInt8
    pub hours: i8,    // SInt8
    pub minutes: i8,  // SInt8
    pub seconds: f64, // double
}

unsafe impl SafeRead for CFGregorianDate {}
impl_GuestRet_for_large_struct!(CFGregorianDate);

/// Absolute time is measured in seconds relative to Jan 1 2001 GMT.
fn CFAbsoluteTimeGetCurrent(_env: &mut Environment) -> CFAbsoluteTime {
    SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64()
}

type CFTimeZoneRef = CFTypeRef;

fn CFTimeZoneCopySystem(_env: &mut Environment) -> CFTimeZoneRef {
    // TODO: implement. Null is treated as GMT by the date helpers below.
    nil
}

// Core Foundation and Foundation time-zone objects are toll-free bridged.
// The Copy rule requires the caller to receive an owned reference.
fn CFTimeZoneCopyDefault(env: &mut Environment) -> CFTimeZoneRef {
    let cls = env.objc.get_known_class("NSTimeZone", &mut env.mem);
    let tz: id = msg![env; cls defaultTimeZone];
    retain(env, tz)
}

pub fn CFAbsoluteTimeGetGregorianDate(
    env: &mut Environment,
    at: CFAbsoluteTime,
    tz: CFTimeZoneRef,
) -> CFGregorianDate {
    // A non-null CFTimeZoneRef is a valid input, not an assertion failure.
    // Use the time-zone's offset for the requested calendar conversion.
    // This is an approximation for historical/DST-dependent dates because
    // secondsFromGMT returns the current offset, not necessarily the offset at `at`.
    let offset_seconds: i32 = if tz.is_null() {
        0
    } else {
        msg![env; tz secondsFromGMT]
    };

    let unix_seconds = at + SECS_FROM_UNIX_TO_APPLE_EPOCHS as f64;
    let local_seconds = unix_seconds + f64::from(offset_seconds);
    let whole_seconds = local_seconds.floor() as time_t;
    let tm = timestamp_to_calendar_date(whole_seconds);
    CFGregorianDate {
        year: 1900 + tm.tm_year,
        month: (tm.tm_mon + 1) as i8,
        day: tm.tm_mday as i8,
        hours: tm.tm_hour as i8,
        minutes: tm.tm_min as i8,
        seconds: f64::from(tm.tm_sec) + local_seconds.rem_euclid(1.0),
    }
}

fn CFAbsoluteTimeGetDayOfWeek(env: &mut Environment, at: CFAbsoluteTime, tz: CFTimeZoneRef) -> i32 {
    // Keep the previous implementation's return convention for compatibility.
    CFAbsoluteTimeGetGregorianDate(env, at, tz).day.into()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFAbsoluteTimeGetCurrent()),
    export_c_func!(CFTimeZoneCopySystem()),
    export_c_func!(CFTimeZoneCopyDefault()),
    export_c_func!(CFAbsoluteTimeGetGregorianDate(_, _)),
    export_c_func!(CFAbsoluteTimeGetDayOfWeek(_, _)),
];
