#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "simd-nightly", feature(portable_simd), feature(simd_ffi))]
#![allow(non_camel_case_types)]
#![allow(improper_ctypes)]

use core::ffi::c_void;

#[cfg_attr(target_os = "macos", link(name = "Hypervisor", kind = "framework"))]
unsafe extern "C" {}

/// The size of a memory page on Apple Silicon.
pub const PAGE_SIZE: usize = 0x4000;

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
// Utils
// -----------------------------------------------------------------------------------------------

unsafe extern "C" {
    pub fn os_release(object: *mut c_void);
}

// -----------------------------------------------------------------------------------------------
// Virtual Machine Management
// -----------------------------------------------------------------------------------------------

/// Supported intermediate physical address (IPA) granules.
#[cfg(feature = "macos-26-0")]
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_ipa_granule_t {
    /// 4KB Granule.
    HV_IPA_GRANULE_4KB,
    /// 16KB Granule.
    HV_IPA_GRANULE_16KB,
}

/// The type that defines a virtual-machine configuration.
pub type hv_vm_config_t = *mut c_void;

/// Memory allocation flags.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_allocate_flags_t {
    /// Default allocation flags.
    HV_ALLOCATE_DEFAULT = 0,
}

unsafe extern "C" {
    /// Creates a virtual machine configuration object.
    ///
    /// # Return Value
    ///
    /// A new virtual-machine configuration object. Release this object with os_release when no
    /// longer used.
    pub fn hv_vm_config_create() -> hv_vm_config_t;

    /// Creates a VM instance for the current process.
    ///
    /// # Parameters
    ///
    /// * `config`: The configuration of the vCPU. Pass NULL for the default configuration.
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

    /// Allocates anonymous memory suitable to be mapped as guest memory.
    ///
    /// # Discussion
    ///
    /// - The memory is allocated with `VM_PROT_DEFAULT` permissions.
    /// - This API enables accurate memory accounting of the allocations it creates.
    /// - Memory allocated with this API should deallocated with [`hv_vm_deallocate`].
    ///
    /// # Parameters
    ///
    /// * `uvap`: Returned virtual address of the allocated memory.
    /// * `size`: Size in bytes of the region to be allocated. Must be a multiple of [`PAGE_SIZE`].
    /// * `flags`: Memory allocation flags.
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-12-1")]
    pub fn hv_vm_allocate(
        uvap: *mut *mut c_void,
        size: libc::size_t,
        flags: hv_allocate_flags_t,
    ) -> hv_return_t;

    /// Deallocate memory previously allocated by [`hv_vm_allocate`].
    ///
    /// # Parameters
    ///
    /// * `uva`: Virtual address of the allocated memory.
    /// * `size`: Size in bytes of the region to be deallocated.
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-12-1")]
    pub fn hv_vm_deallocate(uvap: *const c_void, size: libc::size_t) -> hv_return_t;

    /// Return the maximum intermediate physical address bit length.
    ///
    /// # Discussion
    ///
    /// The bit length is the number of valid bits from an intermediate physical address (IPA).
    /// For example, max IPA bit length of 36 means only the least significant 36 bits of an IPA
    /// are valid, and covers a 64GB range.
    ///
    /// # Parameters
    ///
    /// * `ipa_bit_length`: Pointer to bit length (written on success).
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-13-0")]
    pub fn hv_vm_config_get_max_ipa_size(ipa_bit_length: *mut u32) -> hv_return_t;

    /// Return the default intermediate physical address bit length.
    ///
    /// # Discussion
    ///
    /// This default IPA size is used if the IPA size is not set explicitly.
    ///
    /// # Parameters
    ///
    /// * `ipa_bit_length`: Pointer to bit length (written on success).
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-13-0")]
    pub fn hv_vm_config_get_default_ipa_size(ipa_bit_length: *mut u32) -> hv_return_t;

    /// Set intermediate physical address bit length in virtual machine configuration.
    ///
    /// # Parameters
    ///
    /// * `config`: The configuration of the vCPU.
    /// * `ipa_bit_length`: Intermediate physical address bit length.
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-13-0")]
    pub fn hv_vm_config_set_ipa_size(config: hv_vm_config_t, ipa_bit_length: u32) -> hv_return_t;

    /// Return intermediate physical address bit length in configuration.
    ///
    /// # Parameters
    ///
    /// * `config`: The configuration of the vCPU.
    /// * `ipa_bit_length`: Pointer to bit length (written on success).
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-13-0")]
    pub fn hv_vm_config_get_ipa_size(
        config: hv_vm_config_t,
        ipa_bit_length: *mut u32,
    ) -> hv_return_t;

    /// Return whether or not EL2 is supported on the current platform.
    ///
    /// # Parameters
    ///
    /// * `el2_supported`: Pointer to whether or not EL2 is supported (written on success).
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_vm_config_get_el2_supported(el2_supported: *mut bool) -> hv_return_t;

    /// Return whether or not EL2 is enabled for a VM configuration.
    ///
    /// # Parameters
    ///
    /// * `config`: The configuration of the vCPU.
    /// * `el2_enabled`: Pointer to whether or not EL2 is enabled (written on success).
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_vm_config_get_el2_enabled(
        config: hv_vm_config_t,
        el2_enabled: *mut bool,
    ) -> hv_return_t;

    /// Set whether or not EL2 is enabled for a VM configuration.
    ///
    /// # Parameters
    ///
    /// * `config`: The configuration of the vCPU.
    /// * `el2_enabled`: Whether or not to enable EL2.
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_vm_config_set_el2_enabled(config: hv_vm_config_t, el2_enabled: bool) -> hv_return_t;

    /// Return the default intermediate physical address granule.
    ///
    /// # Parameters
    ///
    /// * `granule`: Pointer to the default intermediate physical address granule size
    /// (written on success).
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-26-0")]
    pub fn hv_vm_config_get_default_ipa_granule(granule: *mut hv_ipa_granule_t) -> hv_return_t;

    /// Return the intermediate physical address granule size in virtual machine configuration.
    ///
    /// # Parameters
    ///
    /// * `config`: Configuration.
    /// * `granule`: Pointer to the default intermediate physical address granule size
    /// (written on success).
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-26-0")]
    pub fn hv_vm_config_get_ipa_granule(
        config: hv_vm_config_t,
        granule: *mut hv_ipa_granule_t,
    ) -> hv_return_t;

    /// Set the intermediate physical address granule size in virtual machine configuration.
    ///
    /// # Parameters
    ///
    /// * `config`: Configuration.
    /// * `granule`: Granule size.
    ///
    /// # Return value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-26-0")]
    pub fn hv_vm_config_set_ipa_granule(
        config: hv_vm_config_t,
        granule: hv_ipa_granule_t,
    ) -> hv_return_t;
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
    ID_AA64DFR0_EL1,
    /// The value that identifies debug feature register 1, EL1 (DFR1_EL1).
    ID_AA64DFR1_EL1,
    /// The value that identifies instruction set attribute register 0, EL1 (ISAR0_EL1).
    ID_AA64ISAR0_EL1,
    /// The value that identifies instruction set attribute register 1, EL1 (ISAR_EL1).
    ID_AA64ISAR1_EL1,
    /// The value that identifies memory model feature register 0, EL1(MMFR0_EL1).
    ID_AA64MMFR0_EL1,
    /// The value that identifies memory model feature register 1, EL1 (MMFR1_EL1).
    ID_AA64MMFR1_EL1,
    /// The value that identifies memory model feature register 2, EL1 (MMFR2_EL1).
    ID_AA64MMFR2_EL1,
    /// The value that identifies processor feature register 0, EL1 (PFR0_EL1).
    ID_AA64PFR0_EL1,
    /// The value that identifies processor feature register 1, EL1 (PFR1_EL1).
    ID_AA64PFR1_EL1,
    /// The value that describes Cache Type Register, EL0.
    CTR_EL0,
    /// The value that describes Cache Level ID Register, EL1.
    CLIDR_EL1,
    /// The values that describes Data Cache Zero ID Register, EL0.
    DCZID_EL0,
    /// The value that describes Scalable Matrix Extension (SME) Feature ID Register 0.
    #[cfg(feature = "macos-15-2")]
    ID_AA64SMFR0_EL1,
    /// The value that describes Scalable Vector Extension instruction (SVE) Feature ID register 0.
    #[cfg(feature = "macos-15-2")]
    ID_AA64ZFR0_EL1,
}

/// The structure that describes an instruction or data cache element.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_cache_type_t {
    /// The value that describes a cached data value.
    DATA,
    /// The value that describes a cached instuction value.
    INSTRUCTION,
}

