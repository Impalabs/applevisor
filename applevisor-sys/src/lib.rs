//! Unsafe Rust bindings for the Apple Silicon Hypervisor.framework
//!
//! These unsafe bindings provide access to the Apple Silicon `Hypervisor.framework` from Rust
//! programs. It is recommended to use the safe version of this library available at the following
//! locations:
//!
//!  * [Applevisor GitHub repository](https://github.com/impalabs/applevisor)
//!  * [Applevisor crates.io page](https://crates.io/crates/applevisor)
//!  * [Applevisor docs.rs page](https://docs.rs/applevisor)
#![feature(portable_simd)]
#![feature(simd_ffi)]
#![allow(non_camel_case_types)]
#![allow(improper_ctypes)]

use core::ffi::c_void;

#[cfg_attr(target_os = "macos", link(name = "Hypervisor", kind = "framework"))]
extern "C" {}

/// The return type of framework functions.
pub type hv_return_t = i32;

/// Errors returned by Hypervisor functions.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_error_t {
    /// The operation completed successfully.
    HV_SUCCESS = 0,
    /// The operation was unsuccessful.
    HV_ERROR = 0xfae94001,
    /// The operation was unsuccessful because the owning resource was busy.
    HV_BUSY = 0xfae94002,
    /// The operation was unsuccessful because the function call had an invalid argument.
    HV_BAD_ARGUMENT = 0xfae94003,
    /// The operation was unsuccessful because the guest is in an illegal state.
    HV_ILLEGAL_GUEST_STATE = 0xfae94004,
    /// The operation was unsuccessful because the host had no resources available to complete the
    /// request.
    HV_NO_RESOURCES = 0xfae94005,
    /// The operation was unsuccessful because no VM or vCPU was available.
    HV_NO_DEVICE = 0xfae94006,
    /// The system didn’t allow the requested operation.
    HV_DENIED = 0xfae94007,
    /// HV_FAULT
    HV_FAULT = 0xfae94008,
    /// The operation requested isn’t supported by the hypervisor.
    HV_UNSUPPORTED = 0xfae9400f,
}

// -----------------------------------------------------------------------------------------------
// Virtual Machine Management
// -----------------------------------------------------------------------------------------------

/// The type that defines a virtual-machine configuration.
pub type hv_vm_config_t = *mut c_void;

extern "C" {
    /// Creates a VM instance for the current process.
    ///
    /// # Parameters
    ///
    /// * `config`: The configuration of the vCPU, which must be nil.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vm_create(config: hv_vm_config_t) -> hv_return_t;

    /// Destroys the VM instance associated with the current process.
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vm_destroy() -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - Configuration
// -----------------------------------------------------------------------------------------------

/// The type that defines a vCPU configuration.
pub type hv_vcpu_config_t = *mut c_void;

/// The type that defines feature registers.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_feature_reg_t {
    /// The value that identifies debug feature register 0, EL1 (DFR0_EL1).
    HV_FEATURE_REG_ID_AA64DFR0_EL1,
    /// The value that identifies debug feature register 1, EL1 (DFR1_EL1).
    HV_FEATURE_REG_ID_AA64DFR1_EL1,
    /// The value that identifies instruction set attribute register 0, EL1 (ISAR0_EL1).
    HV_FEATURE_REG_ID_AA64ISAR0_EL1,
    /// The value that identifies instruction set attribute register 1, EL1 (ISAR_EL1).
    HV_FEATURE_REG_ID_AA64ISAR1_EL1,
    /// The value that identifies memory model feature register 0, EL1(MMFR0_EL1).
    HV_FEATURE_REG_ID_AA64MMFR0_EL1,
    /// The value that identifies memory model feature register 1, EL1 (MMFR1_EL1).
    HV_FEATURE_REG_ID_AA64MMFR1_EL1,
    /// The value that identifies memory model feature register 2, EL1 (MMFR2_EL1).
    HV_FEATURE_REG_ID_AA64MMFR2_EL1,
    /// The value that identifies processor feature register 0, EL1 (PFR0_EL1).
    HV_FEATURE_REG_ID_AA64PFR0_EL1,
    /// The value that identifies processor feature register 1, EL1 (PFR1_EL1).
    HV_FEATURE_REG_ID_AA64PFR1_EL1,
    /// The value that describes Cache Type Register, EL0.
    HV_FEATURE_REG_CTR_EL0,
    /// The value that describes Cache Level ID Register, EL1.
    HV_FEATURE_REG_CLIDR_EL1,
    /// The values that describes Data Cache Zero ID Register, EL0.
    HV_FEATURE_REG_DCZID_EL0,
}

/// The structure that describes an instruction or data cache element.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_cache_type_t {
    /// The value that describes a cached data value.
    HV_CACHE_TYPE_DATA,
    /// The value that describes a cached instuction value.
    HV_CACHE_TYPE_INSTRUCTION,
}

