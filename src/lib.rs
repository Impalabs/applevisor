#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "simd-nightly", feature(portable_simd), feature(simd_ffi))]

pub mod error;
#[cfg(feature = "macos-15-0")]
pub mod gic;
pub mod memory;
pub mod vcpu;
pub mod vm;

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

// -----------------------------------------------------------------------------------------------
// Macros
// -----------------------------------------------------------------------------------------------

/// Macro that calls an ffi hypervisor function and wraps the resulting return value in a
/// [`Result`].
macro_rules! hv_unsafe_call {
    ($x:expr) => {{
        let ret = unsafe { $x };
        match ret {
            x if x == hv_error_t::HV_SUCCESS as i32 => Ok(()),
            code => Err(HypervisorError::from(code)),
        }
    }};
}

pub(crate) use hv_unsafe_call;

// -----------------------------------------------------------------------------------------------
// Prelude
// -----------------------------------------------------------------------------------------------

/// The AppleVisor prelude.
pub mod prelude {
    pub use crate::error::*;
    #[cfg(feature = "macos-15-0")]
    pub use crate::gic::*;
    pub use crate::memory::*;
    pub use crate::vcpu::*;
    pub use crate::vm::*;
}

// -----------------------------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------------------------

#[cfg(test)]
static ALLOC_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(test)]
pub(crate) fn next_mem_addr() -> u64 {
    ALLOC_ID.fetch_add(1, Ordering::Relaxed) * crate::memory::PAGE_SIZE as u64
}