unsafe extern "C" {
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
    CANCELED,
    /// The value that identifies traps caused by the guest operations.
    EXCEPTION,
    /// The value that identifies when the virtual timer enters the pending state.
    VTIMER_ACTIVATED,
    /// The value that identifies unexpected exits.
    UNKNOWN,
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

unsafe extern "C" {
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
    FIQ,
    /// ARM Interrupt Request.
    IRQ,
}

unsafe extern "C" {
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
    X0,
    /// The value that identifies register X1.
    X1,
    /// The value that identifies register X2.
    X2,
    /// The value that identifies register X3.
    X3,
    /// The value that identifies register X4.
    X4,
    /// The value that identifies register X5.
    X5,
    /// The value that identifies register X6.
    X6,
    /// The value that identifies register X7.
    X7,
    /// The value that identifies register X8.
    X8,
    /// The value that identifies register X9.
    X9,
    /// The value that identifies register X10.
    X10,
    /// The value that identifies register X11.
    X11,
    /// The value that identifies register X12.
    X12,
    /// The value that identifies register X13.
    X13,
    /// The value that identifies register X14.
    X14,
    /// The value that identifies register X15.
    X15,
    /// The value that identifies register X16.
    X16,
    /// The value that identifies register X17.
    X17,
    /// The value that identifies register X18.
    X18,
    /// The value that identifies register X19.
    X19,
    /// The value that identifies register X20.
    X20,
    /// The value that identifies register X21.
    X21,
    /// The value that identifies register X22.
    X22,
    /// The value that identifies register X23.
    X23,
    /// The value that identifies register X24.
    X24,
    /// The value that identifies register X25.
    X25,
    /// The value that identifies register X26.
    X26,
    /// The value that identifies register X27.
    X27,
    /// The value that identifies register X28.
    X28,
    /// The value that identifies register X29.
    X29,
    /// The value that identifies register X30.
    X30,
    /// The value that identifies the program counter (PC).
    PC,
    /// The value that identifies the floating-point control register (FPCR).
    FPCR,
    /// The value that identifies the floating-point status register (FPSR).
    FPSR,
    /// The value that identifies the current program status register (CPSR).
    CPSR,
}

impl hv_reg_t {
    /// The value that identifies the frame pointer (FP).
    pub const FP: Self = Self::X29;
    /// The value that identifies the link register (LR).
    pub const LR: Self = Self::X30;
}

unsafe extern "C" {
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
#[cfg(feature = "simd-nightly")]
pub type hv_simd_fp_uchar16_t = std::simd::u8x16;
#[cfg(not(feature = "simd-nightly"))]
pub type hv_simd_fp_uchar16_t = u128;

/// The type that defines SIMD and floating-point registers.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_simd_fp_reg_t {
    /// The value representing SIMD register Q0.
    Q0,
    /// The value representing SIMD register Q1.
    Q1,
    /// The value representing SIMD register Q2.
    Q2,
    /// The value representing SIMD register Q3.
    Q3,
    /// The value representing SIMD register Q4.
    Q4,
    /// The value representing SIMD register Q5.
    Q5,
    /// The value representing SIMD register Q6.
    Q6,
    /// The value representing SIMD register Q7.
    Q7,
    /// The value representing SIMD register Q8.
    Q8,
    /// The value representing SIMD register Q9.
    Q9,
    /// The value representing SIMD register Q10.
    Q10,
    /// The value representing SIMD register Q11.
    Q11,
    /// The value representing SIMD register Q12.
    Q12,
    /// The value representing SIMD register Q13.
    Q13,
    /// The value representing SIMD register Q14.
    Q14,
    /// The value representing SIMD register Q15.
    Q15,
    /// The value representing SIMD register Q16.
    Q16,
    /// The value representing SIMD register Q17.
    Q17,
    /// The value representing SIMD register Q18.
    Q18,
    /// The value representing SIMD register Q19.
    Q19,
    /// The value representing SIMD register Q20.
    Q20,
    /// The value representing SIMD register Q21.
    Q21,
    /// The value representing SIMD register Q22.
    Q22,
    /// The value representing SIMD register Q23.
    Q23,
    /// The value representing SIMD register Q24.
    Q24,
    /// The value representing SIMD register Q25.
    Q25,
    /// The value representing SIMD register Q26.
    Q26,
    /// The value representing SIMD register Q27.
    Q27,
    /// The value representing SIMD register Q28.
    Q28,
    /// The value representing SIMD register Q29.
    Q29,
    /// The value representing SIMD register Q30.
    Q30,
    /// The value representing SIMD register Q31.
    Q31,
}

unsafe extern "C" {
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
// vCPU Management - SVE & SME
// -----------------------------------------------------------------------------------------------

/// Contains information about SME PSTATE.
#[cfg(feature = "macos-15-2")]
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct hv_vcpu_sme_state_t {
    /// Controls `PSTATE.SM`.
    pub streaming_sve_mode_enabled: bool,
    /// Controls `PSTATE.ZA`.
    pub za_storage_enabled: bool,
}

/// Type of an ARM SME Z vector register.
#[cfg(feature = "macos-15-2")]
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_sme_z_reg_t {
    /// The value representing the streaming vector register Z0.
    Z0,
    /// The value representing the streaming vector register Z1.
    Z1,
    /// The value representing the streaming vector register Z2.
    Z2,
    /// The value representing the streaming vector register Z3.
    Z3,
    /// The value representing the streaming vector register Z4.
    Z4,
    /// The value representing the streaming vector register Z5.
    Z5,
    /// The value representing the streaming vector register Z6.
    Z6,
    /// The value representing the streaming vector register Z7.
    Z7,
    /// The value representing the streaming vector register Z8.
    Z8,
    /// The value representing the streaming vector register Z9.
    Z9,
    /// The value representing the streaming vector register Z10.
    Z10,
    /// The value representing the streaming vector register Z11.
    Z11,
    /// The value representing the streaming vector register Z12.
    Z12,
    /// The value representing the streaming vector register Z13.
    Z13,
    /// The value representing the streaming vector register Z14.
    Z14,
    /// The value representing the streaming vector register Z15.
    Z15,
    /// The value representing the streaming vector register Z16.
    Z16,
    /// The value representing the streaming vector register Z17.
    Z17,
    /// The value representing the streaming vector register Z18.
    Z18,
    /// The value representing the streaming vector register Z19.
    Z19,
    /// The value representing the streaming vector register Z20.
    Z20,
    /// The value representing the streaming vector register Z21.
    Z21,
    /// The value representing the streaming vector register Z22.
    Z22,
    /// The value representing the streaming vector register Z23.
    Z23,
    /// The value representing the streaming vector register Z24.
    Z24,
    /// The value representing the streaming vector register Z25.
    Z25,
    /// The value representing the streaming vector register Z26.
    Z26,
    /// The value representing the streaming vector register Z27.
    Z27,
    /// The value representing the streaming vector register Z28.
    Z28,
    /// The value representing the streaming vector register Z29.
    Z29,
    /// The value representing the streaming vector register Z30.
    Z30,
    /// The value representing the streaming vector register Z31.
    Z31,
}

/// Type of an ARM SME P predicate register.
#[cfg(feature = "macos-15-2")]
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_sme_p_reg_t {
    /// The value representing the streaming predicate register P0.
    P0,
    /// The value representing the streaming predicate register P1.
    P1,
    /// The value representing the streaming predicate register P2.
    P2,
    /// The value representing the streaming predicate register P3.
    P3,
    /// The value representing the streaming predicate register P4.
    P4,
    /// The value representing the streaming predicate register P5.
    P5,
    /// The value representing the streaming predicate register P6.
    P6,
    /// The value representing the streaming predicate register P7.
    P7,
    /// The value representing the streaming predicate register P8.
    P8,
    /// The value representing the streaming predicate register P9.
    P9,
    /// The value representing the streaming predicate register P10.
    P10,
    /// The value representing the streaming predicate register P11.
    P11,
    /// The value representing the streaming predicate register P12.
    P12,
    /// The value representing the streaming predicate register P13.
    P13,
    /// The value representing the streaming predicate register P14.
    P14,
    /// The value representing the streaming predicate register P15.
    P15,
}

/// Type of the SME2 ZT0 register.
#[cfg(all(feature = "macos-15-2", not(feature = "simd-nightly")))]
pub type hv_sme_zt0_uchar64_t = [u8; 64];
#[cfg(all(feature = "macos-15-2", feature = "simd-nightly"))]
pub type hv_sme_zt0_uchar64_t = std::simd::u8x64;

unsafe extern "C" {
    /// Returns the value of the maximum Streaming Vector Length (SVL) in bytes.
    ///
    /// # Discussion
    ///
    /// This is the maximum SVL that guests may use and separate from the effective SVL that
    /// guests may set using `SMCR_EL1`.
    ///
    /// # Parameters
    ///
    /// * `value`: Pointer to the value.
    ///
    /// # Return Value
    ///
    /// - Returns `HV_UNSUPPORTED` if SME is not supported.
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-2")]
    pub fn hv_sme_config_get_max_svl_bytes(value: *mut libc::size_t) -> hv_return_t;

    /// Gets the current SME state consisting of the streaming SVE mode (`PSTATE.SM`) and ZA
    /// storage enable (`PSTATE.ZA`).
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// In streaming SVE mode, the SIMD Q registers are aliased to the bottom 128 bits of the
    /// corresponding Z register, and any modification will reflect on the Z register state.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `sme_state`: Pointer to the SME state.
    ///
    /// # Return Value
    ///
    /// - Returns `HV_UNSUPPORTED` if SME is not supported.
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_get_sme_state(
        vcpu: hv_vcpu_t,
        sme_state: *mut hv_vcpu_sme_state_t,
    ) -> hv_return_t;

    /// Sets the SME state consisting of the streaming SVE mode and ZA storage enable.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// For any entry or exit from streaming SVE mode, all Z vector and P predicate registers
    /// are set to zero, and all FPSR flags are set; this state must be saved if it needs to be
    /// retained across streaming SVE mode transitions.
    ///
    /// In streaming SVE mode, the SIMD Q registers are aliased to the bottom 128 bits of the
    /// corresponding Z register, and any modification will reflect on the Z register state.
    ///
    /// If the optional `FEAT_SME_FA64` is implemented, the full SIMD instruction set is supported
    /// in streaming SVE mode; otherwise many legacy SIMD instructions are illegal in this mode.
    ///
    /// When finished, disable streaming SVE mode and ZA storage; this serves as a power-down
    /// hint for SME-related hardware.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `sme_state`: Pointer to the SME state to set.
    ///
    /// # Return Value
    ///
    /// - Returns `HV_UNSUPPORTED` if SME is not supported.
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_set_sme_state(
        vcpu: hv_vcpu_t,
        sme_state: *const hv_vcpu_sme_state_t,
    ) -> hv_return_t;

    /// Returns the value of a vCPU Z vector register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `reg`: ID of the Z vector register.
    /// * `value`: Pointer to the retrieved register value.
    /// * `length`: The length (in bytes) of the provided value storage.
    ///
    /// # Return Value
    ///
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    ///   [`hv_return_t`].
    /// - Returns an error if not in streaming SVE mode (i.e. `streaming_sve_mode_enabled` is
    ///   false), or if the provided value storage is not maximum SVL bytes.
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_get_sme_z_reg(
        vcpu: hv_vcpu_t,
        reg: hv_sme_z_reg_t,
        value: *mut u8,
        length: libc::size_t,
    ) -> hv_return_t;

    /// Sets the value of a vCPU Z vector register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `reg`: ID of the Z vector register.
    /// * `value`: Pointer to the register value to set.
    /// * `length`: The length (in bytes) of the Z register value.
    ///
    /// # Return Value
    ///
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    ///   [`hv_return_t`].
    /// - Returns an error if not in streaming SVE mode (i.e. `streaming_sve_mode_enabled` is
    ///   false), or if the value length is not maximum SVL bytes.
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_set_sme_z_reg(
        vcpu: hv_vcpu_t,
        reg: hv_sme_z_reg_t,
        value: *const u8,
        length: libc::size_t,
    ) -> hv_return_t;

    /// Returns the value of a vCPU P predicate register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `reg`: ID of the P vector register.
    /// * `value`: Pointer to the retrieved register value.
    /// * `length`: The length (in bytes) of the provided value storage.
    ///
    /// # Return Value
    ///
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    ///   [`hv_return_t`].
    /// - Returns an error if not in streaming SVE mode (i.e. `streaming_sve_mode_enabled` is
    ///   false), or if the provided value storage is not maximum SVL bytes.
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_get_sme_p_reg(
        vcpu: hv_vcpu_t,
        reg: hv_sme_p_reg_t,
        value: *mut u8,
        length: libc::size_t,
    ) -> hv_return_t;

    /// Sets the value of a vCPU P predicate register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `reg`: ID of the P vector register.
    /// * `value`: Pointer to the register value to set.
    /// * `length`: The length (in bytes) of the P register value.
    ///
    /// # Return Value
    ///
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    ///   [`hv_return_t`].
    /// - Returns an error if not in streaming SVE mode (i.e. `streaming_sve_mode_enabled` is
    ///   false), or if the value length is not maximum SVL bytes.
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_set_sme_p_reg(
        vcpu: hv_vcpu_t,
        reg: hv_sme_p_reg_t,
        value: *const u8,
        length: libc::size_t,
    ) -> hv_return_t;

    /// Returns the value of the vCPU ZA matrix register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Does not require streaming SVE mode enabled.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `value`: Pointer to the retrieved register value.
    /// * `length`: The length (in bytes) of the provided value storage.
    ///
    /// # Return Value
    ///
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    ///   [`hv_return_t`].
    /// - Returns an error if `PSTATE.ZA` is 0 (i.e. `za_storage_enabled` is false), or if the
    ///   provided value storage is not [maximum SVL bytes x maximum SVL bytes].
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_get_sme_za_reg(
        vcpu: hv_vcpu_t,
        value: *mut u8,
        length: libc::size_t,
    ) -> hv_return_t;

    /// Sets the value of the vCPU ZA matrix register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `value`: Pointer to the register value to set.
    /// * `length`: The length (in bytes) of the provided ZA register value.
    ///
    /// # Return Value
    ///
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    ///   [`hv_return_t`].
    /// - Returns an error if `PSTATE.ZA` is 0 (i.e. `za_storage_enabled` is false), or if the
    ///   value length is not [maximum SVL bytes x maximum SVL bytes].
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_set_sme_za_reg(
        vcpu: hv_vcpu_t,
        value: *const u8,
        length: libc::size_t,
    ) -> hv_return_t;

    /// Returns the current value of the vCPU ZT0 register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Does not require streaming SVE mode enabled.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `value`: Pointer to the retrieved register value.
    ///
    /// # Return Value
    ///
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    ///   [`hv_return_t`].
    /// - Returns an error if `PSTATE.ZA` is 0 (i.e. `za_storage_enabled` is false).
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_get_sme_zt0_reg(vcpu: hv_vcpu_t, value: *mut u8) -> hv_return_t;

    /// Sets the value of the vCPU ZT0 register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: ID of the vCPU instance.
    /// * `value`: Pointer to the register value to set.
    ///
    /// # Return Value
    ///
    /// - `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    ///   [`hv_return_t`].
    /// - Returns an error if `PSTATE.ZA` is 0 (i.e. `za_storage_enabled` is false).
    #[cfg(feature = "macos-15-2")]
    pub fn hv_vcpu_set_sme_zt0_reg(vcpu: hv_vcpu_t, value: *const u8) -> hv_return_t;
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - System Registers
// -----------------------------------------------------------------------------------------------

/// The type of system registers.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_sys_reg_t {
    /// The value that represents the system register DBGBVR0_EL1.
    DBGBVR0_EL1 = 0x8004,
    /// The value that represents the system register DBGBCR0_EL1.
    DBGBCR0_EL1 = 0x8005,
    /// The value that represents the system register DBGWVR0_EL1.
    DBGWVR0_EL1 = 0x8006,
    /// The value that represents the system register DBGWCR0_EL1.
    DBGWCR0_EL1 = 0x8007,
    /// The value that represents the system register DBGBVR1_EL1.
    DBGBVR1_EL1 = 0x800c,
    /// The value that represents the system register DBGBCR1_EL1.
    DBGBCR1_EL1 = 0x800d,
    /// The value that represents the system register DBGWVR1_EL1.
    DBGWVR1_EL1 = 0x800e,
    /// The value that represents the system register DBGWCR1_EL1.
    DBGWCR1_EL1 = 0x800f,
    /// The value that represents the system register MDCCINT_EL1.
    MDCCINT_EL1 = 0x8010,
    /// The value that represents the system register MDSCR_EL1.
    MDSCR_EL1 = 0x8012,
    /// The value that represents the system register DBGBVR2_EL1.
    DBGBVR2_EL1 = 0x8014,
    /// The value that represents the system register DBGBCR2_EL1.
    DBGBCR2_EL1 = 0x8015,
    /// The value that represents the system register DBGWVR2_EL1.
    DBGWVR2_EL1 = 0x8016,
    /// The value that represents the system register DBGWCR2_EL1.
    DBGWCR2_EL1 = 0x8017,
    /// The value that represents the system register DBGBVR3_EL1.
    DBGBVR3_EL1 = 0x801c,
    /// The value that represents the system register DBGBCR3_EL1.
    DBGBCR3_EL1 = 0x801d,
    /// The value that represents the system register DBGWVR3_EL1.
    DBGWVR3_EL1 = 0x801e,
    /// The value that represents the system register DBGWCR3_EL1.
    DBGWCR3_EL1 = 0x801f,
    /// The value that represents the system register DBGBVR4_EL1.
    DBGBVR4_EL1 = 0x8024,
    /// The value that represents the system register DBGBCR4_EL1.
    DBGBCR4_EL1 = 0x8025,
    /// The value that represents the system register DBGWVR4_EL1.
    DBGWVR4_EL1 = 0x8026,
    /// The value that represents the system register DBGWCR4_EL1.
    DBGWCR4_EL1 = 0x8027,
    /// The value that represents the system register DBGBVR5_EL1.
    DBGBVR5_EL1 = 0x802c,
    /// The value that represents the system register DBGBCR5_EL1.
    DBGBCR5_EL1 = 0x802d,
    /// The value that represents the system register DBGWVR5_EL1.
    DBGWVR5_EL1 = 0x802e,
    /// The value that represents the system register DBGWCR5_EL1.
    DBGWCR5_EL1 = 0x802f,
    /// The value that represents the system register DBGBVR6_EL1.
    DBGBVR6_EL1 = 0x8034,
    /// The value that represents the system register DBGBCR6_EL1.
    DBGBCR6_EL1 = 0x8035,
    /// The value that represents the system register DBGWVR6_EL1.
    DBGWVR6_EL1 = 0x8036,
    /// The value that represents the system register DBGWCR6_EL1.
    DBGWCR6_EL1 = 0x8037,
    /// The value that represents the system register DBGBVR7_EL1.
    DBGBVR7_EL1 = 0x803c,
    /// The value that represents the system register DBGBCR7_EL1.
    DBGBCR7_EL1 = 0x803d,
    /// The value that represents the system register DBGWVR7_EL1.
    DBGWVR7_EL1 = 0x803e,
    /// The value that represents the system register DBGWCR7_EL1.
    DBGWCR7_EL1 = 0x803f,
    /// The value that represents the system register DBGBVR8_EL1.
    DBGBVR8_EL1 = 0x8044,
    /// The value that represents the system register DBGBCR8_EL1.
    DBGBCR8_EL1 = 0x8045,
    /// The value that represents the system register DBGWVR8_EL1.
    DBGWVR8_EL1 = 0x8046,
    /// The value that represents the system register DBGWCR8_EL1.
    DBGWCR8_EL1 = 0x8047,
    /// The value that represents the system register DBGBVR9_EL1.
    DBGBVR9_EL1 = 0x804c,
    /// The value that represents the system register DBGBCR9_EL1.
    DBGBCR9_EL1 = 0x804d,
    /// The value that represents the system register DBGWVR9_EL1.
    DBGWVR9_EL1 = 0x804e,
    /// The value that represents the system register DBGWCR9_EL1.
    DBGWCR9_EL1 = 0x804f,
    /// The value that represents the system register DBGBVR10_EL1.
    DBGBVR10_EL1 = 0x8054,
    /// The value that represents the system register DBGBCR10_EL1.
    DBGBCR10_EL1 = 0x8055,
    /// The value that represents the system register DBGWVR10_EL1.
    DBGWVR10_EL1 = 0x8056,
    /// The value that represents the system register DBGWCR10_EL1.
    DBGWCR10_EL1 = 0x8057,
    /// The value that represents the system register DBGBVR11_EL1.
    DBGBVR11_EL1 = 0x805c,
    /// The value that represents the system register DBGBCR11_EL1.
    DBGBCR11_EL1 = 0x805d,
    /// The value that represents the system register DBGWVR11_EL1.
    DBGWVR11_EL1 = 0x805e,
    /// The value that represents the system register DBGWCR11_EL1.
    DBGWCR11_EL1 = 0x805f,
    /// The value that represents the system register DBGBVR12_EL1.
    DBGBVR12_EL1 = 0x8064,
    /// The value that represents the system register DBGBCR12_EL1.
    DBGBCR12_EL1 = 0x8065,
    /// The value that represents the system register DBGWVR12_EL1.
    DBGWVR12_EL1 = 0x8066,
    /// The value that represents the system register DBGWCR12_EL1.
    DBGWCR12_EL1 = 0x8067,
    /// The value that represents the system register DBGBVR13_EL1.
    DBGBVR13_EL1 = 0x806c,
    /// The value that represents the system register DBGBCR13_EL1.
    DBGBCR13_EL1 = 0x806d,
    /// The value that represents the system register DBGWVR13_EL1.
    DBGWVR13_EL1 = 0x806e,
    /// The value that represents the system register DBGWCR13_EL1.
    DBGWCR13_EL1 = 0x806f,
    /// The value that represents the system register DBGBVR14_EL1.
    DBGBVR14_EL1 = 0x8074,
    /// The value that represents the system register DBGBCR14_EL1.
    DBGBCR14_EL1 = 0x8075,
    /// The value that represents the system register DBGWVR14_EL1.
    DBGWVR14_EL1 = 0x8076,
    /// The value that represents the system register DBGWCR14_EL1.
    DBGWCR14_EL1 = 0x8077,
    /// The value that represents the system register DBGBVR15_EL1.
    DBGBVR15_EL1 = 0x807c,
    /// The value that represents the system register DBGBCR15_EL1.
    DBGBCR15_EL1 = 0x807d,
    /// The value that represents the system register DBGWVR15_EL1.
    DBGWVR15_EL1 = 0x807e,
    /// The value that represents the system register DBGWCR15_EL1.
    DBGWCR15_EL1 = 0x807f,
    /// The value that represents the system register MIDR_EL1.
    MIDR_EL1 = 0xc000,
    /// The value that represents the system register MPIDR_EL1.
    MPIDR_EL1 = 0xc005,
    /// The value that describes the AArch64 Processor Feature Register 0.
    ID_AA64PFR0_EL1 = 0xc020,
    /// The value that describes the AArch64 Processor Feature Register 1.
    ID_AA64PFR1_EL1 = 0xc021,
    /// The value that describes the AArch64 SVE Feature ID register 0.
    #[cfg(feature = "macos-15-2")]
    ID_AA64ZFR0_EL1 = 0xc024,
    /// The value that describes the AArch64 SME Feature ID register 0.
    #[cfg(feature = "macos-15-2")]
    ID_AA64SMFR0_EL1 = 0xc025,
    /// The value that describes the AArch64 Debug Feature Register 0.
    ID_AA64DFR0_EL1 = 0xc028,
    /// The value that describes the AArch64 Debug Feature Register 1.
    ID_AA64DFR1_EL1 = 0xc029,
    /// The value that describes the AArch64 Instruction Set Attribute Register 0.
    ID_AA64ISAR0_EL1 = 0xc030,
    /// The value that describes the AArch64 Instruction Set Attribute Register 1.
    ID_AA64ISAR1_EL1 = 0xc031,
    /// The value that describes the AArch64 Memory Model Feature Register 0.
    ID_AA64MMFR0_EL1 = 0xc038,
    /// The value that describes the AArch64 Memory Model Feature Register 1.
    ID_AA64MMFR1_EL1 = 0xc039,
    /// The value that describes the AArch64 Memory Model Feature Register 2.
    ID_AA64MMFR2_EL1 = 0xc03a,
    /// The value that represents the system register SCTLR_EL1.
    SCTLR_EL1 = 0xc080,
    /// The value that represents the system register CPACR_EL1.
    CPACR_EL1 = 0xc082,
    /// The value that represents the system register ACTLR_EL1.
    ///
    /// This only allows getting / setting of the ACTLR_EL1.EnTSO bit (index 1). Setting this bit
    /// to 1 will cause the vcpu to use a TSO memory model, whereas clearing it will cause the vcpu
    /// to use the default ARM64 memory model (weakly ordered loads / stores).
    #[cfg(feature = "macos-15-0")]
    ACTLR_EL1 = 0xc081,
    /// The value that describes the Streaming Mode Priority Register.
    #[cfg(feature = "macos-15-2")]
    SMPRI_EL1 = 0xc094,
    /// The value that describes the SME Control Register.
    #[cfg(feature = "macos-15-2")]
    SMCR_EL1 = 0xc096,
    /// The value that represents the system register TTBR0_EL1.
    TTBR0_EL1 = 0xc100,
    /// The value that represents the system register TTBR1_EL1.
    TTBR1_EL1 = 0xc101,
    /// The value that represents the system register TCR_EL1.
    TCR_EL1 = 0xc102,
    /// The value that represents the system register APIAKEYLO_EL1.
    APIAKEYLO_EL1 = 0xc108,
    /// The value that represents the system register APIAKEYHI_EL1.
    APIAKEYHI_EL1 = 0xc109,
    /// The value that represents the system register APIBKEYLO_EL1.
    APIBKEYLO_EL1 = 0xc10a,
    /// The value that represents the system register APIBKEYHI_EL1.
    APIBKEYHI_EL1 = 0xc10b,
    /// The value that represents the system register APDAKEYLO_EL1.
    APDAKEYLO_EL1 = 0xc110,
    /// The value that represents the system register APDAKEYHI_EL1.
    APDAKEYHI_EL1 = 0xc111,
    /// The value that represents the system register APDBKEYLO_EL1.
    APDBKEYLO_EL1 = 0xc112,
    /// The value that represents the system register APDBKEYHI_EL1.
    APDBKEYHI_EL1 = 0xc113,
    /// The value that represents the system register APGAKEYLO_EL1.
    APGAKEYLO_EL1 = 0xc118,
    /// The value that represents the system register APGAKEYHI_EL1.
    APGAKEYHI_EL1 = 0xc119,
    /// The value that represents the system register SPSR_EL1.
    SPSR_EL1 = 0xc200,
    /// The value that represents the system register ELR_EL1.
    ELR_EL1 = 0xc201,
    /// The value that represents the system register SP_EL0.
    SP_EL0 = 0xc208,
    /// The value that represents the system register AFSR0_EL1.
    AFSR0_EL1 = 0xc288,
    /// The value that represents the system register AFSR1_EL1.
    AFSR1_EL1 = 0xc289,
    /// The value that represents the system register ESR_EL1.
    ESR_EL1 = 0xc290,
    /// The value that represents the system register FAR_EL1.
    FAR_EL1 = 0xc300,
    /// The value that represents the system register PAR_EL1.
    PAR_EL1 = 0xc3a0,
    /// The value that represents the system register MAIR_EL1.
    MAIR_EL1 = 0xc510,
    /// The value that represents the system register AMAIR_EL1.
    AMAIR_EL1 = 0xc518,
    /// The value that represents the system register VBAR_EL1.
    VBAR_EL1 = 0xc600,
    /// The value that represents the system register CONTEXTIDR_EL1.
    CONTEXTIDR_EL1 = 0xc681,
    /// The value that represents the system register TPIDR_EL1.
    TPIDR_EL1 = 0xc684,
    /// The value that represents the system register SCXTNUM_EL1.
    #[cfg(feature = "macos-15-2")]
    SCXTNUM_EL1 = 0xc687,
    /// The value that represents the system register CNTKCTL_EL1.
    CNTKCTL_EL1 = 0xc708,
    /// The value that represents the system register CSSELR_EL1.
    CSSELR_EL1 = 0xd000,
    /// The value that represents the system register TPIDR_EL0.
    TPIDR_EL0 = 0xde82,
    /// The value that represents the system register TPIDRRO_EL0.
    TPIDRRO_EL0 = 0xde83,
    /// The value that represents the system register TPIDR2_EL0.
    #[cfg(feature = "macos-15-2")]
    TPIDR2_EL0 = 0xde85,
    /// The value that represents the system register SCXTNUM_EL0.
    #[cfg(feature = "macos-15-2")]
    SCXTNUM_EL0 = 0xde87,
    /// The value that represents the system register CNTV_CTL_EL0.
    CNTV_CTL_EL0 = 0xdf19,
    /// The value that represents the system register CNTV_CVAL_EL0.
    CNTV_CVAL_EL0 = 0xdf1a,
    /// The value that represents the system register SP_EL1.
    SP_EL1 = 0xe208,
    /// The value that represents the system register CNTP_CTL_EL0.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CNTP_CTL_EL0 = 0xdf11,
    /// The value that represents the system register CNTP_CVAL_EL0.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CNTP_CVAL_EL0 = 0xdf12,
    /// The value that represents the system register CNTP_TVAL_EL0.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CNTP_TVAL_EL0 = 0xdf10,
    /// The value that represents the system register CNTHCTL_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CNTHCTL_EL2 = 0xe708,
    /// The value that represents the system register CNTHP_CTL_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CNTHP_CTL_EL2 = 0xe711,
    /// The value that represents the system register CNTHP_CVAL_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CNTHP_CVAL_EL2 = 0xe712,
    /// The value that represents the system register CNTHP_TVAL_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CNTHP_TVAL_EL2 = 0xe710,
    /// The value that represents the system register CNTVOFF_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CNTVOFF_EL2 = 0xe703,
    /// The value that represents the system register CPTR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    CPTR_EL2 = 0xe08a,
    /// The value that represents the system register ELR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    ELR_EL2 = 0xe201,
    /// The value that represents the system register ESR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    ESR_EL2 = 0xe290,
    /// The value that represents the system register FAR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    FAR_EL2 = 0xe300,
    /// The value that represents the system register HCR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    HCR_EL2 = 0xe088,
    /// The value that represents the system register HPFAR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    HPFAR_EL2 = 0xe304,
    /// The value that represents the system register MAIR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    MAIR_EL2 = 0xe510,
    /// The value that represents the system register MDCR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    MDCR_EL2 = 0xe019,
    /// The value that represents the system register SCTLR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    SCTLR_EL2 = 0xe080,
    /// The value that represents the system register SPSR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    SPSR_EL2 = 0xe200,
    /// The value that represents the system register SP_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    SP_EL2 = 0xf208,
    /// The value that represents the system register TCR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    TCR_EL2 = 0xe102,
    /// The value that represents the system register TPIDR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    TPIDR_EL2 = 0xe682,
    /// The value that represents the system register TTBR0_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    TTBR0_EL2 = 0xe100,
    /// The value that represents the system register TTBR1_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    TTBR1_EL2 = 0xe101,
    /// The value that represents the system register VBAR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    VBAR_EL2 = 0xe600,
    /// The value that represents the system register VMPIDR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    VMPIDR_EL2 = 0xe005,
    /// The value that represents the system register VPIDR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    VPIDR_EL2 = 0xe000,
    /// The value that represents the system register VTCR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    VTCR_EL2 = 0xe10a,
    /// The value that represents the system register VTTBR_EL2.
    /// This register is only available if EL2 was enabled in the VM configuration.
    #[cfg(feature = "macos-15-0")]
    VTTBR_EL2 = 0xe108,
}

unsafe extern "C" {
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

unsafe extern "C" {
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

/// The value that represents no memory permission.
pub const HV_MEMORY_NONE: hv_memory_flags_t = 0u64;
/// The value that represents the memory-read permission.
pub const HV_MEMORY_READ: hv_memory_flags_t = 1u64 << 0;
/// The value that represents the memory-write permission.
pub const HV_MEMORY_WRITE: hv_memory_flags_t = 1u64 << 1;
/// The value that represents the memory-execute permission.
pub const HV_MEMORY_EXEC: hv_memory_flags_t = 1u64 << 2;

unsafe extern "C" {
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

unsafe extern "C" {
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

// -----------------------------------------------------------------------------------------------
// Global Interrupt Controller
// -----------------------------------------------------------------------------------------------

/// Configuration for [`hv_gic_create`].
pub type hv_gic_config_t = *mut c_void;

/// GIC state for [`hv_gic_state_get_data`] and [`hv_gic_state_get_size`]
pub type hv_gic_state_t = *mut c_void;

/// Type of an ARM GIC interrupt id.
///
/// # Discussion
///
/// Note that [`hv_gic_intid_t::MAINTENANCE`] and [`hv_gic_intid_t::EL2_PHYSICAL_TIMER`] are only
/// present when EL2 (nested virtualization) is enabled.
#[repr(C)]
#[cfg(feature = "macos-15-0")]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_gic_intid_t {
    PERFORMANCE_MONITOR = 23,
    MAINTENANCE = 25,
    EL2_PHYSICAL_TIMER = 26,
    EL1_VIRTUAL_TIMER = 27,
    EL1_PHYSICAL_TIMER = 30,
}

/// Type of an ARM GIC distributor register.
#[repr(C)]
#[cfg(feature = "macos-15-0")]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_gic_distributor_reg_t {
    CTLR = 0x0000,
    TYPER = 0x0004,

    IGROUPR0 = 0x0080,
    IGROUPR1 = 0x0084,
    IGROUPR2 = 0x0088,
    IGROUPR3 = 0x008c,
    IGROUPR4 = 0x0090,
    IGROUPR5 = 0x0094,
    IGROUPR6 = 0x0098,
    IGROUPR7 = 0x009c,
    IGROUPR8 = 0x00a0,
    IGROUPR9 = 0x00a4,
    IGROUPR10 = 0x00a8,
    IGROUPR11 = 0x00ac,
    IGROUPR12 = 0x00b0,
    IGROUPR13 = 0x00b4,
    IGROUPR14 = 0x00b8,
    IGROUPR15 = 0x00bc,
    IGROUPR16 = 0x00c0,
    IGROUPR17 = 0x00c4,
    IGROUPR18 = 0x00c8,
    IGROUPR19 = 0x00cc,
    IGROUPR20 = 0x00d0,
    IGROUPR21 = 0x00d4,
    IGROUPR22 = 0x00d8,
    IGROUPR23 = 0x00dc,
    IGROUPR24 = 0x00e0,
    IGROUPR25 = 0x00e4,
    IGROUPR26 = 0x00e8,
    IGROUPR27 = 0x00ec,
    IGROUPR28 = 0x00f0,
    IGROUPR29 = 0x00f4,
    IGROUPR30 = 0x00f8,
    IGROUPR31 = 0x00fc,

    ISENABLER0 = 0x0100,
    ISENABLER1 = 0x0104,
    ISENABLER2 = 0x0108,
    ISENABLER3 = 0x010c,
    ISENABLER4 = 0x0110,
    ISENABLER5 = 0x0114,
    ISENABLER6 = 0x0118,
    ISENABLER7 = 0x011c,
    ISENABLER8 = 0x0120,
    ISENABLER9 = 0x0124,
    ISENABLER10 = 0x0128,
    ISENABLER11 = 0x012c,
    ISENABLER12 = 0x0130,
    ISENABLER13 = 0x0134,
    ISENABLER14 = 0x0138,
    ISENABLER15 = 0x013c,
    ISENABLER16 = 0x0140,
    ISENABLER17 = 0x0144,
    ISENABLER18 = 0x0148,
    ISENABLER19 = 0x014c,
    ISENABLER20 = 0x0150,
    ISENABLER21 = 0x0154,
    ISENABLER22 = 0x0158,
    ISENABLER23 = 0x015c,
    ISENABLER24 = 0x0160,
    ISENABLER25 = 0x0164,
    ISENABLER26 = 0x0168,
    ISENABLER27 = 0x016c,
    ISENABLER28 = 0x0170,
    ISENABLER29 = 0x0174,
    ISENABLER30 = 0x0178,
    ISENABLER31 = 0x017c,

    ICENABLER0 = 0x0180,
    ICENABLER1 = 0x0184,
    ICENABLER2 = 0x0188,
    ICENABLER3 = 0x018c,
    ICENABLER4 = 0x0190,
    ICENABLER5 = 0x0194,
    ICENABLER6 = 0x0198,
    ICENABLER7 = 0x019c,
    ICENABLER8 = 0x01a0,
    ICENABLER9 = 0x01a4,
    ICENABLER10 = 0x01a8,
    ICENABLER11 = 0x01ac,
    ICENABLER12 = 0x01b0,
    ICENABLER13 = 0x01b4,
    ICENABLER14 = 0x01b8,
    ICENABLER15 = 0x01bc,
    ICENABLER16 = 0x01c0,
    ICENABLER17 = 0x01c4,
    ICENABLER18 = 0x01c8,
    ICENABLER19 = 0x01cc,
    ICENABLER20 = 0x01d0,
    ICENABLER21 = 0x01d4,
    ICENABLER22 = 0x01d8,
    ICENABLER23 = 0x01dc,
    ICENABLER24 = 0x01e0,
    ICENABLER25 = 0x01e4,
    ICENABLER26 = 0x01e8,
    ICENABLER27 = 0x01ec,
    ICENABLER28 = 0x01f0,
    ICENABLER29 = 0x01f4,
    ICENABLER30 = 0x01f8,
    ICENABLER31 = 0x01fc,

    ISPENDR0 = 0x0200,
    ISPENDR1 = 0x0204,
    ISPENDR2 = 0x0208,
    ISPENDR3 = 0x020c,
    ISPENDR4 = 0x0210,
    ISPENDR5 = 0x0214,
    ISPENDR6 = 0x0218,
    ISPENDR7 = 0x021c,
    ISPENDR8 = 0x0220,
    ISPENDR9 = 0x0224,
    ISPENDR10 = 0x0228,
    ISPENDR11 = 0x022c,
    ISPENDR12 = 0x0230,
    ISPENDR13 = 0x0234,
    ISPENDR14 = 0x0238,
    ISPENDR15 = 0x023c,
    ISPENDR16 = 0x0240,
    ISPENDR17 = 0x0244,
    ISPENDR18 = 0x0248,
    ISPENDR19 = 0x024c,
    ISPENDR20 = 0x0250,
    ISPENDR21 = 0x0254,
    ISPENDR22 = 0x0258,
    ISPENDR23 = 0x025c,
    ISPENDR24 = 0x0260,
    ISPENDR25 = 0x0264,
    ISPENDR26 = 0x0268,
    ISPENDR27 = 0x026c,
    ISPENDR28 = 0x0270,
    ISPENDR29 = 0x0274,
    ISPENDR30 = 0x0278,
    ISPENDR31 = 0x027c,

    ICPENDR0 = 0x0280,
    ICPENDR1 = 0x0284,
    ICPENDR2 = 0x0288,
    ICPENDR3 = 0x028c,
    ICPENDR4 = 0x0290,
    ICPENDR5 = 0x0294,
    ICPENDR6 = 0x0298,
    ICPENDR7 = 0x029c,
    ICPENDR8 = 0x02a0,
    ICPENDR9 = 0x02a4,
    ICPENDR10 = 0x02a8,
    ICPENDR11 = 0x02ac,
    ICPENDR12 = 0x02b0,
    ICPENDR13 = 0x02b4,
    ICPENDR14 = 0x02b8,
    ICPENDR15 = 0x02bc,
    ICPENDR16 = 0x02c0,
    ICPENDR17 = 0x02c4,
    ICPENDR18 = 0x02c8,
    ICPENDR19 = 0x02cc,
    ICPENDR20 = 0x02d0,
    ICPENDR21 = 0x02d4,
    ICPENDR22 = 0x02d8,
    ICPENDR23 = 0x02dc,
    ICPENDR24 = 0x02e0,
    ICPENDR25 = 0x02e4,
    ICPENDR26 = 0x02e8,
    ICPENDR27 = 0x02ec,
    ICPENDR28 = 0x02f0,
    ICPENDR29 = 0x02f4,
    ICPENDR30 = 0x02f8,
    ICPENDR31 = 0x02fc,

    ISACTIVER0 = 0x0300,
    ISACTIVER1 = 0x0304,
    ISACTIVER2 = 0x0308,
    ISACTIVER3 = 0x030c,
    ISACTIVER4 = 0x0310,
    ISACTIVER5 = 0x0314,
    ISACTIVER6 = 0x0318,
    ISACTIVER7 = 0x031c,
    ISACTIVER8 = 0x0320,
    ISACTIVER9 = 0x0324,
    ISACTIVER10 = 0x0328,
    ISACTIVER11 = 0x032c,
    ISACTIVER12 = 0x0330,
    ISACTIVER13 = 0x0334,
    ISACTIVER14 = 0x0338,
    ISACTIVER15 = 0x033c,
    ISACTIVER16 = 0x0340,
    ISACTIVER17 = 0x0344,
    ISACTIVER18 = 0x0348,
    ISACTIVER19 = 0x034c,
    ISACTIVER20 = 0x0350,
    ISACTIVER21 = 0x0354,
    ISACTIVER22 = 0x0358,
    ISACTIVER23 = 0x035c,
    ISACTIVER24 = 0x0360,
    ISACTIVER25 = 0x0364,
    ISACTIVER26 = 0x0368,
    ISACTIVER27 = 0x036c,
    ISACTIVER28 = 0x0370,
    ISACTIVER29 = 0x0374,
    ISACTIVER30 = 0x0378,
    ISACTIVER31 = 0x037c,

    ICACTIVER0 = 0x0380,
    ICACTIVER1 = 0x0384,
    ICACTIVER2 = 0x0388,
    ICACTIVER3 = 0x038c,
    ICACTIVER4 = 0x0390,
    ICACTIVER5 = 0x0394,
    ICACTIVER6 = 0x0398,
    ICACTIVER7 = 0x039c,
    ICACTIVER8 = 0x03a0,
    ICACTIVER9 = 0x03a4,
    ICACTIVER10 = 0x03a8,
    ICACTIVER11 = 0x03ac,
    ICACTIVER12 = 0x03b0,
    ICACTIVER13 = 0x03b4,
    ICACTIVER14 = 0x03b8,
    ICACTIVER15 = 0x03bc,
    ICACTIVER16 = 0x03c0,
    ICACTIVER17 = 0x03c4,
    ICACTIVER18 = 0x03c8,
    ICACTIVER19 = 0x03cc,
    ICACTIVER20 = 0x03d0,
    ICACTIVER21 = 0x03d4,
    ICACTIVER22 = 0x03d8,
    ICACTIVER23 = 0x03dc,
    ICACTIVER24 = 0x03e0,
    ICACTIVER25 = 0x03e4,
    ICACTIVER26 = 0x03e8,
    ICACTIVER27 = 0x03ec,
    ICACTIVER28 = 0x03f0,
    ICACTIVER29 = 0x03f4,
    ICACTIVER30 = 0x03f8,
    ICACTIVER31 = 0x03fc,

    IPRIORITYR0 = 0x0400,
    IPRIORITYR1 = 0x0404,
    IPRIORITYR2 = 0x0408,
    IPRIORITYR3 = 0x040c,
    IPRIORITYR4 = 0x0410,
    IPRIORITYR5 = 0x0414,
    IPRIORITYR6 = 0x0418,
    IPRIORITYR7 = 0x041c,
    IPRIORITYR8 = 0x0420,
    IPRIORITYR9 = 0x0424,
    IPRIORITYR10 = 0x0428,
    IPRIORITYR11 = 0x042c,
    IPRIORITYR12 = 0x0430,
    IPRIORITYR13 = 0x0434,
    IPRIORITYR14 = 0x0438,
    IPRIORITYR15 = 0x043c,
    IPRIORITYR16 = 0x0440,
    IPRIORITYR17 = 0x0444,
    IPRIORITYR18 = 0x0448,
    IPRIORITYR19 = 0x044c,
    IPRIORITYR20 = 0x0450,
    IPRIORITYR21 = 0x0454,
    IPRIORITYR22 = 0x0458,
    IPRIORITYR23 = 0x045c,
    IPRIORITYR24 = 0x0460,
    IPRIORITYR25 = 0x0464,
    IPRIORITYR26 = 0x0468,
    IPRIORITYR27 = 0x046c,
    IPRIORITYR28 = 0x0470,
    IPRIORITYR29 = 0x0474,
    IPRIORITYR30 = 0x0478,
    IPRIORITYR31 = 0x047c,
    IPRIORITYR32 = 0x0480,
    IPRIORITYR33 = 0x0484,
    IPRIORITYR34 = 0x0488,
    IPRIORITYR35 = 0x048c,
    IPRIORITYR36 = 0x0490,
    IPRIORITYR37 = 0x0494,
    IPRIORITYR38 = 0x0498,
    IPRIORITYR39 = 0x049c,
    IPRIORITYR40 = 0x04a0,
    IPRIORITYR41 = 0x04a4,
    IPRIORITYR42 = 0x04a8,
    IPRIORITYR43 = 0x04ac,
    IPRIORITYR44 = 0x04b0,
    IPRIORITYR45 = 0x04b4,
    IPRIORITYR46 = 0x04b8,
    IPRIORITYR47 = 0x04bc,
    IPRIORITYR48 = 0x04c0,
    IPRIORITYR49 = 0x04c4,
    IPRIORITYR50 = 0x04c8,
    IPRIORITYR51 = 0x04cc,
    IPRIORITYR52 = 0x04d0,
    IPRIORITYR53 = 0x04d4,
    IPRIORITYR54 = 0x04d8,
    IPRIORITYR55 = 0x04dc,
    IPRIORITYR56 = 0x04e0,
    IPRIORITYR57 = 0x04e4,
    IPRIORITYR58 = 0x04e8,
    IPRIORITYR59 = 0x04ec,
    IPRIORITYR60 = 0x04f0,
    IPRIORITYR61 = 0x04f4,
    IPRIORITYR62 = 0x04f8,
    IPRIORITYR63 = 0x04fc,
    IPRIORITYR64 = 0x0500,
    IPRIORITYR65 = 0x0504,
    IPRIORITYR66 = 0x0508,
    IPRIORITYR67 = 0x050c,
    IPRIORITYR68 = 0x0510,
    IPRIORITYR69 = 0x0514,
    IPRIORITYR70 = 0x0518,
    IPRIORITYR71 = 0x051c,
    IPRIORITYR72 = 0x0520,
    IPRIORITYR73 = 0x0524,
    IPRIORITYR74 = 0x0528,
    IPRIORITYR75 = 0x052c,
    IPRIORITYR76 = 0x0530,
    IPRIORITYR77 = 0x0534,
    IPRIORITYR78 = 0x0538,
    IPRIORITYR79 = 0x053c,
    IPRIORITYR80 = 0x0540,
    IPRIORITYR81 = 0x0544,
    IPRIORITYR82 = 0x0548,
    IPRIORITYR83 = 0x054c,
    IPRIORITYR84 = 0x0550,
    IPRIORITYR85 = 0x0554,
    IPRIORITYR86 = 0x0558,
    IPRIORITYR87 = 0x055c,
    IPRIORITYR88 = 0x0560,
    IPRIORITYR89 = 0x0564,
    IPRIORITYR90 = 0x0568,
    IPRIORITYR91 = 0x056c,
    IPRIORITYR92 = 0x0570,
    IPRIORITYR93 = 0x0574,
    IPRIORITYR94 = 0x0578,
    IPRIORITYR95 = 0x057c,
    IPRIORITYR96 = 0x0580,
    IPRIORITYR97 = 0x0584,
    IPRIORITYR98 = 0x0588,
    IPRIORITYR99 = 0x058c,
    IPRIORITYR100 = 0x0590,
    IPRIORITYR101 = 0x0594,
    IPRIORITYR102 = 0x0598,
    IPRIORITYR103 = 0x059c,
    IPRIORITYR104 = 0x05a0,
    IPRIORITYR105 = 0x05a4,
    IPRIORITYR106 = 0x05a8,
    IPRIORITYR107 = 0x05ac,
    IPRIORITYR108 = 0x05b0,
    IPRIORITYR109 = 0x05b4,
    IPRIORITYR110 = 0x05b8,
    IPRIORITYR111 = 0x05bc,
    IPRIORITYR112 = 0x05c0,
    IPRIORITYR113 = 0x05c4,
    IPRIORITYR114 = 0x05c8,
    IPRIORITYR115 = 0x05cc,
    IPRIORITYR116 = 0x05d0,
    IPRIORITYR117 = 0x05d4,
    IPRIORITYR118 = 0x05d8,
    IPRIORITYR119 = 0x05dc,
    IPRIORITYR120 = 0x05e0,
    IPRIORITYR121 = 0x05e4,
    IPRIORITYR122 = 0x05e8,
    IPRIORITYR123 = 0x05ec,
    IPRIORITYR124 = 0x05f0,
    IPRIORITYR125 = 0x05f4,
    IPRIORITYR126 = 0x05f8,
    IPRIORITYR127 = 0x05fc,
    IPRIORITYR128 = 0x0600,
    IPRIORITYR129 = 0x0604,
    IPRIORITYR130 = 0x0608,
    IPRIORITYR131 = 0x060c,
    IPRIORITYR132 = 0x0610,
    IPRIORITYR133 = 0x0614,
    IPRIORITYR134 = 0x0618,
    IPRIORITYR135 = 0x061c,
    IPRIORITYR136 = 0x0620,
    IPRIORITYR137 = 0x0624,
    IPRIORITYR138 = 0x0628,
    IPRIORITYR139 = 0x062c,
    IPRIORITYR140 = 0x0630,
    IPRIORITYR141 = 0x0634,
    IPRIORITYR142 = 0x0638,
    IPRIORITYR143 = 0x063c,
    IPRIORITYR144 = 0x0640,
    IPRIORITYR145 = 0x0644,
    IPRIORITYR146 = 0x0648,
    IPRIORITYR147 = 0x064c,
    IPRIORITYR148 = 0x0650,
    IPRIORITYR149 = 0x0654,
    IPRIORITYR150 = 0x0658,
    IPRIORITYR151 = 0x065c,
    IPRIORITYR152 = 0x0660,
    IPRIORITYR153 = 0x0664,
    IPRIORITYR154 = 0x0668,
    IPRIORITYR155 = 0x066c,
    IPRIORITYR156 = 0x0670,
    IPRIORITYR157 = 0x0674,
    IPRIORITYR158 = 0x0678,
    IPRIORITYR159 = 0x067c,
    IPRIORITYR160 = 0x0680,
    IPRIORITYR161 = 0x0684,
    IPRIORITYR162 = 0x0688,
    IPRIORITYR163 = 0x068c,
    IPRIORITYR164 = 0x0690,
    IPRIORITYR165 = 0x0694,
    IPRIORITYR166 = 0x0698,
    IPRIORITYR167 = 0x069c,
    IPRIORITYR168 = 0x06a0,
    IPRIORITYR169 = 0x06a4,
    IPRIORITYR170 = 0x06a8,
    IPRIORITYR171 = 0x06ac,
    IPRIORITYR172 = 0x06b0,
    IPRIORITYR173 = 0x06b4,
    IPRIORITYR174 = 0x06b8,
    IPRIORITYR175 = 0x06bc,
    IPRIORITYR176 = 0x06c0,
    IPRIORITYR177 = 0x06c4,
    IPRIORITYR178 = 0x06c8,
    IPRIORITYR179 = 0x06cc,
    IPRIORITYR180 = 0x06d0,
    IPRIORITYR181 = 0x06d4,
    IPRIORITYR182 = 0x06d8,
    IPRIORITYR183 = 0x06dc,
    IPRIORITYR184 = 0x06e0,
    IPRIORITYR185 = 0x06e4,
    IPRIORITYR186 = 0x06e8,
    IPRIORITYR187 = 0x06ec,
    IPRIORITYR188 = 0x06f0,
    IPRIORITYR189 = 0x06f4,
    IPRIORITYR190 = 0x06f8,
    IPRIORITYR191 = 0x06fc,
    IPRIORITYR192 = 0x0700,
    IPRIORITYR193 = 0x0704,
    IPRIORITYR194 = 0x0708,
    IPRIORITYR195 = 0x070c,
    IPRIORITYR196 = 0x0710,
    IPRIORITYR197 = 0x0714,
    IPRIORITYR198 = 0x0718,
    IPRIORITYR199 = 0x071c,
    IPRIORITYR200 = 0x0720,
    IPRIORITYR201 = 0x0724,
    IPRIORITYR202 = 0x0728,
    IPRIORITYR203 = 0x072c,
    IPRIORITYR204 = 0x0730,
    IPRIORITYR205 = 0x0734,
    IPRIORITYR206 = 0x0738,
    IPRIORITYR207 = 0x073c,
    IPRIORITYR208 = 0x0740,
    IPRIORITYR209 = 0x0744,
    IPRIORITYR210 = 0x0748,
    IPRIORITYR211 = 0x074c,
    IPRIORITYR212 = 0x0750,
    IPRIORITYR213 = 0x0754,
    IPRIORITYR214 = 0x0758,
    IPRIORITYR215 = 0x075c,
    IPRIORITYR216 = 0x0760,
    IPRIORITYR217 = 0x0764,
    IPRIORITYR218 = 0x0768,
    IPRIORITYR219 = 0x076c,
    IPRIORITYR220 = 0x0770,
    IPRIORITYR221 = 0x0774,
    IPRIORITYR222 = 0x0778,
    IPRIORITYR223 = 0x077c,
    IPRIORITYR224 = 0x0780,
    IPRIORITYR225 = 0x0784,
    IPRIORITYR226 = 0x0788,
    IPRIORITYR227 = 0x078c,
    IPRIORITYR228 = 0x0790,
    IPRIORITYR229 = 0x0794,
    IPRIORITYR230 = 0x0798,
    IPRIORITYR231 = 0x079c,
    IPRIORITYR232 = 0x07a0,
    IPRIORITYR233 = 0x07a4,
    IPRIORITYR234 = 0x07a8,
    IPRIORITYR235 = 0x07ac,
    IPRIORITYR236 = 0x07b0,
    IPRIORITYR237 = 0x07b4,
    IPRIORITYR238 = 0x07b8,
    IPRIORITYR239 = 0x07bc,
    IPRIORITYR240 = 0x07c0,
    IPRIORITYR241 = 0x07c4,
    IPRIORITYR242 = 0x07c8,
    IPRIORITYR243 = 0x07cc,
    IPRIORITYR244 = 0x07d0,
    IPRIORITYR245 = 0x07d4,
    IPRIORITYR246 = 0x07d8,
    IPRIORITYR247 = 0x07dc,
    IPRIORITYR248 = 0x07e0,
    IPRIORITYR249 = 0x07e4,
    IPRIORITYR250 = 0x07e8,
    IPRIORITYR251 = 0x07ec,
    IPRIORITYR252 = 0x07f0,
    IPRIORITYR253 = 0x07f4,
    IPRIORITYR254 = 0x07f8,

    ICFGR0 = 0x0c00,
    ICFGR1 = 0x0c04,
    ICFGR2 = 0x0c08,
    ICFGR3 = 0x0c0c,
    ICFGR4 = 0x0c10,
    ICFGR5 = 0x0c14,
    ICFGR6 = 0x0c18,
    ICFGR7 = 0x0c1c,
    ICFGR8 = 0x0c20,
    ICFGR9 = 0x0c24,
    ICFGR10 = 0x0c28,
    ICFGR11 = 0x0c2c,
    ICFGR12 = 0x0c30,
    ICFGR13 = 0x0c34,
    ICFGR14 = 0x0c38,
    ICFGR15 = 0x0c3c,
    ICFGR16 = 0x0c40,
    ICFGR17 = 0x0c44,
    ICFGR18 = 0x0c48,
    ICFGR19 = 0x0c4c,
    ICFGR20 = 0x0c50,
    ICFGR21 = 0x0c54,
    ICFGR22 = 0x0c58,
    ICFGR23 = 0x0c5c,
    ICFGR24 = 0x0c60,
    ICFGR25 = 0x0c64,
    ICFGR26 = 0x0c68,
    ICFGR27 = 0x0c6c,
    ICFGR28 = 0x0c70,
    ICFGR29 = 0x0c74,
    ICFGR30 = 0x0c78,
    ICFGR31 = 0x0c7c,
    ICFGR32 = 0x0c80,
    ICFGR33 = 0x0c84,
    ICFGR34 = 0x0c88,
    ICFGR35 = 0x0c8c,
    ICFGR36 = 0x0c90,
    ICFGR37 = 0x0c94,
    ICFGR38 = 0x0c98,
    ICFGR39 = 0x0c9c,
    ICFGR40 = 0x0ca0,
    ICFGR41 = 0x0ca4,
    ICFGR42 = 0x0ca8,
    ICFGR43 = 0x0cac,
    ICFGR44 = 0x0cb0,
    ICFGR45 = 0x0cb4,
    ICFGR46 = 0x0cb8,
    ICFGR47 = 0x0cbc,
    ICFGR48 = 0x0cc0,
    ICFGR49 = 0x0cc4,
    ICFGR50 = 0x0cc8,
    ICFGR51 = 0x0ccc,
    ICFGR52 = 0x0cd0,
    ICFGR53 = 0x0cd4,
    ICFGR54 = 0x0cd8,
    ICFGR55 = 0x0cdc,
    ICFGR56 = 0x0ce0,
    ICFGR57 = 0x0ce4,
    ICFGR58 = 0x0ce8,
    ICFGR59 = 0x0cec,
    ICFGR60 = 0x0cf0,
    ICFGR61 = 0x0cf4,
    ICFGR62 = 0x0cf8,
    ICFGR63 = 0x0cfc,

    IROUTER32 = 0x6100,
    IROUTER33 = 0x6108,
    IROUTER34 = 0x6110,
    IROUTER35 = 0x6118,
    IROUTER36 = 0x6120,
    IROUTER37 = 0x6128,
    IROUTER38 = 0x6130,
    IROUTER39 = 0x6138,
    IROUTER40 = 0x6140,
    IROUTER41 = 0x6148,
    IROUTER42 = 0x6150,
    IROUTER43 = 0x6158,
    IROUTER44 = 0x6160,
    IROUTER45 = 0x6168,
    IROUTER46 = 0x6170,
    IROUTER47 = 0x6178,
    IROUTER48 = 0x6180,
    IROUTER49 = 0x6188,
    IROUTER50 = 0x6190,
    IROUTER51 = 0x6198,
    IROUTER52 = 0x61a0,
    IROUTER53 = 0x61a8,
    IROUTER54 = 0x61b0,
    IROUTER55 = 0x61b8,
    IROUTER56 = 0x61c0,
    IROUTER57 = 0x61c8,
    IROUTER58 = 0x61d0,
    IROUTER59 = 0x61d8,
    IROUTER60 = 0x61e0,
    IROUTER61 = 0x61e8,
    IROUTER62 = 0x61f0,
    IROUTER63 = 0x61f8,
    IROUTER64 = 0x6200,
    IROUTER65 = 0x6208,
    IROUTER66 = 0x6210,
    IROUTER67 = 0x6218,
    IROUTER68 = 0x6220,
    IROUTER69 = 0x6228,
    IROUTER70 = 0x6230,
    IROUTER71 = 0x6238,
    IROUTER72 = 0x6240,
    IROUTER73 = 0x6248,
    IROUTER74 = 0x6250,
    IROUTER75 = 0x6258,
    IROUTER76 = 0x6260,
    IROUTER77 = 0x6268,
    IROUTER78 = 0x6270,
    IROUTER79 = 0x6278,
    IROUTER80 = 0x6280,
    IROUTER81 = 0x6288,
    IROUTER82 = 0x6290,
    IROUTER83 = 0x6298,
    IROUTER84 = 0x62a0,
    IROUTER85 = 0x62a8,
    IROUTER86 = 0x62b0,
    IROUTER87 = 0x62b8,
    IROUTER88 = 0x62c0,
    IROUTER89 = 0x62c8,
    IROUTER90 = 0x62d0,
    IROUTER91 = 0x62d8,
    IROUTER92 = 0x62e0,
    IROUTER93 = 0x62e8,
    IROUTER94 = 0x62f0,
    IROUTER95 = 0x62f8,
    IROUTER96 = 0x6300,
    IROUTER97 = 0x6308,
    IROUTER98 = 0x6310,
    IROUTER99 = 0x6318,
    IROUTER100 = 0x6320,
    IROUTER101 = 0x6328,
    IROUTER102 = 0x6330,
    IROUTER103 = 0x6338,
    IROUTER104 = 0x6340,
    IROUTER105 = 0x6348,
    IROUTER106 = 0x6350,
    IROUTER107 = 0x6358,
    IROUTER108 = 0x6360,
    IROUTER109 = 0x6368,
    IROUTER110 = 0x6370,
    IROUTER111 = 0x6378,
    IROUTER112 = 0x6380,
    IROUTER113 = 0x6388,
    IROUTER114 = 0x6390,
    IROUTER115 = 0x6398,
    IROUTER116 = 0x63a0,
    IROUTER117 = 0x63a8,
    IROUTER118 = 0x63b0,
    IROUTER119 = 0x63b8,
    IROUTER120 = 0x63c0,
    IROUTER121 = 0x63c8,
    IROUTER122 = 0x63d0,
    IROUTER123 = 0x63d8,
    IROUTER124 = 0x63e0,
    IROUTER125 = 0x63e8,
    IROUTER126 = 0x63f0,
    IROUTER127 = 0x63f8,
    IROUTER128 = 0x6400,
    IROUTER129 = 0x6408,
    IROUTER130 = 0x6410,
    IROUTER131 = 0x6418,
    IROUTER132 = 0x6420,
    IROUTER133 = 0x6428,
    IROUTER134 = 0x6430,
    IROUTER135 = 0x6438,
    IROUTER136 = 0x6440,
    IROUTER137 = 0x6448,
    IROUTER138 = 0x6450,
    IROUTER139 = 0x6458,
    IROUTER140 = 0x6460,
    IROUTER141 = 0x6468,
    IROUTER142 = 0x6470,
    IROUTER143 = 0x6478,
    IROUTER144 = 0x6480,
    IROUTER145 = 0x6488,
    IROUTER146 = 0x6490,
    IROUTER147 = 0x6498,
    IROUTER148 = 0x64a0,
    IROUTER149 = 0x64a8,
    IROUTER150 = 0x64b0,
    IROUTER151 = 0x64b8,
    IROUTER152 = 0x64c0,
    IROUTER153 = 0x64c8,
    IROUTER154 = 0x64d0,
    IROUTER155 = 0x64d8,
    IROUTER156 = 0x64e0,
    IROUTER157 = 0x64e8,
    IROUTER158 = 0x64f0,
    IROUTER159 = 0x64f8,
    IROUTER160 = 0x6500,
    IROUTER161 = 0x6508,
    IROUTER162 = 0x6510,
    IROUTER163 = 0x6518,
    IROUTER164 = 0x6520,
    IROUTER165 = 0x6528,
    IROUTER166 = 0x6530,
    IROUTER167 = 0x6538,
    IROUTER168 = 0x6540,
    IROUTER169 = 0x6548,
    IROUTER170 = 0x6550,
    IROUTER171 = 0x6558,
    IROUTER172 = 0x6560,
    IROUTER173 = 0x6568,
    IROUTER174 = 0x6570,
    IROUTER175 = 0x6578,
    IROUTER176 = 0x6580,
    IROUTER177 = 0x6588,
    IROUTER178 = 0x6590,
    IROUTER179 = 0x6598,
    IROUTER180 = 0x65a0,
    IROUTER181 = 0x65a8,
    IROUTER182 = 0x65b0,
    IROUTER183 = 0x65b8,
    IROUTER184 = 0x65c0,
    IROUTER185 = 0x65c8,
    IROUTER186 = 0x65d0,
    IROUTER187 = 0x65d8,
    IROUTER188 = 0x65e0,
    IROUTER189 = 0x65e8,
    IROUTER190 = 0x65f0,
    IROUTER191 = 0x65f8,
    IROUTER192 = 0x6600,
    IROUTER193 = 0x6608,
    IROUTER194 = 0x6610,
    IROUTER195 = 0x6618,
    IROUTER196 = 0x6620,
    IROUTER197 = 0x6628,
    IROUTER198 = 0x6630,
    IROUTER199 = 0x6638,
    IROUTER200 = 0x6640,
    IROUTER201 = 0x6648,
    IROUTER202 = 0x6650,
    IROUTER203 = 0x6658,
    IROUTER204 = 0x6660,
    IROUTER205 = 0x6668,
    IROUTER206 = 0x6670,
    IROUTER207 = 0x6678,
    IROUTER208 = 0x6680,
    IROUTER209 = 0x6688,
    IROUTER210 = 0x6690,
    IROUTER211 = 0x6698,
    IROUTER212 = 0x66a0,
    IROUTER213 = 0x66a8,
    IROUTER214 = 0x66b0,
    IROUTER215 = 0x66b8,
    IROUTER216 = 0x66c0,
    IROUTER217 = 0x66c8,
    IROUTER218 = 0x66d0,
    IROUTER219 = 0x66d8,
    IROUTER220 = 0x66e0,
    IROUTER221 = 0x66e8,
    IROUTER222 = 0x66f0,
    IROUTER223 = 0x66f8,
    IROUTER224 = 0x6700,
    IROUTER225 = 0x6708,
    IROUTER226 = 0x6710,
    IROUTER227 = 0x6718,
    IROUTER228 = 0x6720,
    IROUTER229 = 0x6728,
    IROUTER230 = 0x6730,
    IROUTER231 = 0x6738,
    IROUTER232 = 0x6740,
    IROUTER233 = 0x6748,
    IROUTER234 = 0x6750,
    IROUTER235 = 0x6758,
    IROUTER236 = 0x6760,
    IROUTER237 = 0x6768,
    IROUTER238 = 0x6770,
    IROUTER239 = 0x6778,
    IROUTER240 = 0x6780,
    IROUTER241 = 0x6788,
    IROUTER242 = 0x6790,
    IROUTER243 = 0x6798,
    IROUTER244 = 0x67a0,
    IROUTER245 = 0x67a8,
    IROUTER246 = 0x67b0,
    IROUTER247 = 0x67b8,
    IROUTER248 = 0x67c0,
    IROUTER249 = 0x67c8,
    IROUTER250 = 0x67d0,
    IROUTER251 = 0x67d8,
    IROUTER252 = 0x67e0,
    IROUTER253 = 0x67e8,
    IROUTER254 = 0x67f0,
    IROUTER255 = 0x67f8,
    IROUTER256 = 0x6800,
    IROUTER257 = 0x6808,
    IROUTER258 = 0x6810,
    IROUTER259 = 0x6818,
    IROUTER260 = 0x6820,
    IROUTER261 = 0x6828,
    IROUTER262 = 0x6830,
    IROUTER263 = 0x6838,
    IROUTER264 = 0x6840,
    IROUTER265 = 0x6848,
    IROUTER266 = 0x6850,
    IROUTER267 = 0x6858,
    IROUTER268 = 0x6860,
    IROUTER269 = 0x6868,
    IROUTER270 = 0x6870,
    IROUTER271 = 0x6878,
    IROUTER272 = 0x6880,
    IROUTER273 = 0x6888,
    IROUTER274 = 0x6890,
    IROUTER275 = 0x6898,
    IROUTER276 = 0x68a0,
    IROUTER277 = 0x68a8,
    IROUTER278 = 0x68b0,
    IROUTER279 = 0x68b8,
    IROUTER280 = 0x68c0,
    IROUTER281 = 0x68c8,
    IROUTER282 = 0x68d0,
    IROUTER283 = 0x68d8,
    IROUTER284 = 0x68e0,
    IROUTER285 = 0x68e8,
    IROUTER286 = 0x68f0,
    IROUTER287 = 0x68f8,
    IROUTER288 = 0x6900,
    IROUTER289 = 0x6908,
    IROUTER290 = 0x6910,
    IROUTER291 = 0x6918,
    IROUTER292 = 0x6920,
    IROUTER293 = 0x6928,
    IROUTER294 = 0x6930,
    IROUTER295 = 0x6938,
    IROUTER296 = 0x6940,
    IROUTER297 = 0x6948,
    IROUTER298 = 0x6950,
    IROUTER299 = 0x6958,
    IROUTER300 = 0x6960,
    IROUTER301 = 0x6968,
    IROUTER302 = 0x6970,
    IROUTER303 = 0x6978,
    IROUTER304 = 0x6980,
    IROUTER305 = 0x6988,
    IROUTER306 = 0x6990,
    IROUTER307 = 0x6998,
    IROUTER308 = 0x69a0,
    IROUTER309 = 0x69a8,
    IROUTER310 = 0x69b0,
    IROUTER311 = 0x69b8,
    IROUTER312 = 0x69c0,
    IROUTER313 = 0x69c8,
    IROUTER314 = 0x69d0,
    IROUTER315 = 0x69d8,
    IROUTER316 = 0x69e0,
    IROUTER317 = 0x69e8,
    IROUTER318 = 0x69f0,
    IROUTER319 = 0x69f8,
    IROUTER320 = 0x6a00,
    IROUTER321 = 0x6a08,
    IROUTER322 = 0x6a10,
    IROUTER323 = 0x6a18,
    IROUTER324 = 0x6a20,
    IROUTER325 = 0x6a28,
    IROUTER326 = 0x6a30,
    IROUTER327 = 0x6a38,
    IROUTER328 = 0x6a40,
    IROUTER329 = 0x6a48,
    IROUTER330 = 0x6a50,
    IROUTER331 = 0x6a58,
    IROUTER332 = 0x6a60,
    IROUTER333 = 0x6a68,
    IROUTER334 = 0x6a70,
    IROUTER335 = 0x6a78,
    IROUTER336 = 0x6a80,
    IROUTER337 = 0x6a88,
    IROUTER338 = 0x6a90,
    IROUTER339 = 0x6a98,
    IROUTER340 = 0x6aa0,
    IROUTER341 = 0x6aa8,
    IROUTER342 = 0x6ab0,
    IROUTER343 = 0x6ab8,
    IROUTER344 = 0x6ac0,
    IROUTER345 = 0x6ac8,
    IROUTER346 = 0x6ad0,
    IROUTER347 = 0x6ad8,
    IROUTER348 = 0x6ae0,
    IROUTER349 = 0x6ae8,
    IROUTER350 = 0x6af0,
    IROUTER351 = 0x6af8,
    IROUTER352 = 0x6b00,
    IROUTER353 = 0x6b08,
    IROUTER354 = 0x6b10,
    IROUTER355 = 0x6b18,
    IROUTER356 = 0x6b20,
    IROUTER357 = 0x6b28,
    IROUTER358 = 0x6b30,
    IROUTER359 = 0x6b38,
    IROUTER360 = 0x6b40,
    IROUTER361 = 0x6b48,
    IROUTER362 = 0x6b50,
    IROUTER363 = 0x6b58,
    IROUTER364 = 0x6b60,
    IROUTER365 = 0x6b68,
    IROUTER366 = 0x6b70,
    IROUTER367 = 0x6b78,
    IROUTER368 = 0x6b80,
    IROUTER369 = 0x6b88,
    IROUTER370 = 0x6b90,
    IROUTER371 = 0x6b98,
    IROUTER372 = 0x6ba0,
    IROUTER373 = 0x6ba8,
    IROUTER374 = 0x6bb0,
    IROUTER375 = 0x6bb8,
    IROUTER376 = 0x6bc0,
    IROUTER377 = 0x6bc8,
    IROUTER378 = 0x6bd0,
    IROUTER379 = 0x6bd8,
    IROUTER380 = 0x6be0,
    IROUTER381 = 0x6be8,
    IROUTER382 = 0x6bf0,
    IROUTER383 = 0x6bf8,
    IROUTER384 = 0x6c00,
    IROUTER385 = 0x6c08,
    IROUTER386 = 0x6c10,
    IROUTER387 = 0x6c18,
    IROUTER388 = 0x6c20,
    IROUTER389 = 0x6c28,
    IROUTER390 = 0x6c30,
    IROUTER391 = 0x6c38,
    IROUTER392 = 0x6c40,
    IROUTER393 = 0x6c48,
    IROUTER394 = 0x6c50,
    IROUTER395 = 0x6c58,
    IROUTER396 = 0x6c60,
    IROUTER397 = 0x6c68,
    IROUTER398 = 0x6c70,
    IROUTER399 = 0x6c78,
    IROUTER400 = 0x6c80,
    IROUTER401 = 0x6c88,
    IROUTER402 = 0x6c90,
    IROUTER403 = 0x6c98,
    IROUTER404 = 0x6ca0,
    IROUTER405 = 0x6ca8,
    IROUTER406 = 0x6cb0,
    IROUTER407 = 0x6cb8,
    IROUTER408 = 0x6cc0,
    IROUTER409 = 0x6cc8,
    IROUTER410 = 0x6cd0,
    IROUTER411 = 0x6cd8,
    IROUTER412 = 0x6ce0,
    IROUTER413 = 0x6ce8,
    IROUTER414 = 0x6cf0,
    IROUTER415 = 0x6cf8,
    IROUTER416 = 0x6d00,
    IROUTER417 = 0x6d08,
    IROUTER418 = 0x6d10,
    IROUTER419 = 0x6d18,
    IROUTER420 = 0x6d20,
    IROUTER421 = 0x6d28,
    IROUTER422 = 0x6d30,
    IROUTER423 = 0x6d38,
    IROUTER424 = 0x6d40,
    IROUTER425 = 0x6d48,
    IROUTER426 = 0x6d50,
    IROUTER427 = 0x6d58,
    IROUTER428 = 0x6d60,
    IROUTER429 = 0x6d68,
    IROUTER430 = 0x6d70,
    IROUTER431 = 0x6d78,
    IROUTER432 = 0x6d80,
    IROUTER433 = 0x6d88,
    IROUTER434 = 0x6d90,
    IROUTER435 = 0x6d98,
    IROUTER436 = 0x6da0,
    IROUTER437 = 0x6da8,
    IROUTER438 = 0x6db0,
    IROUTER439 = 0x6db8,
    IROUTER440 = 0x6dc0,
    IROUTER441 = 0x6dc8,
    IROUTER442 = 0x6dd0,
    IROUTER443 = 0x6dd8,
    IROUTER444 = 0x6de0,
    IROUTER445 = 0x6de8,
    IROUTER446 = 0x6df0,
    IROUTER447 = 0x6df8,
    IROUTER448 = 0x6e00,
    IROUTER449 = 0x6e08,
    IROUTER450 = 0x6e10,
    IROUTER451 = 0x6e18,
    IROUTER452 = 0x6e20,
    IROUTER453 = 0x6e28,
    IROUTER454 = 0x6e30,
    IROUTER455 = 0x6e38,
    IROUTER456 = 0x6e40,
    IROUTER457 = 0x6e48,
    IROUTER458 = 0x6e50,
    IROUTER459 = 0x6e58,
    IROUTER460 = 0x6e60,
    IROUTER461 = 0x6e68,
    IROUTER462 = 0x6e70,
    IROUTER463 = 0x6e78,
    IROUTER464 = 0x6e80,
    IROUTER465 = 0x6e88,
    IROUTER466 = 0x6e90,
    IROUTER467 = 0x6e98,
    IROUTER468 = 0x6ea0,
    IROUTER469 = 0x6ea8,
    IROUTER470 = 0x6eb0,
    IROUTER471 = 0x6eb8,
    IROUTER472 = 0x6ec0,
    IROUTER473 = 0x6ec8,
    IROUTER474 = 0x6ed0,
    IROUTER475 = 0x6ed8,
    IROUTER476 = 0x6ee0,
    IROUTER477 = 0x6ee8,
    IROUTER478 = 0x6ef0,
    IROUTER479 = 0x6ef8,
    IROUTER480 = 0x6f00,
    IROUTER481 = 0x6f08,
    IROUTER482 = 0x6f10,
    IROUTER483 = 0x6f18,
    IROUTER484 = 0x6f20,
    IROUTER485 = 0x6f28,
    IROUTER486 = 0x6f30,
    IROUTER487 = 0x6f38,
    IROUTER488 = 0x6f40,
    IROUTER489 = 0x6f48,
    IROUTER490 = 0x6f50,
    IROUTER491 = 0x6f58,
    IROUTER492 = 0x6f60,
    IROUTER493 = 0x6f68,
    IROUTER494 = 0x6f70,
    IROUTER495 = 0x6f78,
    IROUTER496 = 0x6f80,
    IROUTER497 = 0x6f88,
    IROUTER498 = 0x6f90,
    IROUTER499 = 0x6f98,
    IROUTER500 = 0x6fa0,
    IROUTER501 = 0x6fa8,
    IROUTER502 = 0x6fb0,
    IROUTER503 = 0x6fb8,
    IROUTER504 = 0x6fc0,
    IROUTER505 = 0x6fc8,
    IROUTER506 = 0x6fd0,
    IROUTER507 = 0x6fd8,
    IROUTER508 = 0x6fe0,
    IROUTER509 = 0x6fe8,
    IROUTER510 = 0x6ff0,
    IROUTER511 = 0x6ff8,
    IROUTER512 = 0x7000,
    IROUTER513 = 0x7008,
    IROUTER514 = 0x7010,
    IROUTER515 = 0x7018,
    IROUTER516 = 0x7020,
    IROUTER517 = 0x7028,
    IROUTER518 = 0x7030,
    IROUTER519 = 0x7038,
    IROUTER520 = 0x7040,
    IROUTER521 = 0x7048,
    IROUTER522 = 0x7050,
    IROUTER523 = 0x7058,
    IROUTER524 = 0x7060,
    IROUTER525 = 0x7068,
    IROUTER526 = 0x7070,
    IROUTER527 = 0x7078,
    IROUTER528 = 0x7080,
    IROUTER529 = 0x7088,
    IROUTER530 = 0x7090,
    IROUTER531 = 0x7098,
    IROUTER532 = 0x70a0,
    IROUTER533 = 0x70a8,
    IROUTER534 = 0x70b0,
    IROUTER535 = 0x70b8,
    IROUTER536 = 0x70c0,
    IROUTER537 = 0x70c8,
    IROUTER538 = 0x70d0,
    IROUTER539 = 0x70d8,
    IROUTER540 = 0x70e0,
    IROUTER541 = 0x70e8,
    IROUTER542 = 0x70f0,
    IROUTER543 = 0x70f8,
    IROUTER544 = 0x7100,
    IROUTER545 = 0x7108,
    IROUTER546 = 0x7110,
    IROUTER547 = 0x7118,
    IROUTER548 = 0x7120,
    IROUTER549 = 0x7128,
    IROUTER550 = 0x7130,
    IROUTER551 = 0x7138,
    IROUTER552 = 0x7140,
    IROUTER553 = 0x7148,
    IROUTER554 = 0x7150,
    IROUTER555 = 0x7158,
    IROUTER556 = 0x7160,
    IROUTER557 = 0x7168,
    IROUTER558 = 0x7170,
    IROUTER559 = 0x7178,
    IROUTER560 = 0x7180,
    IROUTER561 = 0x7188,
    IROUTER562 = 0x7190,
    IROUTER563 = 0x7198,
    IROUTER564 = 0x71a0,
    IROUTER565 = 0x71a8,
    IROUTER566 = 0x71b0,
    IROUTER567 = 0x71b8,
    IROUTER568 = 0x71c0,
    IROUTER569 = 0x71c8,
    IROUTER570 = 0x71d0,
    IROUTER571 = 0x71d8,
    IROUTER572 = 0x71e0,
    IROUTER573 = 0x71e8,
    IROUTER574 = 0x71f0,
    IROUTER575 = 0x71f8,
    IROUTER576 = 0x7200,
    IROUTER577 = 0x7208,
    IROUTER578 = 0x7210,
    IROUTER579 = 0x7218,
    IROUTER580 = 0x7220,
    IROUTER581 = 0x7228,
    IROUTER582 = 0x7230,
    IROUTER583 = 0x7238,
    IROUTER584 = 0x7240,
    IROUTER585 = 0x7248,
    IROUTER586 = 0x7250,
    IROUTER587 = 0x7258,
    IROUTER588 = 0x7260,
    IROUTER589 = 0x7268,
    IROUTER590 = 0x7270,
    IROUTER591 = 0x7278,
    IROUTER592 = 0x7280,
    IROUTER593 = 0x7288,
    IROUTER594 = 0x7290,
    IROUTER595 = 0x7298,
    IROUTER596 = 0x72a0,
    IROUTER597 = 0x72a8,
    IROUTER598 = 0x72b0,
    IROUTER599 = 0x72b8,
    IROUTER600 = 0x72c0,
    IROUTER601 = 0x72c8,
    IROUTER602 = 0x72d0,
    IROUTER603 = 0x72d8,
    IROUTER604 = 0x72e0,
    IROUTER605 = 0x72e8,
    IROUTER606 = 0x72f0,
    IROUTER607 = 0x72f8,
    IROUTER608 = 0x7300,
    IROUTER609 = 0x7308,
    IROUTER610 = 0x7310,
    IROUTER611 = 0x7318,
    IROUTER612 = 0x7320,
    IROUTER613 = 0x7328,
    IROUTER614 = 0x7330,
    IROUTER615 = 0x7338,
    IROUTER616 = 0x7340,
    IROUTER617 = 0x7348,
    IROUTER618 = 0x7350,
    IROUTER619 = 0x7358,
    IROUTER620 = 0x7360,
    IROUTER621 = 0x7368,
    IROUTER622 = 0x7370,
    IROUTER623 = 0x7378,
    IROUTER624 = 0x7380,
    IROUTER625 = 0x7388,
    IROUTER626 = 0x7390,
    IROUTER627 = 0x7398,
    IROUTER628 = 0x73a0,
    IROUTER629 = 0x73a8,
    IROUTER630 = 0x73b0,
    IROUTER631 = 0x73b8,
    IROUTER632 = 0x73c0,
    IROUTER633 = 0x73c8,
    IROUTER634 = 0x73d0,
    IROUTER635 = 0x73d8,
    IROUTER636 = 0x73e0,
    IROUTER637 = 0x73e8,
    IROUTER638 = 0x73f0,
    IROUTER639 = 0x73f8,
    IROUTER640 = 0x7400,
    IROUTER641 = 0x7408,
    IROUTER642 = 0x7410,
    IROUTER643 = 0x7418,
    IROUTER644 = 0x7420,
    IROUTER645 = 0x7428,
    IROUTER646 = 0x7430,
    IROUTER647 = 0x7438,
    IROUTER648 = 0x7440,
    IROUTER649 = 0x7448,
    IROUTER650 = 0x7450,
    IROUTER651 = 0x7458,
    IROUTER652 = 0x7460,
    IROUTER653 = 0x7468,
    IROUTER654 = 0x7470,
    IROUTER655 = 0x7478,
    IROUTER656 = 0x7480,
    IROUTER657 = 0x7488,
    IROUTER658 = 0x7490,
    IROUTER659 = 0x7498,
    IROUTER660 = 0x74a0,
    IROUTER661 = 0x74a8,
    IROUTER662 = 0x74b0,
    IROUTER663 = 0x74b8,
    IROUTER664 = 0x74c0,
    IROUTER665 = 0x74c8,
    IROUTER666 = 0x74d0,
    IROUTER667 = 0x74d8,
    IROUTER668 = 0x74e0,
    IROUTER669 = 0x74e8,
    IROUTER670 = 0x74f0,
    IROUTER671 = 0x74f8,
    IROUTER672 = 0x7500,
    IROUTER673 = 0x7508,
    IROUTER674 = 0x7510,
    IROUTER675 = 0x7518,
    IROUTER676 = 0x7520,
    IROUTER677 = 0x7528,
    IROUTER678 = 0x7530,
    IROUTER679 = 0x7538,
    IROUTER680 = 0x7540,
    IROUTER681 = 0x7548,
    IROUTER682 = 0x7550,
    IROUTER683 = 0x7558,
    IROUTER684 = 0x7560,
    IROUTER685 = 0x7568,
    IROUTER686 = 0x7570,
    IROUTER687 = 0x7578,
    IROUTER688 = 0x7580,
    IROUTER689 = 0x7588,
    IROUTER690 = 0x7590,
    IROUTER691 = 0x7598,
    IROUTER692 = 0x75a0,
    IROUTER693 = 0x75a8,
    IROUTER694 = 0x75b0,
    IROUTER695 = 0x75b8,
    IROUTER696 = 0x75c0,
    IROUTER697 = 0x75c8,
    IROUTER698 = 0x75d0,
    IROUTER699 = 0x75d8,
    IROUTER700 = 0x75e0,
    IROUTER701 = 0x75e8,
    IROUTER702 = 0x75f0,
    IROUTER703 = 0x75f8,
    IROUTER704 = 0x7600,
    IROUTER705 = 0x7608,
    IROUTER706 = 0x7610,
    IROUTER707 = 0x7618,
    IROUTER708 = 0x7620,
    IROUTER709 = 0x7628,
    IROUTER710 = 0x7630,
    IROUTER711 = 0x7638,
    IROUTER712 = 0x7640,
    IROUTER713 = 0x7648,
    IROUTER714 = 0x7650,
    IROUTER715 = 0x7658,
    IROUTER716 = 0x7660,
    IROUTER717 = 0x7668,
    IROUTER718 = 0x7670,
    IROUTER719 = 0x7678,
    IROUTER720 = 0x7680,
    IROUTER721 = 0x7688,
    IROUTER722 = 0x7690,
    IROUTER723 = 0x7698,
    IROUTER724 = 0x76a0,
    IROUTER725 = 0x76a8,
    IROUTER726 = 0x76b0,
    IROUTER727 = 0x76b8,
    IROUTER728 = 0x76c0,
    IROUTER729 = 0x76c8,
    IROUTER730 = 0x76d0,
    IROUTER731 = 0x76d8,
    IROUTER732 = 0x76e0,
    IROUTER733 = 0x76e8,
    IROUTER734 = 0x76f0,
    IROUTER735 = 0x76f8,
    IROUTER736 = 0x7700,
    IROUTER737 = 0x7708,
    IROUTER738 = 0x7710,
    IROUTER739 = 0x7718,
    IROUTER740 = 0x7720,
    IROUTER741 = 0x7728,
    IROUTER742 = 0x7730,
    IROUTER743 = 0x7738,
    IROUTER744 = 0x7740,
    IROUTER745 = 0x7748,
    IROUTER746 = 0x7750,
    IROUTER747 = 0x7758,
    IROUTER748 = 0x7760,
    IROUTER749 = 0x7768,
    IROUTER750 = 0x7770,
    IROUTER751 = 0x7778,
    IROUTER752 = 0x7780,
    IROUTER753 = 0x7788,
    IROUTER754 = 0x7790,
    IROUTER755 = 0x7798,
    IROUTER756 = 0x77a0,
    IROUTER757 = 0x77a8,
    IROUTER758 = 0x77b0,
    IROUTER759 = 0x77b8,
    IROUTER760 = 0x77c0,
    IROUTER761 = 0x77c8,
    IROUTER762 = 0x77d0,
    IROUTER763 = 0x77d8,
    IROUTER764 = 0x77e0,
    IROUTER765 = 0x77e8,
    IROUTER766 = 0x77f0,
    IROUTER767 = 0x77f8,
    IROUTER768 = 0x7800,
    IROUTER769 = 0x7808,
    IROUTER770 = 0x7810,
    IROUTER771 = 0x7818,
    IROUTER772 = 0x7820,
    IROUTER773 = 0x7828,
    IROUTER774 = 0x7830,
    IROUTER775 = 0x7838,
    IROUTER776 = 0x7840,
    IROUTER777 = 0x7848,
    IROUTER778 = 0x7850,
    IROUTER779 = 0x7858,
    IROUTER780 = 0x7860,
    IROUTER781 = 0x7868,
    IROUTER782 = 0x7870,
    IROUTER783 = 0x7878,
    IROUTER784 = 0x7880,
    IROUTER785 = 0x7888,
    IROUTER786 = 0x7890,
    IROUTER787 = 0x7898,
    IROUTER788 = 0x78a0,
    IROUTER789 = 0x78a8,
    IROUTER790 = 0x78b0,
    IROUTER791 = 0x78b8,
    IROUTER792 = 0x78c0,
    IROUTER793 = 0x78c8,
    IROUTER794 = 0x78d0,
    IROUTER795 = 0x78d8,
    IROUTER796 = 0x78e0,
    IROUTER797 = 0x78e8,
    IROUTER798 = 0x78f0,
    IROUTER799 = 0x78f8,
    IROUTER800 = 0x7900,
    IROUTER801 = 0x7908,
    IROUTER802 = 0x7910,
    IROUTER803 = 0x7918,
    IROUTER804 = 0x7920,
    IROUTER805 = 0x7928,
    IROUTER806 = 0x7930,
    IROUTER807 = 0x7938,
    IROUTER808 = 0x7940,
    IROUTER809 = 0x7948,
    IROUTER810 = 0x7950,
    IROUTER811 = 0x7958,
    IROUTER812 = 0x7960,
    IROUTER813 = 0x7968,
    IROUTER814 = 0x7970,
    IROUTER815 = 0x7978,
    IROUTER816 = 0x7980,
    IROUTER817 = 0x7988,
    IROUTER818 = 0x7990,
    IROUTER819 = 0x7998,
    IROUTER820 = 0x79a0,
    IROUTER821 = 0x79a8,
    IROUTER822 = 0x79b0,
    IROUTER823 = 0x79b8,
    IROUTER824 = 0x79c0,
    IROUTER825 = 0x79c8,
    IROUTER826 = 0x79d0,
    IROUTER827 = 0x79d8,
    IROUTER828 = 0x79e0,
    IROUTER829 = 0x79e8,
    IROUTER830 = 0x79f0,
    IROUTER831 = 0x79f8,
    IROUTER832 = 0x7a00,
    IROUTER833 = 0x7a08,
    IROUTER834 = 0x7a10,
    IROUTER835 = 0x7a18,
    IROUTER836 = 0x7a20,
    IROUTER837 = 0x7a28,
    IROUTER838 = 0x7a30,
    IROUTER839 = 0x7a38,
    IROUTER840 = 0x7a40,
    IROUTER841 = 0x7a48,
    IROUTER842 = 0x7a50,
    IROUTER843 = 0x7a58,
    IROUTER844 = 0x7a60,
    IROUTER845 = 0x7a68,
    IROUTER846 = 0x7a70,
    IROUTER847 = 0x7a78,
    IROUTER848 = 0x7a80,
    IROUTER849 = 0x7a88,
    IROUTER850 = 0x7a90,
    IROUTER851 = 0x7a98,
    IROUTER852 = 0x7aa0,
    IROUTER853 = 0x7aa8,
    IROUTER854 = 0x7ab0,
    IROUTER855 = 0x7ab8,
    IROUTER856 = 0x7ac0,
    IROUTER857 = 0x7ac8,
    IROUTER858 = 0x7ad0,
    IROUTER859 = 0x7ad8,
    IROUTER860 = 0x7ae0,
    IROUTER861 = 0x7ae8,
    IROUTER862 = 0x7af0,
    IROUTER863 = 0x7af8,
    IROUTER864 = 0x7b00,
    IROUTER865 = 0x7b08,
    IROUTER866 = 0x7b10,
    IROUTER867 = 0x7b18,
    IROUTER868 = 0x7b20,
    IROUTER869 = 0x7b28,
    IROUTER870 = 0x7b30,
    IROUTER871 = 0x7b38,
    IROUTER872 = 0x7b40,
    IROUTER873 = 0x7b48,
    IROUTER874 = 0x7b50,
    IROUTER875 = 0x7b58,
    IROUTER876 = 0x7b60,
    IROUTER877 = 0x7b68,
    IROUTER878 = 0x7b70,
    IROUTER879 = 0x7b78,
    IROUTER880 = 0x7b80,
    IROUTER881 = 0x7b88,
    IROUTER882 = 0x7b90,
    IROUTER883 = 0x7b98,
    IROUTER884 = 0x7ba0,
    IROUTER885 = 0x7ba8,
    IROUTER886 = 0x7bb0,
    IROUTER887 = 0x7bb8,
    IROUTER888 = 0x7bc0,
    IROUTER889 = 0x7bc8,
    IROUTER890 = 0x7bd0,
    IROUTER891 = 0x7bd8,
    IROUTER892 = 0x7be0,
    IROUTER893 = 0x7be8,
    IROUTER894 = 0x7bf0,
    IROUTER895 = 0x7bf8,
    IROUTER896 = 0x7c00,
    IROUTER897 = 0x7c08,
    IROUTER898 = 0x7c10,
    IROUTER899 = 0x7c18,
    IROUTER900 = 0x7c20,
    IROUTER901 = 0x7c28,
    IROUTER902 = 0x7c30,
    IROUTER903 = 0x7c38,
    IROUTER904 = 0x7c40,
    IROUTER905 = 0x7c48,
    IROUTER906 = 0x7c50,
    IROUTER907 = 0x7c58,
    IROUTER908 = 0x7c60,
    IROUTER909 = 0x7c68,
    IROUTER910 = 0x7c70,
    IROUTER911 = 0x7c78,
    IROUTER912 = 0x7c80,
    IROUTER913 = 0x7c88,
    IROUTER914 = 0x7c90,
    IROUTER915 = 0x7c98,
    IROUTER916 = 0x7ca0,
    IROUTER917 = 0x7ca8,
    IROUTER918 = 0x7cb0,
    IROUTER919 = 0x7cb8,
    IROUTER920 = 0x7cc0,
    IROUTER921 = 0x7cc8,
    IROUTER922 = 0x7cd0,
    IROUTER923 = 0x7cd8,
    IROUTER924 = 0x7ce0,
    IROUTER925 = 0x7ce8,
    IROUTER926 = 0x7cf0,
    IROUTER927 = 0x7cf8,
    IROUTER928 = 0x7d00,
    IROUTER929 = 0x7d08,
    IROUTER930 = 0x7d10,
    IROUTER931 = 0x7d18,
    IROUTER932 = 0x7d20,
    IROUTER933 = 0x7d28,
    IROUTER934 = 0x7d30,
    IROUTER935 = 0x7d38,
    IROUTER936 = 0x7d40,
    IROUTER937 = 0x7d48,
    IROUTER938 = 0x7d50,
    IROUTER939 = 0x7d58,
    IROUTER940 = 0x7d60,
    IROUTER941 = 0x7d68,
    IROUTER942 = 0x7d70,
    IROUTER943 = 0x7d78,
    IROUTER944 = 0x7d80,
    IROUTER945 = 0x7d88,
    IROUTER946 = 0x7d90,
    IROUTER947 = 0x7d98,
    IROUTER948 = 0x7da0,
    IROUTER949 = 0x7da8,
    IROUTER950 = 0x7db0,
    IROUTER951 = 0x7db8,
    IROUTER952 = 0x7dc0,
    IROUTER953 = 0x7dc8,
    IROUTER954 = 0x7dd0,
    IROUTER955 = 0x7dd8,
    IROUTER956 = 0x7de0,
    IROUTER957 = 0x7de8,
    IROUTER958 = 0x7df0,
    IROUTER959 = 0x7df8,
    IROUTER960 = 0x7e00,
    IROUTER961 = 0x7e08,
    IROUTER962 = 0x7e10,
    IROUTER963 = 0x7e18,
    IROUTER964 = 0x7e20,
    IROUTER965 = 0x7e28,
    IROUTER966 = 0x7e30,
    IROUTER967 = 0x7e38,
    IROUTER968 = 0x7e40,
    IROUTER969 = 0x7e48,
    IROUTER970 = 0x7e50,
    IROUTER971 = 0x7e58,
    IROUTER972 = 0x7e60,
    IROUTER973 = 0x7e68,
    IROUTER974 = 0x7e70,
    IROUTER975 = 0x7e78,
    IROUTER976 = 0x7e80,
    IROUTER977 = 0x7e88,
    IROUTER978 = 0x7e90,
    IROUTER979 = 0x7e98,
    IROUTER980 = 0x7ea0,
    IROUTER981 = 0x7ea8,
    IROUTER982 = 0x7eb0,
    IROUTER983 = 0x7eb8,
    IROUTER984 = 0x7ec0,
    IROUTER985 = 0x7ec8,
    IROUTER986 = 0x7ed0,
    IROUTER987 = 0x7ed8,
    IROUTER988 = 0x7ee0,
    IROUTER989 = 0x7ee8,
    IROUTER990 = 0x7ef0,
    IROUTER991 = 0x7ef8,
    IROUTER992 = 0x7f00,
    IROUTER993 = 0x7f08,
    IROUTER994 = 0x7f10,
    IROUTER995 = 0x7f18,
    IROUTER996 = 0x7f20,
    IROUTER997 = 0x7f28,
    IROUTER998 = 0x7f30,
    IROUTER999 = 0x7f38,
    IROUTER1000 = 0x7f40,
    IROUTER1001 = 0x7f48,
    IROUTER1002 = 0x7f50,
    IROUTER1003 = 0x7f58,
    IROUTER1004 = 0x7f60,
    IROUTER1005 = 0x7f68,
    IROUTER1006 = 0x7f70,
    IROUTER1007 = 0x7f78,
    IROUTER1008 = 0x7f80,
    IROUTER1009 = 0x7f88,
    IROUTER1010 = 0x7f90,
    IROUTER1011 = 0x7f98,
    IROUTER1012 = 0x7fa0,
    IROUTER1013 = 0x7fa8,
    IROUTER1014 = 0x7fb0,
    IROUTER1015 = 0x7fb8,
    IROUTER1016 = 0x7fc0,
    IROUTER1017 = 0x7fc8,
    IROUTER1018 = 0x7fd0,
    IROUTER1019 = 0x7fd8,

    PIDR2 = 0xffe8,
}

/// Type of an ARM GIC redistributor register.
#[repr(C)]
#[cfg(feature = "macos-15-0")]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_gic_redistributor_reg_t {
    TYPER = 0x0008,
    PIDR2 = 0xffe8,

    IGROUPR0 = 0x10080,
    ISENABLER0 = 0x10100,
    ICENABLER0 = 0x10180,
    ISPENDR0 = 0x10200,
    ICPENDR0 = 0x10280,
    ISACTIVER0 = 0x10300,
    ICACTIVER0 = 0x10380,

    IPRIORITYR0 = 0x10400,
    IPRIORITYR1 = 0x10404,
    IPRIORITYR2 = 0x10408,
    IPRIORITYR3 = 0x1040c,
    IPRIORITYR4 = 0x10410,
    IPRIORITYR5 = 0x10414,
    IPRIORITYR6 = 0x10418,
    IPRIORITYR7 = 0x1041c,

    ICFGR0 = 0x10c00,
    ICFGR1 = 0x10c04,
}

/// Type of an ARM GIC ICC system control register.
#[repr(C)]
#[cfg(feature = "macos-15-0")]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_gic_icc_reg_t {
    PMR_EL1 = 0xc230,
    BPR0_EL1 = 0xc643,
    AP0R0_EL1 = 0xc644,
    AP1R0_EL1 = 0xc648,
    RPR_EL1 = 0xc65b,
    BPR1_EL1 = 0xc663,
    CTLR_EL1 = 0xc664,
    SRE_EL1 = 0xc665,
    IGRPEN0_EL1 = 0xc666,
    IGRPEN1_EL1 = 0xc667,
    SRE_EL2 = 0xe64d,
}

/// Type of an ARM GIC virtualization control system register.
#[repr(C)]
#[cfg(feature = "macos-15-0")]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_gic_ich_reg_t {
    AP0R0_EL2 = 0xe640,
    AP1R0_EL2 = 0xe648,
    HCR_EL2 = 0xe658,
    VTR_EL2 = 0xe659,
    MISR_EL2 = 0xe65a,
    EISR_EL2 = 0xe65b,
    ELRSR_EL2 = 0xe65d,
    VMCR_EL2 = 0xe65f,
    LR0_EL2 = 0xe660,
    LR1_EL2 = 0xe661,
    LR2_EL2 = 0xe662,
    LR3_EL2 = 0xe663,
    LR4_EL2 = 0xe664,
    LR5_EL2 = 0xe665,
    LR6_EL2 = 0xe666,
    LR7_EL2 = 0xe667,
    LR8_EL2 = 0xe668,
    LR9_EL2 = 0xe669,
    LR10_EL2 = 0xe66a,
    LR11_EL2 = 0xe66b,
    LR12_EL2 = 0xe66c,
    LR13_EL2 = 0xe66d,
    LR14_EL2 = 0xe66e,
    LR15_EL2 = 0xe66f,
}

/// Type of an ARM GIC ICV system control register.
#[repr(C)]
#[cfg(feature = "macos-15-0")]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_gic_icv_reg_t {
    PMR_EL1 = 0xc230,
    BPR0_EL1 = 0xc643,
    AP0R0_EL1 = 0xc644,
    AP1R0_EL1 = 0xc648,
    RPR_EL1 = 0xc65b,
    BPR1_EL1 = 0xc663,
    CTLR_EL1 = 0xc664,
    SRE_EL1 = 0xc665,
    IGRPEN0_EL1 = 0xc666,
    IGRPEN1_EL1 = 0xc667,
}

