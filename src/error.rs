//! Error objects used by this crate.

#[cfg(not(feature = "macos-12-1"))]
use std::alloc::LayoutError;

use applevisor_sys::*;

// -----------------------------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------------------------

/// Convenient Result type for hypervisor errors.
pub type Result<T> = std::result::Result<T, HypervisorError>;

/// The error type for hypervisor errors.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum HypervisorError {
    /// A bad argument was provided to the function called.
    BadArgument,
    /// The owning resource is busy.
    Busy,
    /// The operation was denied by the system.
    Denied,
    /// The operation was unsuccessful.
    Error,
    /// An hypervisor fault occured.
    Fault,
    /// The guest is in an illegal state.
    IllegalState,
    /// No VM or vCPU available.
    NoDevice,
    /// No host resources available to complete the request.
    NoResources,
    /// An unknown error type.
    Unknown(hv_return_t),
    /// The operation is not supported.
    Unsupported,
    /// A layout error occured during a memory allocation.
    ///
    /// This can only be returned by [`Mapping::new`] if feature
    /// [`macos-12-1`](#feature-macos-12-1) is disabled.
    #[cfg(not(feature = "macos-12-1"))]
    LayoutError,
}

impl HypervisorError {
    /// Returns a description for a given hypervisor error.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::BadArgument => "function call has an invalid argument",
            Self::Busy => "owning resource is busy",
            Self::Denied => "operation not allowed by the system",
            Self::Error => "operation unsuccessful",
            Self::Fault => "hypervisor fault",
            Self::IllegalState => "guest in an illegal state",
            Self::NoDevice => "no VM or vCPU available",
            Self::NoResources => "no host resources available to complete the request",
            Self::Unknown(_) => "unknown error",
            Self::Unsupported => "unsupported operation",
            #[cfg(not(feature = "macos-12-1"))]
            Self::LayoutError => "layout error",
        }
    }
}

impl From<hv_return_t> for HypervisorError {
    fn from(code: hv_return_t) -> Self {
        match code {
            x if x == hv_error_t::HV_BAD_ARGUMENT as hv_return_t => Self::BadArgument,
            x if x == hv_error_t::HV_BUSY as hv_return_t => Self::Busy,
            x if x == hv_error_t::HV_DENIED as hv_return_t => Self::Denied,
            x if x == hv_error_t::HV_ERROR as hv_return_t => Self::Error,
            x if x == hv_error_t::HV_FAULT as hv_return_t => Self::Fault,
            x if x == hv_error_t::HV_ILLEGAL_GUEST_STATE as hv_return_t => Self::IllegalState,
            x if x == hv_error_t::HV_NO_DEVICE as hv_return_t => Self::NoDevice,
            x if x == hv_error_t::HV_NO_RESOURCES as hv_return_t => Self::NoResources,
            x if x == hv_error_t::HV_UNSUPPORTED as hv_return_t => Self::Unsupported,
            _ => Self::Unknown(code),
        }
    }
}

#[cfg(not(feature = "macos-12-1"))]
impl From<LayoutError> for HypervisorError {
    fn from(_err: LayoutError) -> Self {
        HypervisorError::LayoutError
    }
}

impl From<HypervisorError> for hv_return_t {
    fn from(err: HypervisorError) -> Self {
        match err {
            HypervisorError::BadArgument => hv_error_t::HV_BAD_ARGUMENT as hv_return_t,
            HypervisorError::Busy => hv_error_t::HV_BUSY as hv_return_t,
            HypervisorError::Denied => hv_error_t::HV_DENIED as hv_return_t,
            HypervisorError::Error => hv_error_t::HV_ERROR as hv_return_t,
            HypervisorError::Fault => hv_error_t::HV_FAULT as hv_return_t,
            HypervisorError::IllegalState => hv_error_t::HV_ILLEGAL_GUEST_STATE as hv_return_t,
            HypervisorError::NoDevice => hv_error_t::HV_NO_DEVICE as hv_return_t,
            HypervisorError::NoResources => hv_error_t::HV_NO_RESOURCES as hv_return_t,
            HypervisorError::Unsupported => hv_error_t::HV_UNSUPPORTED as hv_return_t,
            HypervisorError::Unknown(code) => code,
            #[cfg(not(feature = "macos-12-1"))]
            HypervisorError::LayoutError => hv_error_t::HV_ERROR as hv_return_t,
        }
    }
}

impl std::error::Error for HypervisorError {}

impl std::fmt::Display for HypervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (error {:#08x})",
            self.as_str(),
            Into::<hv_return_t>::into(*self)
        )
    }
}

impl std::fmt::Debug for HypervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HypervisorError")
            .field("code", &Into::<hv_return_t>::into(*self))
            .field("description", &self.as_str())
            .finish()
    }
}
