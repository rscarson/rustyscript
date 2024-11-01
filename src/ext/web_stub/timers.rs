// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
//! This module helps deno implement timers and performance APIs.

use deno_core::op2;
use deno_core::OpState;
use std::time::Instant;

pub type StartTime = Instant;

// Returns a milliseconds and nanoseconds subsec
// since the start time of the deno runtime.
// If the High precision flag is not set, the
// nanoseconds are rounded on 2ms.
#[op2(fast)]
pub fn op_now(state: &mut OpState, #[buffer] buf: &mut [u8]) {
    let start_time = state.borrow::<StartTime>();
    let elapsed = start_time.elapsed();
    let seconds = elapsed.as_secs();
    let mut subsec_nanos = elapsed.subsec_nanos();

    let reduced_time_precision = 2_000_000; // 2ms in nanoseconds
    subsec_nanos -= subsec_nanos % reduced_time_precision;
    if buf.len() < 8 {
        return;
    }
    let buf: &mut [u32] =
    // SAFETY: buffer is at least 8 bytes long.
    unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast(), 2) };
    buf[0] = u32::try_from(seconds).unwrap_or_default();
    buf[1] = subsec_nanos;
}

#[allow(clippy::unused_async)]
#[op2(async(lazy), fast)]
pub async fn op_defer() {}