/// Type of an ARM GIC Distributor message based interrupt register.
#[repr(C)]
#[cfg(feature = "macos-15-0")]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum hv_gic_msi_reg_t {
    TYPER = 0x0008,
    SET_SPI_NSR = 0x0040,
}

// GIC configuration functions.
unsafe extern "C" {
    /// Create a GIC configuration object.
    ///
    /// # Discussion
    ///
    /// Create the GIC configuration after the virtual machine has been created.
    ///
    /// # Return Value
    ///
    /// A new GIC configuration object. Release with [`os_release`] when no longer needed.
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_config_create() -> hv_gic_config_t;

    /// Set the GIC distributor region base address.
    ///
    /// # Parameters
    ///
    /// * `config`: GIC configuration object.
    /// * `distributor_base_address`: Guest physical address for distributor.
    ///
    /// # Discussion
    ///
    /// Guest physical address for distributor base aligned to byte value
    /// returned by hv_gic_get_distributor_base_alignment.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_config_set_distributor_base(
        config: hv_gic_config_t,
        distributor_base_address: hv_ipa_t,
    ) -> hv_return_t;

    /// Set the GIC redistributor region base address.
    ///
    /// # Parameters
    ///
    /// * `config`: GIC configuration object.
    /// * `redistributor_base_address`: Guest physical address for redistributor.
    ///
    /// # Discussion
    ///
    /// Guest physical address for redistributor base aligned to byte value
    /// returned by hv_gic_get_redistributor_base_alignment. The redistributor
    /// region will contain redistributors for all vCPUs supported by the
    /// virtual machine.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_config_set_redistributor_base(
        config: hv_gic_config_t,
        redistributor_base_address: hv_ipa_t,
    ) -> hv_return_t;

    /// Set the GIC MSI region base address.
    ///
    /// # Parameters
    ///
    /// * `config`: GIC configuration object.
    /// * `msi_region_base_address`: Guest physical address for MSI region.
    ///
    /// # Discussion
    ///
    /// Guest physical address for MSI region base aligned to byte value
    /// returned by [`hv_gic_get_msi_region_base_alignment`].
    ///
    /// For MSI support, you also need to set the interrupt range with
    /// [`hv_gic_config_set_msi_interrupt_range`].
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_config_set_msi_region_base(
        config: hv_gic_config_t,
        msi_region_base_address: hv_ipa_t,
    ) -> hv_return_t;

    /// Sets the range of MSIs supported.
    ///
    /// # Parameters
    ///
    /// * `config`: GIC configuration object.
    /// * `msi_intid_base`: Lowest MSI interrupt number.
    /// * `msi_intid_count`: Number of MSIs.
    ///
    /// # Discussion
    ///
    /// Configures the range of identifiers supported for MSIs. If it is outside of
    /// the range given by [`hv_gic_get_spi_interrupt_range`] an error will be
    /// returned.
    ///
    /// For MSI support, you also need to set the region base address with
    /// [`hv_gic_config_set_msi_region_base`].
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_config_set_msi_interrupt_range(
        config: hv_gic_config_t,
        msi_intid_base: u32,
        msi_intid_count: u32,
    ) -> hv_return_t;
}