extern "C" {
    /// Creates a vCPU configuration object.
    ///
    /// # Return
    ///
    /// A new vCPU configuration object.
    pub fn hv_vcpu_config_create() -> hv_vcpu_config_t;

    /// Gets the value of a feature register.
    ///
    /// # Parameters
    ///
    /// * `config`: The vCPU configuration.
    /// * `feature_reg`: The ID of the feature register.
    /// * `value`: The value of `feature_reg` on output. Undefined if the call doesn’t succeed.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_config_get_feature_reg(
        config: hv_vcpu_config_t,
        feature_reg: hv_feature_reg_t,
        value: *mut u64,
    ) -> hv_return_t;

    /// Returns the Cache Size ID Register (CCSIDR_EL1) values for the vCPU configuration and
    /// cache type you specify.
    ///
    /// # Parameters
    ///
    /// * `config`: The vCPU configuration.
    /// * `cache_type`: The cache type from the available [`hv_cache_type_t`] types.
    /// * `values`: A pointer to the location for the return values.
    ///
    /// # Return Value
    ///
    /// A [`hv_return_t`] value that indicates that result of the function.
    pub fn hv_vcpu_config_get_ccsidr_el1_sys_reg_values(
        config: hv_vcpu_config_t,
        cache_type: hv_cache_type_t,
        values: *mut u64,
    ) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - Creation and Destruction
// -----------------------------------------------------------------------------------------------

/// An opaque value that represents a vCPU instance.
pub type hv_vcpu_t = u64;

/// The structure that describes information about an exit from the virtual CPU (vCPU) to the host.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct hv_vcpu_exit_exception_t {
    /// The vCPU exception syndrome causing the exception.
    pub syndrome: u64,
    /// The vCPU virtual address of the exception.
    pub virtual_address: u64,
    /// The intermediate physical address of the exception in the client.
    pub physical_address: u64,
}

/// The type that describes the event that triggered a guest exit to the host.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_exit_reason_t {
    /// The value that identifies exits requested by exit handler on the host.
    HV_EXIT_REASON_CANCELED,
    /// The value that identifies traps caused by the guest operations.
    HV_EXIT_REASON_EXCEPTION,
    /// The value that identifies when the virtual timer enters the pending state.
    HV_EXIT_REASON_VTIMER_ACTIVATED,
    /// The value that identifies unexpected exits.
    HV_EXIT_REASON_UNKNOWN,
}

/// Information about an exit from the vCPU to the host.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct hv_vcpu_exit_t {
    /// Information about an exit from the vcpu to the host.
    pub reason: hv_exit_reason_t,
    /// Information about an exit exception from the vcpu to the host.
    pub exception: hv_vcpu_exit_exception_t,
}

extern "C" {
    /// Returns the maximum number of vCPUs that the hypervisor supports.
    ///
    /// # Parameters
    ///
    /// * `max_vcpu_count`: The maximum number of vCPUs on output. Undefined if the call doesn’t
    ///                     succeed.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vm_get_max_vcpu_count(max_vcpu_count: *mut u32) -> hv_return_t;

    /// Creates a vCPU instance for the current thread.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: An argument that the hypervisor populates with the instance of a vCPU on a
    ///           successful return.
    /// * `exit`: The pointer to the vCPU exit information. The function hv_vcpu_run updates this
    ///           structure on return.
    /// * `config`: The configuration of the vCPU or nil for a default configuration.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_create(
        vcpu: *mut hv_vcpu_t,
        exit: *mut *const hv_vcpu_exit_t,
        config: hv_vcpu_config_t,
    ) -> hv_return_t;

    /// Destroys the vCPU instance associated with the current thread.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The instance of the vCPU.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_destroy(vcpu: hv_vcpu_t) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - Runtime
// -----------------------------------------------------------------------------------------------

/// The type that defines the vCPU’s interrupts.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_interrupt_type_t {
    /// ARM Fast Interrupt Request.
    HV_INTERRUPT_TYPE_FIQ,
    /// ARM Interrupt Request.
    HV_INTERRUPT_TYPE_IRQ,
}

extern "C" {
    /// Starts the execution of a vCPU.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The instance of the vCPU.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_run(vcpu: hv_vcpu_t) -> hv_return_t;

    /// Forces an immediate exit of a set of vCPUs of the VM.
    ///
    /// # Parameters
    ///
    /// * `vcpus`: An array of vCPU instances.
    /// * `vcpu_count`: The number of vCPUs in the array.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpus_exit(vcpus: *const hv_vcpu_t, vcpu_count: u32) -> hv_return_t;

    /// Gets pending interrupts for a vCPU.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The instance of the vCPU.
    /// * `type`: The interrupt from Interrupt Constants.
    /// * `pending`: A variable that indicates whether, on output, the interrupt of type is
    ///              pending or not.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_pending_interrupt(
        vcpu: hv_vcpu_t,
        _type: hv_interrupt_type_t,
        pending: *mut bool,
    ) -> hv_return_t;

    /// Sets pending interrupts for a vCPU.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The instance of the vCPU.
    /// * `type`: The interrupt from Interrupt Constants.
    /// * `pending`: A Boolean that indicates whether the interrupt is pending.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_set_pending_interrupt(
        vcpu: hv_vcpu_t,
        _type: hv_interrupt_type_t,
        pending: bool,
    ) -> hv_return_t;

    /// Returns, by reference, the cumulative execution time of a vCPU, in nanoseconds.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The instance of the vCPU.
    /// * `time`: The execution time on output, in nanoseconds.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_exec_time(vcpu: hv_vcpu_t, time: *mut u64) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - General Registers
