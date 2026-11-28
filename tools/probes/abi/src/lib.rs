use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

pub type Callback = Option<unsafe extern "C" fn(context: *mut c_void, value: i32) -> i32>;

#[repr(C)]
pub struct Probe {
    callback: unsafe extern "C" fn(context: *mut c_void, value: i32) -> i32,
    context: *mut c_void,
    scratch: [u8; SCRATCH_SIZE],
}

const OK: i32 = 0;
const INVALID: i32 = 1;
const PANIC: i32 = 3;
const SCRATCH_SIZE: usize = 64;

/// # Safety
/// A non-null output slot must point to writable, aligned storage containing an
/// initialized pointer. A supplied callback and its borrowed context must remain
/// valid for the lifetime of the probe.
#[no_mangle]
pub unsafe extern "C" fn oot_abi_probe_create(
    callback: Callback,
    context: *mut c_void,
    out_probe: *mut *mut Probe,
) -> i32 {
    guarded(|| {
        if out_probe.is_null() {
            return INVALID;
        }
        if !unsafe { *out_probe }.is_null() {
            return INVALID;
        }
        let Some(callback) = callback else {
            return INVALID;
        };
        let probe = Box::new(Probe {
            callback,
            context,
            scratch: [0; SCRATCH_SIZE],
        });
        unsafe { *out_probe = Box::into_raw(probe) };
        OK
    })
}

/// Copy bytes from the probe-owned scratch area. Zero-byte reads permit a null
/// destination; nonzero reads require a writable destination of `count` bytes.
/// The probe must be live, and calls must be serialized with other probe calls.
///
/// # Safety
/// `probe` must be a live handle created by `oot_abi_probe_create`. For nonzero
/// counts, `destination` must be writable for `count` bytes and must not overlap
/// the probe allocation. The destination must also be properly aligned for `u8`.
#[no_mangle]
pub unsafe extern "C" fn oot_abi_probe_read(
    probe: *mut Probe,
    offset: u64,
    count: u64,
    destination: *mut u8,
) -> i32 {
    guarded(|| {
        let Some((start, end, count)) = checked_range(offset, count) else {
            return INVALID;
        };
        if probe.is_null() || (count != 0 && destination.is_null()) {
            return INVALID;
        }
        if count != 0 {
            let p = unsafe { &*probe };
            unsafe { ptr::copy_nonoverlapping(p.scratch[start..end].as_ptr(), destination, count) };
        }
        OK
    })
}

/// Copy bytes into the probe-owned scratch area. Zero-byte writes permit a null
/// source. A rejected range leaves the entire scratch area unchanged.
///
/// # Safety
/// `probe` must be a live handle created by `oot_abi_probe_create`. For nonzero
/// counts, `source` must be readable for `count` bytes and must not overlap the
/// probe allocation. The source must also be properly aligned for `u8`.
#[no_mangle]
pub unsafe extern "C" fn oot_abi_probe_write(
    probe: *mut Probe,
    offset: u64,
    count: u64,
    source: *const u8,
) -> i32 {
    guarded(|| {
        let Some((start, end, count)) = checked_range(offset, count) else {
            return INVALID;
        };
        if probe.is_null() || (count != 0 && source.is_null()) {
            return INVALID;
        }
        if count != 0 {
            let p = unsafe { &mut *probe };
            unsafe { ptr::copy_nonoverlapping(source, p.scratch[start..end].as_mut_ptr(), count) };
        }
        OK
    })
}

fn checked_range(offset: u64, count: u64) -> Option<(usize, usize, usize)> {
    let end = offset.checked_add(count)?;
    if end > SCRATCH_SIZE as u64 {
        return None;
    }
    Some((offset as usize, end as usize, count as usize))
}

/// # Safety
/// Non-null arguments must be a live probe from create and an aligned, writable
/// result slot outside its storage. The probe must remain live throughout the
/// call. Calls may not overlap or reenter with the same probe.
#[no_mangle]
pub unsafe extern "C" fn oot_abi_probe_call(
    probe: *mut Probe,
    value: i32,
    out_value: *mut i32,
) -> i32 {
    guarded(|| {
        if probe.is_null() || out_value.is_null() {
            return INVALID;
        }
        let p = unsafe { &*probe };
        let value = unsafe { (p.callback)(p.context, value) };
        unsafe { *out_value = value };
        OK
    })
}

/// # Safety
/// A non-null pointer slot must be aligned, initialized and writable, and must
/// be outside the probe allocation. It must contain null or the sole owning
/// pointer from create, with no active calls on that probe.
#[no_mangle]
pub unsafe extern "C" fn oot_abi_probe_shutdown(probe: *mut *mut Probe) -> i32 {
    guarded(|| {
        if probe.is_null() {
            return INVALID;
        }
        let raw = unsafe { *probe };
        if raw.is_null() {
            return INVALID;
        }
        unsafe {
            drop(Box::from_raw(raw));
            *probe = ptr::null_mut();
        }
        OK
    })
}

fn guarded(f: impl FnOnce() -> i32) -> i32 {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(PANIC)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scratch_ranges_are_bounded_without_overflow() {
        assert_eq!(checked_range(62, 2), Some((62, 64, 2)));
        assert_eq!(checked_range(64, 0), Some((64, 64, 0)));
        assert_eq!(checked_range(65, 0), None);
        assert_eq!(checked_range(63, 2), None);
        assert_eq!(checked_range(u64::MAX, 1), None);
    }
}