// GIC core functions.
unsafe extern "C" {
    /// Create a GIC v3 device for a VM configuration.
    ///
    /// # Parameters
    ///
    /// * `gic_config`: GIC configuration object.
    ///
    /// # Discussion
    ///
    /// This function can be used to create an ARM Generic Interrupt Controller
    /// (GIC) v3 device. There must only be a single instance of this device per
    /// virtual machine. The device supports a distributor, redistributors, msi and
    /// GIC CPU system registers. When EL2 is enabled, the device supports GIC
    /// hypervisor control registers which are used by the guest hypervisor for
    /// injecting interrupts to its guest. `hv_vcpu_{get/set}_interrupt` functions
    /// are unsupported for injecting interrupts to a nested guest.
    ///
    /// The [`hv_gic_create`] API must only be called after a virtual machine has
    /// been created. It must also be done before vCPU's have been created so that
    /// GIC CPU system resources can be allocated. If either of these conditions
    /// aren't met an error is returned.
    ///
    /// GIC v3 uses affinity based interrupt routing. vCPU's must set affinity
    /// values in their `MPIDR_EL1` register. Once the virtual machine vcpus are
    /// running, its topology is considered final. Destroy vcpus only when you are
    /// tearing down the virtual machine.
    ///
    /// GIC MSI support is only provided if both an MSI region base address is
    /// configured and an MSI interrupt range is set.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_create(gic_config: hv_gic_config_t) -> hv_return_t;

    /// Trigger a Shared Peripheral Interrupt (SPI).
    ///
    /// # Parameters
    ///
    /// * `intid`: Interrupt number of the SPI.
    /// * `level`: High or low level for an interrupt. Setting level also causes an edge on the
    /// line for an edge triggered interrupt.
    ///
    /// # Discussion
    ///
    /// Level interrupts can be caused by setting a level value. If you want to
    /// cause an edge interrupt, call with a level of true. A level of false, for
    /// an edge interrupt will be ignored.
    ///
    /// An interrupt identifier outside of [`hv_gic_get_spi_interrupt_range`] or in
    /// the MSI interrupt range will return a [`hv_error_t::HV_BAD_ARGUMENT`] error code.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_set_spi(intid: u32, level: bool) -> hv_return_t;

    /// Send a Message Signaled Interrupt (MSI).
    ///
    /// # Parameters
    ///
    /// * `address`: Guest physical address for message based SPI.
    /// * `intid`: Interrupt identifier for the message based SPI.
    ///
    /// # Discussion
    ///
    /// Use the address of the HV_GIC_REG_GICM_SET_SPI_NSR register in the MSI frame.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_send_msi(address: hv_ipa_t, intid: u32) -> hv_return_t;

    /// Read a GIC distributor register.
    ///
    /// # Parameters
    ///
    /// * `reg`: GIC distributor register enum.
    /// * `value`: Pointer to distributor register value (written on success).
    ///
    /// # Discussion
    ///
    /// GIC distributor register enum values are equal to the device register
    /// offsets defined in the ARM GIC v3 specification. The client can use the
    /// offset alternatively, while looping through large register arrays.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_distributor_reg(
        reg: hv_gic_distributor_reg_t,
        value: *mut u64,
    ) -> hv_return_t;

    /// Write a GIC distributor register.
    ///
    /// # Parameters
    ///
    /// * `reg`: GIC distributor register enum.
    /// * `value`: GIC distributor register value to be written.
    ///
    /// # Discussion
    ///
    /// GIC distributor register enum values are equal to the device register
    /// offsets defined in the ARM GIC v3 specification. The client can use the
    /// offset alternatively, while looping through large register arrays.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_set_distributor_reg(reg: hv_gic_distributor_reg_t, value: u64) -> hv_return_t;

    /// Gets the redistributor base guest physical address for the given vcpu.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Handle for the vcpu.
    /// * `redistributor_base_address`: Pointer to the redistributor base guest physical address
    /// (written on success).
    ///
    /// # Discussion
    ///
    /// Must be called after the affinity of the given vCPU has been set in its MPIDR_EL1 register.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_redistributor_base(
        vcpu: hv_vcpu_t,
        redistributor_base_address: *mut hv_ipa_t,
    ) -> hv_return_t;

    /// Read a GIC redistributor register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Redistributor block for the vcpu.
    /// * `reg`: GIC redistributor register enum.
    /// * `value`: Pointer to redistributor register value (written on success).
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// GIC redistributor register enum values are equal to the device register
    /// offsets defined in the ARM GIC v3 specification. The client can use the
    /// offset alternatively, while looping through large register arrays.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_redistributor_reg(
        vcpu: hv_vcpu_t,
        reg: hv_gic_redistributor_reg_t,
        value: *mut u64,
    ) -> hv_return_t;

    /// Write a GIC redistributor register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Redistributor block for the vcpu.
    /// * `reg`: GIC redistributor register enum.
    /// * `value`: GIC redistributor register value to be written.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// GIC redistributor register enum values are equal to the device register
    /// offsets defined in the ARM GIC v3 specification. The client can use the
    /// offset alternatively, while looping through large register arrays.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_set_redistributor_reg(
        vcpu: hv_vcpu_t,
        reg: hv_gic_redistributor_reg_t,
        value: u64,
    ) -> hv_return_t;

    /// Read a GIC ICC cpu system register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Handle for the vcpu.
    /// * `reg`: GIC ICC system register enum.
    /// * `value`: Pointer to ICC register value (written on success).
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_icc_reg(
        vcpu: hv_vcpu_t,
        reg: hv_gic_icc_reg_t,
        value: *mut u64,
    ) -> hv_return_t;

    /// Write a GIC ICC cpu system register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Handle for the vcpu.
    /// * `reg`: GIC ICC system register enum.
    /// * `value`: GIC ICC register value to be written.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_set_icc_reg(vcpu: hv_vcpu_t, reg: hv_gic_icc_reg_t, value: u64) -> hv_return_t;

    /// Read a GIC ICH virtualization control system register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Handle for the vcpu.
    /// * `reg`: GIC ICH system register enum.
    /// * `value`: Pointer to ICH register value (written on success).
    ///
    /// # Discussion
    ///
    /// ICH registers are only available when EL2 is enabled, otherwise returns
    /// an error.
    ///
    /// Must be called by the owning thread.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_ich_reg(
        vcpu: hv_vcpu_t,
        reg: hv_gic_ich_reg_t,
        value: *mut u64,
    ) -> hv_return_t;

    /// Write a GIC ICH virtualization control system register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Handle for the vcpu.
    /// * `reg`: GIC ICH system register enum.
    /// * `value`: GIC ICH register value to be written.
    ///
    /// # Discussion
    ///
    /// ICH registers are only available when EL2 is enabled, otherwise returns
    /// an error.
    ///
    /// Must be called by the owning thread.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_set_ich_reg(vcpu: hv_vcpu_t, reg: hv_gic_ich_reg_t, value: u64) -> hv_return_t;

    /// Read a GIC ICV system register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Handle for the vcpu.
    /// * `reg`: GIC ICV system register enum.
    /// * `value`: Pointer to ICV register value (written on success).
    ///
    /// # Discussion
    ///
    /// ICV registers are only available when EL2 is enabled, otherwise returns
    /// an error.
    ///
    /// Must be called by the owning thread.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_icv_reg(
        vcpu: hv_vcpu_t,
        reg: hv_gic_icv_reg_t,
        value: *mut u64,
    ) -> hv_return_t;

    /// Write a GIC ICV system register.
    ///
    /// # Parameters
    ///
    /// * `vcpu`: Handle for the vcpu.
    /// * `reg`: GIC ICV system register enum.
    /// * `value`: GIC ICV register value to be written.
    ///
    /// # Discussion
    ///
    /// ICV registers are only available when EL2 is enabled, otherwise returns
    /// an error.
    ///
    /// Must be called by the owning thread.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_set_icv_reg(vcpu: hv_vcpu_t, reg: hv_gic_icv_reg_t, value: u64) -> hv_return_t;

    /// Read a GIC distributor MSI register.
    ///
    /// # Parameters
    ///
    /// * `reg`: GIC distributor MSI register enum.
    /// * `value`: Pointer to distributor MSI register value (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_msi_reg(reg: hv_gic_msi_reg_t, value: *mut u64) -> hv_return_t;

    /// Write a GIC distributor MSI register.
    ///
    /// # Parameters
    ///
    /// * `reg`: GIC distributor MSI register enum.
    /// * `value`: GIC distributor MSI register value to be written.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_set_msi_reg(reg: hv_gic_msi_reg_t, value: u64) -> hv_return_t;

    /// Set state for GIC device to be restored.
    ///
    /// # Parameters
    ///
    /// * `gic_state_data`: Pointer to the state buffer to set GIC with.
    /// * `gic_state_size`: Size of GIC state buffer.
    ///
    /// # Discussion
    ///
    /// GIC state can only be restored after a GIC device and vcpus have been
    /// created and must be done before vcpu's are run. The rest of the virtual
    /// machine including GIC CPU registers must also be restored compatibly with
    /// the gic_state.
    ///
    /// In some cases [`hv_gic_set_state`] can fail if a software update has changed
    /// the host in a way that would be incompatible with the previous format.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_set_state(gic_state_data: *const c_void, gic_state_size: usize) -> hv_return_t;

    /// Reset the GIC device.
    ///
    /// # Parameters
    ///
    /// * `gic_config`: GIC configuration object.
    ///
    /// # Discussion
    ///
    /// When the virtual machine is being reset, call this function to reset the
    /// GIC distributor, redistributor registers and the internal state of the
    /// device.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_reset() -> hv_return_t;
}