// -----------------------------------------------------------------------------------------------

/// The type that defines general registers.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_reg_t {
    /// The value that identifies register X0.
    HV_REG_X0,
    /// The value that identifies register X1.
    HV_REG_X1,
    /// The value that identifies register X2.
    HV_REG_X2,
    /// The value that identifies register X3.
    HV_REG_X3,
    /// The value that identifies register X4.
    HV_REG_X4,
    /// The value that identifies register X5.
    HV_REG_X5,
    /// The value that identifies register X6.
    HV_REG_X6,
    /// The value that identifies register X7.
    HV_REG_X7,
    /// The value that identifies register X8.
    HV_REG_X8,
    /// The value that identifies register X9.
    HV_REG_X9,
    /// The value that identifies register X10.
    HV_REG_X10,
    /// The value that identifies register X11.
    HV_REG_X11,
    /// The value that identifies register X12.
    HV_REG_X12,
    /// The value that identifies register X13.
    HV_REG_X13,
    /// The value that identifies register X14.
    HV_REG_X14,
    /// The value that identifies register X15.
    HV_REG_X15,
    /// The value that identifies register X16.
    HV_REG_X16,
    /// The value that identifies register X17.
    HV_REG_X17,
    /// The value that identifies register X18.
    HV_REG_X18,
    /// The value that identifies register X19.
    HV_REG_X19,
    /// The value that identifies register X20.
    HV_REG_X20,
    /// The value that identifies register X21.
    HV_REG_X21,
    /// The value that identifies register X22.
    HV_REG_X22,
    /// The value that identifies register X23.
    HV_REG_X23,
    /// The value that identifies register X24.
    HV_REG_X24,
    /// The value that identifies register X25.
    HV_REG_X25,
    /// The value that identifies register X26.
    HV_REG_X26,
    /// The value that identifies register X27.
    HV_REG_X27,
    /// The value that identifies register X28.
    HV_REG_X28,
    /// The value that identifies register X29.
    HV_REG_X29,
    /// The value that identifies register X30.
    HV_REG_X30,
    /// The value that identifies the program counter (PC).
    HV_REG_PC,
    /// The value that identifies the floating-point control register (FPCR).
    HV_REG_FPCR,
    /// The value that identifies the floating-point status register (FPSR).
    HV_REG_FPSR,
    /// The value that identifies the current program status register (CPSR).
    HV_REG_CPSR,
}

impl hv_reg_t {
    /// The value that identifies the frame pointer (FP).
    pub const HV_REG_FP: Self = Self::HV_REG_X29;
    /// The value that identifies the link register (LR).
    pub const HV_REG_LR: Self = Self::HV_REG_X30;
}

extern "C" {
    /// Gets the current value of a vCPU register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `reg`: The ID of the general register.
    /// * `value`: The value of the register reg on output.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_reg(vcpu: hv_vcpu_t, reg: hv_reg_t, value: *mut u64) -> hv_return_t;

    /// Sets the value of a vCPU register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `reg`: The ID of the general register.
    /// * `value`: The new value of the register.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_set_reg(vcpu: hv_vcpu_t, reg: hv_reg_t, value: u64) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - SIMD & Floating-Point Registers
// -----------------------------------------------------------------------------------------------

/// The value that represents an ARM SIMD and FP register.
pub type hv_simd_fp_uchar16_t = std::simd::i8x16;

/// The type that defines SIMD and floating-point registers.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_simd_fp_reg_t {
    /// The value representing SIMD register Q0.
    HV_SIMD_FP_REG_Q0,
    /// The value representing SIMD register Q1.
    HV_SIMD_FP_REG_Q1,
    /// The value representing SIMD register Q2.
    HV_SIMD_FP_REG_Q2,
    /// The value representing SIMD register Q3.
    HV_SIMD_FP_REG_Q3,
    /// The value representing SIMD register Q4.
    HV_SIMD_FP_REG_Q4,
    /// The value representing SIMD register Q5.
    HV_SIMD_FP_REG_Q5,
    /// The value representing SIMD register Q6.
    HV_SIMD_FP_REG_Q6,
    /// The value representing SIMD register Q7.
    HV_SIMD_FP_REG_Q7,
    /// The value representing SIMD register Q8.
    HV_SIMD_FP_REG_Q8,
    /// The value representing SIMD register Q9.
    HV_SIMD_FP_REG_Q9,
    /// The value representing SIMD register Q10.
    HV_SIMD_FP_REG_Q10,
    /// The value representing SIMD register Q11.
    HV_SIMD_FP_REG_Q11,
    /// The value representing SIMD register Q12.
    HV_SIMD_FP_REG_Q12,
    /// The value representing SIMD register Q13.
    HV_SIMD_FP_REG_Q13,
    /// The value representing SIMD register Q14.
    HV_SIMD_FP_REG_Q14,
    /// The value representing SIMD register Q15.
    HV_SIMD_FP_REG_Q15,
    /// The value representing SIMD register Q16.
    HV_SIMD_FP_REG_Q16,
    /// The value representing SIMD register Q17.
    HV_SIMD_FP_REG_Q17,
    /// The value representing SIMD register Q18.
    HV_SIMD_FP_REG_Q18,
    /// The value representing SIMD register Q19.
    HV_SIMD_FP_REG_Q19,
    /// The value representing SIMD register Q20.
    HV_SIMD_FP_REG_Q20,
    /// The value representing SIMD register Q21.
    HV_SIMD_FP_REG_Q21,
    /// The value representing SIMD register Q22.
    HV_SIMD_FP_REG_Q22,
    /// The value representing SIMD register Q23.
    HV_SIMD_FP_REG_Q23,
    /// The value representing SIMD register Q24.
    HV_SIMD_FP_REG_Q24,
    /// The value representing SIMD register Q25.
    HV_SIMD_FP_REG_Q25,
    /// The value representing SIMD register Q26.
    HV_SIMD_FP_REG_Q26,
    /// The value representing SIMD register Q27.
    HV_SIMD_FP_REG_Q27,
    /// The value representing SIMD register Q28.
    HV_SIMD_FP_REG_Q28,
    /// The value representing SIMD register Q29.
    HV_SIMD_FP_REG_Q29,
    /// The value representing SIMD register Q30.
    HV_SIMD_FP_REG_Q30,
    /// The value representing SIMD register Q31.
    HV_SIMD_FP_REG_Q31,
}

