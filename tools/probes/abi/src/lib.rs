use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

pub type Callback = Option<extern "C" fn(context: *mut c_void, value: i32) -> i32>;

#[repr(C)]
pub struct Probe {
    callback: extern "C" fn(context: *mut c_void, value: i32) -> i32,
    context: *mut c_void,
}

const OK: i32 = 0;
const INVALID: i32 = 1;
const PANIC: i32 = 3;

#[no_mangle]
pub extern "C" fn oot_abi_probe_create(
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
        let Some(callback) = callback else { return INVALID };
        let probe = Box::new(Probe { callback, context });
        unsafe { *out_probe = Box::into_raw(probe) };
        OK
    })
}

#[no_mangle]
pub extern "C" fn oot_abi_probe_call(probe: *mut Probe, value: i32, out_value: *mut i32) -> i32 {
    guarded(|| {
        if probe.is_null() || out_value.is_null() { return INVALID; }
        let p = unsafe { &*probe };
        let value = (p.callback)(p.context, value);
        unsafe { *out_value = value };
        OK
    })
}

#[no_mangle]
pub extern "C" fn oot_abi_probe_shutdown(probe: *mut *mut Probe) -> i32 {
    guarded(|| {
        if probe.is_null() { return INVALID; }
        let raw = unsafe { *probe };
        if raw.is_null() { return INVALID; }
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
    fn layout_is_c_compatible_pointer_sized() {
        assert_eq!(std::mem::size_of::<Probe>(), 2 * std::mem::size_of::<usize>());
    }
}