// GIC parameters functions.
unsafe extern "C" {
    /// Gets the size in bytes of the GIC distributor region.
    ///
    /// # Parameters
    ///
    /// * `distributor_size`: Pointer to GIC distributor region size (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_distributor_size(distributor_size: *mut usize) -> hv_return_t;

    /// Gets the alignment in bytes for the base address of the GIC distributor region.
    ///
    /// # Parameters
    ///
    /// * `distributor_base_alignment`: Pointer to GIC distributor base address alignment
    /// (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_distributor_base_alignment(
        distributor_base_alignment: *mut usize,
    ) -> hv_return_t;

    /// Gets the total size in bytes of the GIC redistributor region.
    ///
    /// # Parameters
    ///
    /// * `redistributor_region_size`: Pointer to GIC redistributor region size
    /// (written on success).
    ///
    /// # Discussion
    ///
    /// Provides the total size of the GIC redistributor regions supported. Each
    /// redistributor is two 64 kilobyte frames per vCPU and is contiguously
    /// placed.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_redistributor_region_size(
        redistributor_region_size: *mut usize,
    ) -> hv_return_t;

    /// Gets the size in bytes of a single GIC redistributor.
    ///
    /// # Parameters
    ///
    /// * `redistributor_size`: Pointer to GIC redistributor region size
    /// (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_redistributor_size(redistributor_size: *mut usize) -> hv_return_t;

    /// Gets the alignment in bytes for the base address of the GIC redistributor region.
    ///
    /// # Parameters
    ///
    /// * `redistributor_base_alignment`: Pointer to GIC redistributor base address alignment
    /// (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_redistributor_base_alignment(
        redistributor_base_alignment: *mut usize,
    ) -> hv_return_t;

    /// Gets the size in bytes of the GIC MSI region.
    ///
    /// # Parameters
    ///
    /// * `msi_region_size`: Pointer to GIC MSI region size (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_msi_region_size(msi_region_size: *mut usize) -> hv_return_t;

    /// Gets the alignment in bytes for the base address of the GIC MSI region.
    ///
    /// # Parameters
    ///
    /// * `msi_region_base_alignment`: Pointer to GIC MSI region base address alignment
    /// (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_msi_region_base_alignment(
        msi_region_base_alignment: *mut usize,
    ) -> hv_return_t;

    /// Gets the range of SPIs supported.
    ///
    /// # Parameters
    ///
    /// * `spi_intid_base`: Pointer to the lowest SPI number (written on success).
    /// * `spi_intid_count`: Pointer to the number of SPIs supported (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_spi_interrupt_range(
        spi_intid_base: *mut u32,
        spi_intid_count: *mut u32,
    ) -> hv_return_t;

    /// Gets the interrupt id for reserved interrupts.
    ///
    /// # Parameters
    ///
    /// * `interrupt`: Enum value for reserved interrupts.
    /// * `intid`: Pointer to the interrupt number (written on success).
    ///
    /// # Discussion
    ///
    /// Provides the interrupt id for interrupts that are reserved by the
    /// hypervisor framework.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_get_intid(interrupt: hv_gic_intid_t, intid: *mut u32) -> hv_return_t;
}