extern "C" {
    /// Gets the current value of a vCPU SIMD and FP register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `reg`: The ID of the SIMD and FP register.
    /// * `value`: The value of the register reg on output.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_simd_fp_reg(
        vcpu: hv_vcpu_t,
        reg: hv_simd_fp_reg_t,
        value: *mut hv_simd_fp_uchar16_t,
    ) -> hv_return_t;

    /// Sets the value of a vCPU SIMD&FP register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `reg`: The ID of the SIMD and FP register.
    /// * `value`: The new value of the register.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_set_simd_fp_reg(
        vcpu: hv_vcpu_t,
        reg: hv_simd_fp_reg_t,
        value: hv_simd_fp_uchar16_t,
    ) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - System Registers
// -----------------------------------------------------------------------------------------------

/// The type of system registers.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_sys_reg_t {
    /// The value that represents the system register DBGBVR0_EL1.
    HV_SYS_REG_DBGBVR0_EL1 = 0x8004,
    /// The value that represents the system register DBGBCR0_EL1.
    HV_SYS_REG_DBGBCR0_EL1 = 0x8005,
    /// The value that represents the system register DBGWVR0_EL1.
    HV_SYS_REG_DBGWVR0_EL1 = 0x8006,
    /// The value that represents the system register DBGWCR0_EL1.
    HV_SYS_REG_DBGWCR0_EL1 = 0x8007,
    /// The value that represents the system register DBGBVR1_EL1.
    HV_SYS_REG_DBGBVR1_EL1 = 0x800c,
    /// The value that represents the system register DBGBCR1_EL1.
    HV_SYS_REG_DBGBCR1_EL1 = 0x800d,
    /// The value that represents the system register DBGWVR1_EL1.
    HV_SYS_REG_DBGWVR1_EL1 = 0x800e,
    /// The value that represents the system register DBGWCR1_EL1.
    HV_SYS_REG_DBGWCR1_EL1 = 0x800f,
    /// The value that represents the system register MDCCINT_EL1.
    HV_SYS_REG_MDCCINT_EL1 = 0x8010,
    /// The value that represents the system register MDSCR_EL1.
    HV_SYS_REG_MDSCR_EL1 = 0x8012,
    /// The value that represents the system register DBGBVR2_EL1.
    HV_SYS_REG_DBGBVR2_EL1 = 0x8014,
    /// The value that represents the system register DBGBCR2_EL1.
    HV_SYS_REG_DBGBCR2_EL1 = 0x8015,
    /// The value that represents the system register DBGWVR2_EL1.
    HV_SYS_REG_DBGWVR2_EL1 = 0x8016,
    /// The value that represents the system register DBGWCR2_EL1.
    HV_SYS_REG_DBGWCR2_EL1 = 0x8017,
    /// The value that represents the system register DBGBVR3_EL1.
    HV_SYS_REG_DBGBVR3_EL1 = 0x801c,
    /// The value that represents the system register DBGBCR3_EL1.
    HV_SYS_REG_DBGBCR3_EL1 = 0x801d,
    /// The value that represents the system register DBGWVR3_EL1.
    HV_SYS_REG_DBGWVR3_EL1 = 0x801e,
    /// The value that represents the system register DBGWCR3_EL1.
    HV_SYS_REG_DBGWCR3_EL1 = 0x801f,
    /// The value that represents the system register DBGBVR4_EL1.
    HV_SYS_REG_DBGBVR4_EL1 = 0x8024,
    /// The value that represents the system register DBGBCR4_EL1.
    HV_SYS_REG_DBGBCR4_EL1 = 0x8025,
    /// The value that represents the system register DBGWVR4_EL1.
    HV_SYS_REG_DBGWVR4_EL1 = 0x8026,
    /// The value that represents the system register DBGWCR4_EL1.
    HV_SYS_REG_DBGWCR4_EL1 = 0x8027,
    /// The value that represents the system register DBGBVR5_EL1.
    HV_SYS_REG_DBGBVR5_EL1 = 0x802c,
    /// The value that represents the system register DBGBCR5_EL1.
    HV_SYS_REG_DBGBCR5_EL1 = 0x802d,
    /// The value that represents the system register DBGWVR5_EL1.
    HV_SYS_REG_DBGWVR5_EL1 = 0x802e,
    /// The value that represents the system register DBGWCR5_EL1.
    HV_SYS_REG_DBGWCR5_EL1 = 0x802f,
    /// The value that represents the system register DBGBVR6_EL1.
    HV_SYS_REG_DBGBVR6_EL1 = 0x8034,
    /// The value that represents the system register DBGBCR6_EL1.
    HV_SYS_REG_DBGBCR6_EL1 = 0x8035,
    /// The value that represents the system register DBGWVR6_EL1.
    HV_SYS_REG_DBGWVR6_EL1 = 0x8036,
    /// The value that represents the system register DBGWCR6_EL1.
    HV_SYS_REG_DBGWCR6_EL1 = 0x8037,
    /// The value that represents the system register DBGBVR7_EL1.
    HV_SYS_REG_DBGBVR7_EL1 = 0x803c,
    /// The value that represents the system register DBGBCR7_EL1.
    HV_SYS_REG_DBGBCR7_EL1 = 0x803d,
    /// The value that represents the system register DBGWVR7_EL1.
    HV_SYS_REG_DBGWVR7_EL1 = 0x803e,
    /// The value that represents the system register DBGWCR7_EL1.
    HV_SYS_REG_DBGWCR7_EL1 = 0x803f,
    /// The value that represents the system register DBGBVR8_EL1.
    HV_SYS_REG_DBGBVR8_EL1 = 0x8044,
    /// The value that represents the system register DBGBCR8_EL1.
    HV_SYS_REG_DBGBCR8_EL1 = 0x8045,
    /// The value that represents the system register DBGWVR8_EL1.
    HV_SYS_REG_DBGWVR8_EL1 = 0x8046,
    /// The value that represents the system register DBGWCR8_EL1.
    HV_SYS_REG_DBGWCR8_EL1 = 0x8047,
    /// The value that represents the system register DBGBVR9_EL1.
    HV_SYS_REG_DBGBVR9_EL1 = 0x804c,
    /// The value that represents the system register DBGBCR9_EL1.
    HV_SYS_REG_DBGBCR9_EL1 = 0x804d,
    /// The value that represents the system register DBGWVR9_EL1.
    HV_SYS_REG_DBGWVR9_EL1 = 0x804e,
    /// The value that represents the system register DBGWCR9_EL1.
    HV_SYS_REG_DBGWCR9_EL1 = 0x804f,
    /// The value that represents the system register DBGBVR10_EL1.
    HV_SYS_REG_DBGBVR10_EL1 = 0x8054,
    /// The value that represents the system register DBGBCR10_EL1.
    HV_SYS_REG_DBGBCR10_EL1 = 0x8055,
    /// The value that represents the system register DBGWVR10_EL1.
    HV_SYS_REG_DBGWVR10_EL1 = 0x8056,
    /// The value that represents the system register DBGWCR10_EL1.
    HV_SYS_REG_DBGWCR10_EL1 = 0x8057,
    /// The value that represents the system register DBGBVR11_EL1.
    HV_SYS_REG_DBGBVR11_EL1 = 0x805c,
    /// The value that represents the system register DBGBCR11_EL1.
    HV_SYS_REG_DBGBCR11_EL1 = 0x805d,
    /// The value that represents the system register DBGWVR11_EL1.
    HV_SYS_REG_DBGWVR11_EL1 = 0x805e,
    /// The value that represents the system register DBGWCR11_EL1.
    HV_SYS_REG_DBGWCR11_EL1 = 0x805f,
    /// The value that represents the system register DBGBVR12_EL1.
    HV_SYS_REG_DBGBVR12_EL1 = 0x8064,
    /// The value that represents the system register DBGBCR12_EL1.
    HV_SYS_REG_DBGBCR12_EL1 = 0x8065,
    /// The value that represents the system register DBGWVR12_EL1.
    HV_SYS_REG_DBGWVR12_EL1 = 0x8066,
    /// The value that represents the system register DBGWCR12_EL1.
    HV_SYS_REG_DBGWCR12_EL1 = 0x8067,
    /// The value that represents the system register DBGBVR13_EL1.
    HV_SYS_REG_DBGBVR13_EL1 = 0x806c,
    /// The value that represents the system register DBGBCR13_EL1.
    HV_SYS_REG_DBGBCR13_EL1 = 0x806d,
    /// The value that represents the system register DBGWVR13_EL1.
    HV_SYS_REG_DBGWVR13_EL1 = 0x806e,
    /// The value that represents the system register DBGWCR13_EL1.
    HV_SYS_REG_DBGWCR13_EL1 = 0x806f,
    /// The value that represents the system register DBGBVR14_EL1.
    HV_SYS_REG_DBGBVR14_EL1 = 0x8074,
    /// The value that represents the system register DBGBCR14_EL1.
    HV_SYS_REG_DBGBCR14_EL1 = 0x8075,
    /// The value that represents the system register DBGWVR14_EL1.
    HV_SYS_REG_DBGWVR14_EL1 = 0x8076,
    /// The value that represents the system register DBGWCR14_EL1.
    HV_SYS_REG_DBGWCR14_EL1 = 0x8077,
    /// The value that represents the system register DBGBVR15_EL1.
    HV_SYS_REG_DBGBVR15_EL1 = 0x807c,
    /// The value that represents the system register DBGBCR15_EL1.
    HV_SYS_REG_DBGBCR15_EL1 = 0x807d,
    /// The value that represents the system register DBGWVR15_EL1.
    HV_SYS_REG_DBGWVR15_EL1 = 0x807e,
    /// The value that represents the system register DBGWCR15_EL1.
    HV_SYS_REG_DBGWCR15_EL1 = 0x807f,
    /// The value that represents the system register MIDR_EL1.
    HV_SYS_REG_MIDR_EL1 = 0xc000,
    /// The value that represents the system register MPIDR_EL1.
    HV_SYS_REG_MPIDR_EL1 = 0xc005,
    /// The value that describes the AArch64 Processor Feature Register 0.
    HV_SYS_REG_ID_AA64PFR0_EL1 = 0xc020,
    /// The value that describes the AArch64 Processor Feature Register 1.
    HV_SYS_REG_ID_AA64PFR1_EL1 = 0xc021,
    /// The value that describes the AArch64 Debug Feature Register 0.
    HV_SYS_REG_ID_AA64DFR0_EL1 = 0xc028,
    /// The value that describes the AArch64 Debug Feature Register 1.
    HV_SYS_REG_ID_AA64DFR1_EL1 = 0xc029,
    /// The value that describes the AArch64 Instruction Set Attribute Register 0.
    HV_SYS_REG_ID_AA64ISAR0_EL1 = 0xc030,
    /// The value that describes the AArch64 Instruction Set Attribute Register 1.
    HV_SYS_REG_ID_AA64ISAR1_EL1 = 0xc031,
    /// The value that describes the AArch64 Memory Model Feature Register 0.
    HV_SYS_REG_ID_AA64MMFR0_EL1 = 0xc038,
    /// The value that describes the AArch64 Memory Model Feature Register 1.
    HV_SYS_REG_ID_AA64MMFR1_EL1 = 0xc039,
    /// The value that describes the AArch64 Memory Model Feature Register 2.
    HV_SYS_REG_ID_AA64MMFR2_EL1 = 0xc03a,
    /// The value that represents the system register SCTLR_EL1.
    HV_SYS_REG_SCTLR_EL1 = 0xc080,
    /// The value that represents the system register CPACR_EL1.
    HV_SYS_REG_CPACR_EL1 = 0xc082,
    /// The value that represents the system register TTBR0_EL1.
    HV_SYS_REG_TTBR0_EL1 = 0xc100,
    /// The value that represents the system register TTBR1_EL1.
    HV_SYS_REG_TTBR1_EL1 = 0xc101,
    /// The value that represents the system register TCR_EL1.
    HV_SYS_REG_TCR_EL1 = 0xc102,
    /// The value that represents the system register APIAKEYLO_EL1.
    HV_SYS_REG_APIAKEYLO_EL1 = 0xc108,
    /// The value that represents the system register APIAKEYHI_EL1.
    HV_SYS_REG_APIAKEYHI_EL1 = 0xc109,
    /// The value that represents the system register APIBKEYLO_EL1.
    HV_SYS_REG_APIBKEYLO_EL1 = 0xc10a,
    /// The value that represents the system register APIBKEYHI_EL1.
    HV_SYS_REG_APIBKEYHI_EL1 = 0xc10b,
    /// The value that represents the system register APDAKEYLO_EL1.
    HV_SYS_REG_APDAKEYLO_EL1 = 0xc110,
    /// The value that represents the system register APDAKEYHI_EL1.
    HV_SYS_REG_APDAKEYHI_EL1 = 0xc111,
    /// The value that represents the system register APDBKEYLO_EL1.
    HV_SYS_REG_APDBKEYLO_EL1 = 0xc112,
    /// The value that represents the system register APDBKEYHI_EL1.
    HV_SYS_REG_APDBKEYHI_EL1 = 0xc113,
    /// The value that represents the system register APGAKEYLO_EL1.
    HV_SYS_REG_APGAKEYLO_EL1 = 0xc118,
    /// The value that represents the system register APGAKEYHI_EL1.
    HV_SYS_REG_APGAKEYHI_EL1 = 0xc119,
    /// The value that represents the system register SPSR_EL1.
    HV_SYS_REG_SPSR_EL1 = 0xc200,
    /// The value that represents the system register ELR_EL1.
    HV_SYS_REG_ELR_EL1 = 0xc201,
    /// The value that represents the system register SP_EL0.
    HV_SYS_REG_SP_EL0 = 0xc208,
    /// The value that represents the system register AFSR0_EL1.
    HV_SYS_REG_AFSR0_EL1 = 0xc288,
    /// The value that represents the system register AFSR1_EL1.
    HV_SYS_REG_AFSR1_EL1 = 0xc289,
    /// The value that represents the system register ESR_EL1.
    HV_SYS_REG_ESR_EL1 = 0xc290,
    /// The value that represents the system register FAR_EL1.
    HV_SYS_REG_FAR_EL1 = 0xc300,
    /// The value that represents the system register PAR_EL1.
    HV_SYS_REG_PAR_EL1 = 0xc3a0,
    /// The value that represents the system register MAIR_EL1.
    HV_SYS_REG_MAIR_EL1 = 0xc510,
    /// The value that represents the system register AMAIR_EL1.
    HV_SYS_REG_AMAIR_EL1 = 0xc518,
    /// The value that represents the system register VBAR_EL1.
    HV_SYS_REG_VBAR_EL1 = 0xc600,
    /// The value that represents the system register CONTEXTIDR_EL1.
    HV_SYS_REG_CONTEXTIDR_EL1 = 0xc681,
    /// The value that represents the system register TPIDR_EL1.
    HV_SYS_REG_TPIDR_EL1 = 0xc684,
    /// The value that represents the system register CNTKCTL_EL1.
    HV_SYS_REG_CNTKCTL_EL1 = 0xc708,
    /// The value that represents the system register CSSELR_EL1.
    HV_SYS_REG_CSSELR_EL1 = 0xd000,
    /// The value that represents the system register TPIDR_EL0.
    HV_SYS_REG_TPIDR_EL0 = 0xde82,
    /// The value that represents the system register TPIDRRO_EL0.
    HV_SYS_REG_TPIDRRO_EL0 = 0xde83,
    /// The value that represents the system register CNTV_CTL_EL0.
    HV_SYS_REG_CNTV_CTL_EL0 = 0xdf19,
    /// The value that represents the system register CNTV_CVAL_EL0.
    HV_SYS_REG_CNTV_CVAL_EL0 = 0xdf1a,
    /// The value that represents the system register SP_EL1.
    HV_SYS_REG_SP_EL1 = 0xe208,
}