// GIC state functions.
unsafe extern "C" {
    /// Create a GIC state object.
    ///
    /// # Discussion
    ///
    /// The function returns no object if the current GIC state can not be represented in a GIC
    /// state object, or if there is no GIC present in the virtual machine.
    ///
    /// The virtual machine must be in a stopped state prior to calling this function.
    ///
    /// # Return Value
    ///
    /// A new GIC state object that is representative of the current GIC state.
    /// This should be released with [`os_release`] when no longer used.
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_state_create() -> hv_gic_state_t;

    /// Get size of buffer required for GIC state.
    ///
    /// # Parameters
    ///
    /// * `state`: GIC configuration object.
    /// * `gic_state_size`: Pointer to GIC data size (written on success).
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_state_get_size(state: hv_gic_state_t, gic_state_size: *mut usize) -> hv_return_t;

    /// Get the state data for GIC.
    ///
    /// # Parameters
    ///
    /// * `state`: GIC configuration object.
    /// * `gic_state_data`: Pointer to GIC state buffer (written on success).
    ///
    /// # Discussion
    ///
    /// The function returns an opaque data buffer that contains the complete
    /// serialized state of the device, except for the GIC cpu registers. The data
    /// can be written to a file and is stable. It is also versioned for detecting
    /// incompatibilities on restore of the state. The size of this GIC state buffer
    /// must be at least as large as the size returned by [`hv_gic_state_get_size`].
    ///
    /// GIC CPU system registers can be read separately, and saved to restore the
    /// cpu state for the virtual machine.
    ///
    /// # Return Value
    ///
    /// `HV_SUCCESS` if the operation was successful, otherwise an error code specified in
    /// [`hv_return_t`].
    #[cfg(feature = "macos-15-0")]
    pub fn hv_gic_state_get_data(state: hv_gic_state_t, gic_state_data: *mut c_void)
        -> hv_return_t;
}