extern "C" {
    /// Gets the current value of a vCPU system register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `reg`: The ID of the system register.
    /// * `value`: The value of the register reg on output.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_sys_reg(vcpu: hv_vcpu_t, reg: hv_sys_reg_t, value: *mut u64) -> hv_return_t;

    /// Sets the value of a vCPU system register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `reg`: The ID of the system register.
    /// * `value`: The new value of the register.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_set_sys_reg(vcpu: hv_vcpu_t, reg: hv_sys_reg_t, value: u64) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - Trap Configuration
// -----------------------------------------------------------------------------------------------

extern "C" {
    /// Gets whether debug exceptions exit the guest.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `value`: Indicates whether debug exceptions in the guest trap to the host on output.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_trap_debug_exceptions(vcpu: hv_vcpu_t, value: *mut bool) -> hv_return_t;

    /// Sets whether debug exceptions exit the guest.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `value`: A Boolean value that if true indicates debug exceptions in the guest trap to
    ///            the host.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_set_trap_debug_exceptions(vcpu: hv_vcpu_t, value: bool) -> hv_return_t;

    /// Gets whether debug-register accesses exit the guest.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `value`: Indicates whether debug-register accesses in the guest trap to the host on
    ///            output.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_trap_debug_reg_accesses(vcpu: hv_vcpu_t, value: *mut bool) -> hv_return_t;

    /// Sets whether debug-register accesses exit the guest.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The vCPU instance.
    /// * `value`: A Boolean value that if true indicates debug-register accesses in the guest
    ///            trap to the host.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_set_trap_debug_reg_accesses(vcpu: hv_vcpu_t, value: bool) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// Memory Management
// -----------------------------------------------------------------------------------------------

/// The type of an intermediate physical address, which is a guest physical address space of the
/// VM.
pub type hv_ipa_t = u64;
/// The permissions for guest physical memory regions.
pub type hv_memory_flags_t = u64;

/// The value that represents the memory-read permission.
pub const HV_MEMORY_READ: hv_memory_flags_t = 1u64 << 0;
/// The value that represents the memory-write permission.
pub const HV_MEMORY_WRITE: hv_memory_flags_t = 1u64 << 1;
/// The value that represents the memory-execute permission.
pub const HV_MEMORY_EXEC: hv_memory_flags_t = 1u64 << 2;

extern "C" {
    /// Maps a region in the virtual address space of the current process into the guest physical
    /// address space of the VM.
    ///
    /// # Parameters
    ///
    /// * `addr`: The address in the current process. It must be page-aligned.
    /// * `ipa`: The address in the intermediate physical address space. It must be page-aligned.
    /// * `size`: The size of the mapped region in bytes. It must be a multiple of the page size.
    /// * `flags`: The permissions for the mapped region. For a list of valid options, see
    ///            [`hv_memory_flags_t`].
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vm_map(
        addr: *const c_void,
        ipa: hv_ipa_t,
        size: usize,
        flags: hv_memory_flags_t,
    ) -> hv_return_t;

    /// Unmaps a region in the guest physical address space of the VM.
    ///
    /// # Parameters
    ///
    /// * `ipa`: The address in the intermediate physical address space. It must be page-aligned.
    /// * `size`: The size of the region to unmap, in bytes. It must be a multiple of the page
    ///           size.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vm_unmap(ipa: hv_ipa_t, size: usize) -> hv_return_t;

    /// Modifies the permissions of a region in the guest physical address space of the VM.
    ///
    /// # Parameters
    ///
    /// * `ipa`: The address in the intermediate physical address space. It must be page-aligned.
    /// * `size`: The size of the region to unmap, in bytes. It must be a multiple of the page
    ///           size.
    /// * `flags`: The permissions for the protected region. For a list of valid options, see
    ///            [`hv_memory_flags_t.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vm_protect(ipa: hv_ipa_t, size: usize, flags: hv_memory_flags_t) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// Timer Functions
// -----------------------------------------------------------------------------------------------

extern "C" {
    /// Gets the virtual timer mask.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The ID of the vCPU instance.
    /// * `vtimer_is_masked`: The value of the mask.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_vtimer_mask(vcpu: hv_vcpu_t, vtimer_is_masked: *mut bool) -> hv_return_t;

    /// Sets or clears the virtual timer mask.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The ID of the vCPU instance.
    /// * `vtimer_is_masked`: A Boolean value that indicates whether the vTimer has a mask set.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_set_vtimer_mask(vcpu: hv_vcpu_t, vtimer_is_masked: bool) -> hv_return_t;

    /// Returns the vTimer offset for the vCPU ID you specify.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The ID of the vCPU instance.
    /// * `vtimer_offset`: A pointer to vTimer offset; the Hypervisor writes to this value on
    ///                    success.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_get_vtimer_offset(vcpu: hv_vcpu_t, vtimer_offset: *mut u64) -> hv_return_t;

    /// Sets the vTimer offset to a value that you provide.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: The ID of the vCPU instance.
    /// * `vtimer_offset`: The new vTimer offset.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    pub fn hv_vcpu_set_vtimer_offset(vcpu: hv_vcpu_t, vtimer_offset: u64) -> hv_return_t;
}

#[cfg(test)]
mod tests {
    // Tests must be run with `--test-threads=1`, since only one VM instance is allowed per
    // process. Tests could fail because `hv_vm_create` is called multiple times in concurrent
    // threads.

    use super::*;
    use std::alloc::{alloc, Layout};
    use std::ptr;

    #[test]
    fn vm_create_destroy() {
        let config = ptr::null_mut();
        // Creates a VM instance for the current process.
        let ret = unsafe { hv_vm_create(config) };
        assert_eq!(ret, hv_error_t::HV_SUCCESS as i32);
        // Trying to create a second instance leads to a HV_BUSY error.
        let ret = unsafe { hv_vm_create(config) };
        assert_eq!(ret, hv_error_t::HV_BUSY as i32);
        // Destroys the process instance.
        let ret = unsafe { hv_vm_destroy() };
        assert_eq!(ret, hv_error_t::HV_SUCCESS as i32);
    }

    #[test]
    pub fn vcpu_create_destroy() {
        let config = ptr::null_mut();
        // Creates a VM instance for the current process.
        let ret = unsafe { hv_vm_create(config) };
        assert_eq!(ret, hv_error_t::HV_SUCCESS as i32);
        // Retrieves the maximum number of vCPU.
        let mut max_vcpu_count = 0;
        let ret = unsafe { hv_vm_get_max_vcpu_count(&mut max_vcpu_count) };
        assert_eq!(ret, hv_error_t::HV_SUCCESS as i32);
        // Creates a vCPU.
        let mut vcpu: hv_vcpu_t = 0;
        let layout = Layout::new::<hv_vcpu_exit_t>();
        let ret = unsafe {
            let exit = alloc(layout) as *mut *const hv_vcpu_exit_t;
            hv_vcpu_create(&mut vcpu, exit, config)
        };
        assert_eq!(ret, hv_error_t::HV_SUCCESS as i32);
        // Destroys a vCPU.
        let ret = unsafe { hv_vcpu_destroy(vcpu) };
        assert_eq!(ret, hv_error_t::HV_SUCCESS as i32);
        // Destroys the process instance.
        let ret = unsafe { hv_vm_destroy() };
        assert_eq!(ret, hv_error_t::HV_SUCCESS as i32);
    }
}
