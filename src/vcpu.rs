//! Management and interaction with virtual CPUs.

#[cfg(feature = "simd-nightly")]
use std::simd;

use crate::hv_unsafe_call;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

use applevisor_sys::*;

use crate::error::*;
#[cfg(feature = "macos-15-0")]
use crate::gic::*;
#[cfg(feature = "macos-15-2")]
use crate::vm::*;

// -----------------------------------------------------------------------------------------------
// vCPU Management - Configuration
// -----------------------------------------------------------------------------------------------

/// The type that defines feature registers.
pub type FeatureReg = hv_feature_reg_t;

/// The structure that describes an instruction or data cache element.
pub type CacheType = hv_cache_type_t;

/// Represents a vCPU configuration.
#[derive(Debug)]
pub struct VcpuConfig(pub(crate) hv_vcpu_config_t);

impl Default for VcpuConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for VcpuConfig {
    fn drop(&mut self) {
        unsafe { os_release(self.0) }
    }
}

impl VcpuConfig {
    /// Instanciates a new configuration.
    pub fn new() -> Self {
        let config = unsafe { hv_vcpu_config_create() };
        VcpuConfig(config)
    }

    /// Retrieves the value of a feature register.
    pub fn get_feature_reg(&self, reg: FeatureReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_config_get_feature_reg(self.0, reg, &mut value))?;
        Ok(value)
    }

    /// Returns the Cache Size ID Register (CCSIDR_EL1) values for the vCPU configuration and
    /// cache type you specify.
    pub fn get_ccsidr_el1_sys_reg_values(&self, cache_type: CacheType) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_config_get_ccsidr_el1_sys_reg_values(
            self.0, cache_type, &mut value
        ))?;
        Ok(value)
    }
}

// -----------------------------------------------------------------------------------------------
// vCPU
// -----------------------------------------------------------------------------------------------

/// The type that describes the event that triggered a guest exit to the host.
pub type ExitReason = hv_exit_reason_t;

/// Represents vCPU exit info.
pub type VcpuExit = hv_vcpu_exit_t;

/// The type that defines the vCPU’s interrupts.
pub type InterruptType = hv_interrupt_type_t;

/// The structure that describes information about an exit from the virtual CPU (vCPU) to the host.
pub type VcpuExitException = hv_vcpu_exit_exception_t;

/// The type that defines general registers.
pub type Reg = hv_reg_t;

/// The type that defines SIMD and floating-point registers.
pub type SimdFpReg = hv_simd_fp_reg_t;

/// The type of system registers.
pub type SysReg = hv_sys_reg_t;

/// Contains information about SME PSTATE.
#[cfg(feature = "macos-15-2")]
pub type SmeState = hv_vcpu_sme_state_t;

/// Type of an ARM SME Z vector register.
#[cfg(feature = "macos-15-2")]
pub type SmeZReg = hv_sme_z_reg_t;

/// Type of an ARM SME P predicate register.
#[cfg(feature = "macos-15-2")]
pub type SmePReg = hv_sme_p_reg_t;

/// Type of the SME2 ZT0 register.
#[cfg(feature = "macos-15-2")]
pub type SmeZt0 = hv_sme_zt0_uchar64_t;

/// Represents a handle to a Virtual CPU.
///
/// This object can be safely shared among threads, but will become invalid when the vCPU it
/// corresponds to is destroyed.
#[derive(Clone, Debug)]
pub struct VcpuHandle {
    vcpu: hv_vcpu_t,
    _guard: Weak<()>,
}

impl VcpuHandle {
    /// Returns the ID of the vCPU associated to this handle.
    pub fn id(&self) -> u64 {
        self.vcpu
    }

    /// Returns `true` if the vCPU associated to this handle is still alive.
    pub fn is_valid(&self) -> bool {
        Weak::strong_count(&self._guard) > 0
    }

    /// Takes a strong reference to the vCPU. This prevents its resources to be deallocated
    /// if operations are perfomed on the vCPU.
    ///
    /// This method is currently private to prevent other threads from arbitrarily stopping
    /// the destruction of the vCPU.
    pub(crate) fn take_ref(&self) -> Option<Arc<()>> {
        Weak::upgrade(&self._guard)
    }
}

/// Represents a Virtual CPU.
#[derive(Debug)]
pub struct Vcpu {
    /// Opaque ID of the vCPU instance.
    pub(crate) vcpu: hv_vcpu_t,
    /// vCPU exit information.
    pub(crate) exit: *const hv_vcpu_exit_t,
    /// Strong reference to the virtual machine this vCPU belongs to.
    pub(crate) _guard_vm: Arc<()>,
    /// References shared between the vCPU and its handles.
    pub(crate) _guard_self: Arc<()>,
    /// Vcpu are bound to a specific thread and can't be sent to another.
    /// This marker is superfluous here, but it makes things explicit.
    pub(crate) _phantom: PhantomData<*const ()>,
}

impl Drop for Vcpu {
    fn drop(&mut self) {
        hv_unsafe_call!(hv_vcpu_destroy(self.vcpu))
            .expect("Could not properly destroy vCPU instance");
    }
}

impl Vcpu {
    /// Returns the vCPU handle id.
    pub fn id(&self) -> u64 {
        self.vcpu
    }

    /// Returns an handle to the vCPU allowing other threads to perform operations on it.
    ///
    /// # Discussion
    ///
    /// This is currently only used by [`VirtualMachineInstance::vcpus_exit`].
    pub fn get_handle(&self) -> VcpuHandle {
        VcpuHandle {
            vcpu: self.vcpu,
            _guard: Arc::downgrade(&self._guard_self),
        }
    }

    /// Returns the maximum number of vCPUs that can be created by the hypervisor.
    pub fn get_max_count() -> Result<u32> {
        let mut count = 0;
        hv_unsafe_call!(hv_vm_get_max_vcpu_count(&mut count))?;
        Ok(count)
    }

    /// Starts the vCPU.
    pub fn run(&self) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_run(self.vcpu))
    }

    /// Gets vCPU exit info.
    pub fn get_exit_info(&self) -> VcpuExit {
        VcpuExit::from(unsafe { *self.exit })
    }

    /// Gets pending interrupts for a vCPU.
    pub fn get_pending_interrupt(&self, intr: InterruptType) -> Result<bool> {
        let mut pending = false;
        hv_unsafe_call!(hv_vcpu_get_pending_interrupt(self.vcpu, intr, &mut pending))?;
        Ok(pending)
    }

    /// Sets pending interrupts for a vCPU.
    pub fn set_pending_interrupt(&self, intr: InterruptType, pending: bool) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_pending_interrupt(self.vcpu, intr, pending))
    }

    /// Gets the value of a vCPU general purpose register.
    pub fn get_reg(&self, reg: Reg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_get_reg(self.vcpu, reg, &mut value))?;
        Ok(value)
    }

    /// Sets the value of a vCPU general purpose register.
    pub fn set_reg(&self, reg: Reg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_reg(self.vcpu, reg, value))
    }

    /// Gets the value of a vCPU system register.
    pub fn get_sys_reg(&self, reg: SysReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_get_sys_reg(self.vcpu, reg, &mut value))?;
        Ok(value)
    }

    /// Sets the value of a vCPU general purpose register.
    pub fn set_sys_reg(&self, reg: SysReg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_sys_reg(self.vcpu, reg, value))
    }

    /// Gets the value of a vCPU floating point register
    #[cfg(feature = "simd-nightly")]
    pub fn get_simd_fp_reg(&self, reg: SimdFpReg) -> Result<simd::u8x16> {
        let mut value = simd::u8x16::from_array([0; 16]);
        hv_unsafe_call!(hv_vcpu_get_simd_fp_reg(
            self.vcpu,
            Into::<hv_simd_fp_reg_t>::into(reg),
            &mut value
        ))?;
        Ok(value)
    }

    #[cfg(feature = "simd-nightly")]
    /// Sets the value of a vCPU floating point register
    pub fn set_simd_fp_reg(&self, reg: SimdFpReg, value: simd::u8x16) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_simd_fp_reg(
            self.vcpu,
            Into::<hv_simd_fp_reg_t>::into(reg),
            value
        ))
    }

    /// Gets the value of a vCPU floating point register
    #[cfg(not(feature = "simd-nightly"))]
    pub fn get_simd_fp_reg(&self, reg: SimdFpReg) -> Result<u128> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_get_simd_fp_reg(self.vcpu, reg, &mut value))?;
        Ok(value)
    }

    /// Sets the value of a vCPU floating point register
    #[cfg(not(feature = "simd-nightly"))]
    pub fn set_simd_fp_reg(&self, reg: SimdFpReg, value: u128) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_simd_fp_reg(self.vcpu, reg, value))
    }

    /// Gets the current SME state consisting of the streaming SVE mode (`PSTATE.SM`) and ZA
    /// storage enable (`PSTATE.ZA`).
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// In streaming SVE mode, the SIMD Q registers are aliased to the bottom 128 bits of the
    /// corresponding Z register, and any modification will reflect on the Z register state.
    #[cfg(feature = "macos-15-2")]
    pub fn get_sme_state(&self) -> Result<SmeState> {
        let mut state = SmeState::default();
        hv_unsafe_call!(hv_vcpu_get_sme_state(self.vcpu, &mut state))?;
        Ok(state)
    }

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
    #[cfg(feature = "macos-15-2")]
    pub fn set_sme_state(&self, state: &SmeState) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_sme_state(self.vcpu, state))
    }

    /// Returns the value of a vCPU Z vector register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Returns an error if not in streaming SVE mode (i.e. `streaming_sve_mode_enabled` is false),
    /// or if the provided value storage is not maximum SVL bytes.
    #[cfg(feature = "macos-15-2")]
    pub fn get_sme_z_reg(&self, reg: SmeZReg, value: &mut [u8]) -> Result<()> {
        let size: usize = VirtualMachineConfig::get_max_svl_bytes()?;
        if value.len() != size {
            return Err(HypervisorError::BadArgument);
        }
        hv_unsafe_call!(hv_vcpu_get_sme_z_reg(
            self.vcpu,
            reg,
            value.as_mut_ptr(),
            size
        ))
    }

    /// Sets the value of a vCPU Z vector register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    #[cfg(feature = "macos-15-2")]
    pub fn set_sme_z_reg(&self, reg: SmeZReg, value: &[u8]) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_sme_z_reg(
            self.vcpu,
            reg,
            value.as_ptr(),
            value.len()
        ))
    }

    /// Returns the value of a vCPU P predicate register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Returns an error if not in streaming SVE mode (i.e. `streaming_sve_mode_enabled` is false),
    /// or if the provided value storage is not maximum SVL bytes.
    #[cfg(feature = "macos-15-2")]
    pub fn get_sme_p_reg(&self, reg: SmePReg, value: &mut [u8]) -> Result<()> {
        let size: usize = VirtualMachineConfig::get_max_svl_bytes()? / 8;
        if value.len() != size {
            return Err(HypervisorError::BadArgument);
        }
        hv_unsafe_call!(hv_vcpu_get_sme_p_reg(
            self.vcpu,
            reg,
            value.as_mut_ptr(),
            size
        ))
    }

    /// Sets the value of a vCPU P predicate register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Returns an error if not in streaming SVE mode (i.e. `streaming_sve_mode_enabled` is false),
    /// or if the provided value storage is not maximum SVL bytes.
    #[cfg(feature = "macos-15-2")]
    pub fn set_sme_p_reg(&self, reg: SmePReg, value: &[u8]) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_sme_p_reg(
            self.vcpu,
            reg,
            value.as_ptr(),
            value.len()
        ))
    }

    /// Returns the value of the vCPU ZA matrix register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Does not require streaming SVE mode enabled.
    ///
    /// Returns an error if `PSTATE.ZA` is 0 (i.e. `za_storage_enabled` is false), or if the
    /// provided value storage is not [maximum SVL bytes x maximum SVL bytes].
    #[cfg(feature = "macos-15-2")]
    pub fn get_sme_za_reg(&self, value: &mut [u8]) -> Result<()> {
        let size: usize = VirtualMachineConfig::get_max_svl_bytes()?;
        let size = size * size;
        if value.len() != size {
            return Err(HypervisorError::BadArgument);
        }
        hv_unsafe_call!(hv_vcpu_get_sme_za_reg(self.vcpu, value.as_mut_ptr(), size))
    }

    /// Sets the value of the vCPU ZA matrix register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Returns an error if `PSTATE.ZA` is 0 (i.e. `za_storage_enabled` is false), or if the
    /// value length is not [maximum SVL bytes x maximum SVL bytes].
    #[cfg(feature = "macos-15-2")]
    pub fn set_sme_za_reg(&self, value: &[u8]) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_sme_za_reg(
            self.vcpu,
            value.as_ptr(),
            value.len()
        ))
    }

    /// Returns the current value of the vCPU ZT0 register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Does not require streaming SVE mode enabled.
    ///
    /// Returns an error if `PSTATE.ZA` is 0 (i.e. `za_storage_enabled` is false).
    #[cfg(feature = "macos-15-2")]
    pub fn get_sme_zt0_reg(&self, value: &mut SmeZt0) -> Result<()> {
        #[cfg(not(feature = "simd-nightly"))]
        {
            hv_unsafe_call!(hv_vcpu_get_sme_zt0_reg(
                self.vcpu,
                value.as_mut_ptr() as *mut u8
            ))
        }
        #[cfg(feature = "simd-nightly")]
        {
            hv_unsafe_call!(hv_vcpu_get_sme_zt0_reg(
                self.vcpu,
                value.as_mut_array() as *mut u8
            ))
        }
    }

    /// Sets the value of the vCPU ZT0 register in streaming SVE mode.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// Returns an error if `PSTATE.ZA` is 0 (i.e. `za_storage_enabled` is false).
    #[cfg(feature = "macos-15-2")]
    pub fn set_sme_zt0_reg(&self, value: &SmeZt0) -> Result<()> {
        #[cfg(not(feature = "simd-nightly"))]
        {
            hv_unsafe_call!(hv_vcpu_set_sme_zt0_reg(
                self.vcpu,
                value.as_ptr() as *const u8
            ))
        }
        #[cfg(feature = "simd-nightly")]
        {
            hv_unsafe_call!(hv_vcpu_set_sme_zt0_reg(
                self.vcpu,
                value.as_array() as *const u8
            ))
        }
    }

    /// Gets the redistributor base guest physical address for the given vcpu.
    ///
    /// # Discussion
    ///
    /// Must be called after the affinity of the given vCPU has been set in its MPIDR_EL1 register.
    #[cfg(feature = "macos-15-0")]
    pub fn get_redistributor_base(&self) -> Result<u64> {
        let mut redistributor_base_address = 0;
        hv_unsafe_call!(hv_gic_get_redistributor_base(
            self.vcpu,
            &mut redistributor_base_address,
        ))?;
        Ok(redistributor_base_address)
    }

    /// Read a GIC redistributor register.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// GIC redistributor register enum values are equal to the device register
    /// offsets defined in the ARM GIC v3 specification. The client can use the
    /// offset alternatively, while looping through large register arrays.
    #[cfg(feature = "macos-15-0")]
    pub fn get_redistributor_reg(&self, reg: GicRedistributorReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_gic_get_redistributor_reg(self.vcpu, reg, &mut value))?;
        Ok(value)
    }

    /// Write a GIC redistributor register.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    ///
    /// GIC redistributor register enum values are equal to the device register
    /// offsets defined in the ARM GIC v3 specification. The client can use the
    /// offset alternatively, while looping through large register arrays.
    #[cfg(feature = "macos-15-0")]
    pub fn set_redistributor_reg(&self, reg: GicRedistributorReg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_set_redistributor_reg(self.vcpu, reg, value))?;
        Ok(())
    }

    /// Read a GIC ICC cpu system register.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    #[cfg(feature = "macos-15-0")]
    pub fn get_icc_reg(&self, reg: GicIccReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_gic_get_icc_reg(self.vcpu, reg, &mut value))?;
        Ok(value)
    }

    /// Write a GIC ICC cpu system register.
    ///
    /// # Discussion
    ///
    /// Must be called by the owning thread.
    #[cfg(feature = "macos-15-0")]
    pub fn set_icc_reg(&self, reg: GicIccReg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_set_icc_reg(self.vcpu, reg, value))?;
        Ok(())
    }

    /// Read a GIC ICH virtualization control system register.
    ///
    /// # Discussion
    ///
    /// ICH registers are only available when EL2 is enabled, otherwise returns
    /// an error.
    ///
    /// Must be called by the owning thread.
    #[cfg(feature = "macos-15-0")]
    pub fn get_ich_reg(&self, reg: GicIchReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_gic_get_ich_reg(self.vcpu, reg, &mut value))?;
        Ok(value)
    }

    /// Write a GIC ICH virtualization control system register.
    ///
    /// # Discussion
    ///
    /// ICH registers are only available when EL2 is enabled, otherwise returns
    /// an error.
    ///
    /// Must be called by the owning thread.
    #[cfg(feature = "macos-15-0")]
    pub fn set_ich_reg(&self, reg: GicIchReg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_set_ich_reg(self.vcpu, reg, value))?;
        Ok(())
    }

    /// Read a GIC ICV system register.
    ///
    /// # Discussion
    ///
    /// ICV registers are only available when EL2 is enabled, otherwise returns
    /// an error.
    ///
    /// Must be called by the owning thread.
    #[cfg(feature = "macos-15-0")]
    pub fn get_icv_reg(&self, reg: GicIcvReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_gic_get_icv_reg(self.vcpu, reg, &mut value))?;
        Ok(value)
    }

    /// Write a GIC ICV system register.
    ///
    /// # Discussion
    ///
    /// ICV registers are only available when EL2 is enabled, otherwise returns
    /// an error.
    ///
    /// Must be called by the owning thread.
    #[cfg(feature = "macos-15-0")]
    pub fn set_icv_reg(&self, reg: GicIcvReg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_set_icv_reg(self.vcpu, reg, value))?;
        Ok(())
    }

    /// Gets whether debug exceptions exit the guest.
    pub fn get_trap_debug_exceptions(&self) -> Result<bool> {
        let mut value = false;
        hv_unsafe_call!(hv_vcpu_get_trap_debug_exceptions(self.vcpu, &mut value))?;
        Ok(value)
    }

    /// Sets whether debug exceptions exit the guest.
    pub fn set_trap_debug_exceptions(&self, value: bool) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_trap_debug_exceptions(self.vcpu, value))
    }

    /// Gets whether debug-register accesses exit the guest.
    pub fn get_trap_debug_reg_accesses(&self) -> Result<bool> {
        let mut value = false;
        hv_unsafe_call!(hv_vcpu_get_trap_debug_reg_accesses(self.vcpu, &mut value))?;
        Ok(value)
    }

    /// Sets whether debug-register accesses exit the guest.
    pub fn set_trap_debug_reg_accesses(&self, value: bool) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_trap_debug_reg_accesses(self.vcpu, value))
    }

    /// Returns the cumulative execution time of a vCPU, in nanoseconds.
    pub fn get_exec_time(&self) -> Result<u64> {
        let mut time = 0;
        hv_unsafe_call!(hv_vcpu_get_exec_time(self.vcpu, &mut time))?;
        Ok(time)
    }

    /// Gets the virtual timer mask.
    pub fn get_vtimer_mask(&self) -> Result<bool> {
        let mut vtimer_is_masked = false;
        hv_unsafe_call!(hv_vcpu_get_vtimer_mask(self.vcpu, &mut vtimer_is_masked))?;
        Ok(vtimer_is_masked)
    }

    /// Sets or clears the virtual timer mask.
    pub fn set_vtimer_mask(&self, vtimer_is_masked: bool) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_vtimer_mask(self.vcpu, vtimer_is_masked))
    }

    /// Returns the vTimer offset for the vCPU ID you specify.
    pub fn get_vtimer_offset(&self) -> Result<u64> {
        let mut vtimer_offset = 0;
        hv_unsafe_call!(hv_vcpu_get_vtimer_offset(self.vcpu, &mut vtimer_offset))?;
        Ok(vtimer_offset)
    }

    /// Sets the vTimer offset to a value that you provide.
    pub fn set_vtimer_offset(&self, vtimer_offset: u64) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_vtimer_offset(self.vcpu, vtimer_offset))
    }
}

// -----------------------------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serial_test::*;
    use std::sync::{mpsc, Barrier};
    use std::thread;

    use crate::memory::PAGE_SIZE;
    use crate::memory::*;
    use crate::next_mem_addr;
    use crate::vm::*;

    use super::*;

    #[test]
    #[parallel]
    fn create_and_use_a_vcpu_configuration() {
        let config = VcpuConfig::default();
        assert!(config.get_feature_reg(FeatureReg::CTR_EL0).is_ok());
        assert!(config
            .get_ccsidr_el1_sys_reg_values(CacheType::DATA)
            .is_ok());
    }

    #[test]
    #[parallel]
    fn create_a_vcpu_instance() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu_config = VcpuConfig::default();

        // The vCPU should be created without issue.
        let vcpu = vm.vcpu_with_config(vcpu_config);
        assert!(vcpu.is_ok());
    }

    #[test]
    #[serial]
    fn run_multiple_concurrent_vcpus() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        // We want as many threads as possible.
        let thread_count = Vcpu::get_max_count().unwrap();

        let barrier = Barrier::new(thread_count as usize);
        let vm = VirtualMachine::new().unwrap();

        thread::scope(|s| {
            for _ in 0..thread_count {
                let vm_thread = vm.clone();
                let barrier = &barrier;
                s.spawn(move || {
                    // Create the vCPU and memory region.
                    let vcpu = vm_thread.vcpu_create().unwrap();
                    let addr = next_mem_addr();
                    let mut mem = vm_thread.memory_create(PAGE_SIZE).unwrap();
                    mem.map(addr, MemPerms::ReadWriteExec).unwrap();

                    // mov x1, x0
                    mem.write_u32(addr + 0, 0xaa0003e1).unwrap();
                    // brk #0
                    mem.write_u32(addr + 8, 0xd4200000).unwrap();

                    // X0 is set to the current vcpu id.
                    vcpu.set_reg(Reg::X0, vcpu.id()).unwrap();
                    vcpu.set_reg(Reg::PC, addr).unwrap();

                    // Waiting for all vCPUs to be initialized.
                    barrier.wait();

                    // Running all vCPUs.
                    vcpu.run().unwrap();

                    // Making sure that we retrieved the vCPU id and execution was stopped by
                    // the breakpoint.
                    assert_eq!(vcpu.get_reg(Reg::X1), Ok(vcpu.id()));
                    assert_eq!(vcpu.get_exit_info().reason, ExitReason::EXCEPTION);
                });
            }
        });
    }

    #[test]
    #[parallel]
    fn check_vcpu_handle_validity() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        // We want a barrier between all spawned threads as well as the main one.
        let thread_count = 4;
        let barrier = Barrier::new(thread_count + 1);

        // Array storing the vCPU handles.
        let mut handles = vec![];

        // Channel to retrieve the vCPU handles from.
        let (tx, rx) = mpsc::channel();

        thread::scope(|s| {
            for _ in 0..thread_count {
                let vm_thread = vm.clone();
                let tx_thread = tx.clone();
                let barrier = &barrier;
                s.spawn(move || {
                    // We create a thread and retrieve its handle...
                    let vcpu = vm_thread.vcpu_create().unwrap();
                    let handle = vcpu.get_handle();
                    // ... and then send it back to the main thread.
                    tx_thread.send(handle).unwrap();

                    // We wait for all threads to send their handles and for the main thread
                    // to receive them.
                    barrier.wait();
                });
            }
            // Wait for the vCPU handles from each thread.
            for _ in 0..thread_count {
                handles.push(rx.recv().unwrap());
            }
            // At this stage, all handles should be valid.
            assert!(handles.iter().all(|h| h.is_valid()));
            // All threads terminate and destroy the vCPUs.
            barrier.wait();
        });

        // Now all handles should be invalid.
        assert!(handles.iter().all(|h| !h.is_valid()));
    }

    #[test]
    #[parallel]
    fn get_and_set_pending_interrupt() {
        let _ = VirtualMachineStaticInstance::init().unwrap();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu = vm.vcpu_create().unwrap();
        assert_eq!(vcpu.get_pending_interrupt(InterruptType::IRQ), Ok(false));
        assert_eq!(vcpu.set_pending_interrupt(InterruptType::IRQ, true), Ok(()));
        assert_eq!(vcpu.get_pending_interrupt(InterruptType::IRQ), Ok(true));
    }

    macro_rules! get_and_set_reg_u64 {
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[test]
                #[parallel]
                fn $name() {
                    let _ = VirtualMachineStaticInstance::init();
                    let vm = VirtualMachineStaticInstance::get().unwrap();

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    let value = 0xdeadbeefdeadbeef;
                    let vcpu = vm.vcpu_create().unwrap();
                    assert_eq!(vcpu.set_reg(Reg::$reg, value), Ok(()));
                    assert_eq!(vcpu.get_reg(Reg::$reg), Ok(value));
                }
            )*
        }
    }

    get_and_set_reg_u64!(
        get_and_set_register_x0: X0,
        get_and_set_register_x1: X1,
        get_and_set_register_x2: X2,
        get_and_set_register_x3: X3,
        get_and_set_register_x4: X4,
        get_and_set_register_x5: X5,
        get_and_set_register_x6: X6,
        get_and_set_register_x7: X7,
        get_and_set_register_x8: X8,
        get_and_set_register_x9: X9,
        get_and_set_register_x10: X10,
        get_and_set_register_x11: X11,
        get_and_set_register_x12: X12,
        get_and_set_register_x13: X13,
        get_and_set_register_x14: X14,
        get_and_set_register_x15: X15,
        get_and_set_register_x16: X16,
        get_and_set_register_x17: X17,
        get_and_set_register_x18: X18,
        get_and_set_register_x19: X19,
        get_and_set_register_x20: X20,
        get_and_set_register_x21: X21,
        get_and_set_register_x22: X22,
        get_and_set_register_x23: X23,
        get_and_set_register_x24: X24,
        get_and_set_register_x25: X25,
        get_and_set_register_x26: X26,
        get_and_set_register_x27: X27,
        get_and_set_register_x28: X28,
        get_and_set_register_x29: X29,
        get_and_set_register_x30: X30,
        get_and_set_register_pc: PC,
    );

    macro_rules! get_and_set_reg_u32 {
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[test]
                #[parallel]
                fn $name() {
                    let _ = VirtualMachineStaticInstance::init();
                    let vm = VirtualMachineStaticInstance::get().unwrap();

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    let value = 0xdeadbeef;
                    let vcpu = vm.vcpu_create().unwrap();
                    assert_eq!(vcpu.set_reg(Reg::$reg, value.into()), Ok(()));
                    assert_eq!(vcpu.get_reg(Reg::$reg), Ok(value));
                }
            )*
        }
    }

    get_and_set_reg_u32!(
        get_and_set_register_fpcr: FPCR,
        get_and_set_register_fpsr: FPSR,
        get_and_set_register_cpsr: CPSR,
    );

    macro_rules! get_and_set_sys_reg {
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[test]
                #[serial]
                fn $name() {
                    vm_static_instance_reset();

                    #[cfg(feature = "macos-15-0")]
                    let vm = {
                        let mut vm_config = VirtualMachineConfig::default();
                        vm_config.set_el2_enabled(true).unwrap();
                        VirtualMachine::with_config(vm_config).unwrap()
                    };
                    #[cfg(not(feature = "macos-15-0"))]
                    let vm = VirtualMachine::new().unwrap();

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    let vcpu = vm.vcpu_create().unwrap();
                    let value = vcpu.get_sys_reg(SysReg::$reg);
                    assert!(value.is_ok());
                    assert_eq!(vcpu.set_sys_reg(SysReg::$reg, value.unwrap()), Ok(()));
                }
            )*
        }
    }

    get_and_set_sys_reg!(
        get_and_set_icv_reg_dbgbvr0_el1: DBGBVR0_EL1,
        get_and_set_icv_reg_dbgbcr0_el1: DBGBCR0_EL1,
        get_and_set_icv_reg_dbgwvr0_el1: DBGWVR0_EL1,
        get_and_set_icv_reg_dbgwcr0_el1: DBGWCR0_EL1,
        get_and_set_icv_reg_dbgbvr1_el1: DBGBVR1_EL1,
        get_and_set_icv_reg_dbgbcr1_el1: DBGBCR1_EL1,
        get_and_set_icv_reg_dbgwvr1_el1: DBGWVR1_EL1,
        get_and_set_icv_reg_dbgwcr1_el1: DBGWCR1_EL1,
        get_and_set_icv_reg_mdccint_el1: MDCCINT_EL1,
        get_and_set_icv_reg_mdscr_el1: MDSCR_EL1,
        get_and_set_icv_reg_dbgbvr2_el1: DBGBVR2_EL1,
        get_and_set_icv_reg_dbgbcr2_el1: DBGBCR2_EL1,
        get_and_set_icv_reg_dbgwvr2_el1: DBGWVR2_EL1,
        get_and_set_icv_reg_dbgwcr2_el1: DBGWCR2_EL1,
        get_and_set_icv_reg_dbgbvr3_el1: DBGBVR3_EL1,
        get_and_set_icv_reg_dbgbcr3_el1: DBGBCR3_EL1,
        get_and_set_icv_reg_dbgwvr3_el1: DBGWVR3_EL1,
        get_and_set_icv_reg_dbgwcr3_el1: DBGWCR3_EL1,
        get_and_set_icv_reg_dbgbvr4_el1: DBGBVR4_EL1,
        get_and_set_icv_reg_dbgbcr4_el1: DBGBCR4_EL1,
        get_and_set_icv_reg_dbgwvr4_el1: DBGWVR4_EL1,
        get_and_set_icv_reg_dbgwcr4_el1: DBGWCR4_EL1,
        get_and_set_icv_reg_dbgbvr5_el1: DBGBVR5_EL1,
        get_and_set_icv_reg_dbgbcr5_el1: DBGBCR5_EL1,
        get_and_set_icv_reg_dbgwvr5_el1: DBGWVR5_EL1,
        get_and_set_icv_reg_dbgwcr5_el1: DBGWCR5_EL1,
        get_and_set_icv_reg_dbgbvr6_el1: DBGBVR6_EL1,
        get_and_set_icv_reg_dbgbcr6_el1: DBGBCR6_EL1,
        get_and_set_icv_reg_dbgwvr6_el1: DBGWVR6_EL1,
        get_and_set_icv_reg_dbgwcr6_el1: DBGWCR6_EL1,
        get_and_set_icv_reg_dbgbvr7_el1: DBGBVR7_EL1,
        get_and_set_icv_reg_dbgbcr7_el1: DBGBCR7_EL1,
        get_and_set_icv_reg_dbgwvr7_el1: DBGWVR7_EL1,
        get_and_set_icv_reg_dbgwcr7_el1: DBGWCR7_EL1,
        get_and_set_icv_reg_dbgbvr8_el1: DBGBVR8_EL1,
        get_and_set_icv_reg_dbgbcr8_el1: DBGBCR8_EL1,
        get_and_set_icv_reg_dbgwvr8_el1: DBGWVR8_EL1,
        get_and_set_icv_reg_dbgwcr8_el1: DBGWCR8_EL1,
        get_and_set_icv_reg_dbgbvr9_el1: DBGBVR9_EL1,
        get_and_set_icv_reg_dbgbcr9_el1: DBGBCR9_EL1,
        get_and_set_icv_reg_dbgwvr9_el1: DBGWVR9_EL1,
        get_and_set_icv_reg_dbgwcr9_el1: DBGWCR9_EL1,
        get_and_set_icv_reg_dbgbvr10_el1: DBGBVR10_EL1,
        get_and_set_icv_reg_dbgbcr10_el1: DBGBCR10_EL1,
        get_and_set_icv_reg_dbgwvr10_el1: DBGWVR10_EL1,
        get_and_set_icv_reg_dbgwcr10_el1: DBGWCR10_EL1,
        get_and_set_icv_reg_dbgbvr11_el1: DBGBVR11_EL1,
        get_and_set_icv_reg_dbgbcr11_el1: DBGBCR11_EL1,
        get_and_set_icv_reg_dbgwvr11_el1: DBGWVR11_EL1,
        get_and_set_icv_reg_dbgwcr11_el1: DBGWCR11_EL1,
        get_and_set_icv_reg_dbgbvr12_el1: DBGBVR12_EL1,
        get_and_set_icv_reg_dbgbcr12_el1: DBGBCR12_EL1,
        get_and_set_icv_reg_dbgwvr12_el1: DBGWVR12_EL1,
        get_and_set_icv_reg_dbgwcr12_el1: DBGWCR12_EL1,
        get_and_set_icv_reg_dbgbvr13_el1: DBGBVR13_EL1,
        get_and_set_icv_reg_dbgbcr13_el1: DBGBCR13_EL1,
        get_and_set_icv_reg_dbgwvr13_el1: DBGWVR13_EL1,
        get_and_set_icv_reg_dbgwcr13_el1: DBGWCR13_EL1,
        get_and_set_icv_reg_dbgbvr14_el1: DBGBVR14_EL1,
        get_and_set_icv_reg_dbgbcr14_el1: DBGBCR14_EL1,
        get_and_set_icv_reg_dbgwvr14_el1: DBGWVR14_EL1,
        get_and_set_icv_reg_dbgwcr14_el1: DBGWCR14_EL1,
        get_and_set_icv_reg_dbgbvr15_el1: DBGBVR15_EL1,
        get_and_set_icv_reg_dbgbcr15_el1: DBGBCR15_EL1,
        get_and_set_icv_reg_dbgwvr15_el1: DBGWVR15_EL1,
        get_and_set_icv_reg_dbgwcr15_el1: DBGWCR15_EL1,
        get_and_set_icv_reg_midr_el1: MIDR_EL1,
        get_and_set_icv_reg_mpidr_el1: MPIDR_EL1,
        get_and_set_icv_reg_id_aa64pfr0_el1: ID_AA64PFR0_EL1,
        get_and_set_icv_reg_id_aa64pfr1_el1: ID_AA64PFR1_EL1,
        get_and_set_icv_reg_id_aa64dfr0_el1: ID_AA64DFR0_EL1,
        get_and_set_icv_reg_id_aa64dfr1_el1: ID_AA64DFR1_EL1,
        get_and_set_icv_reg_id_aa64isar0_el1: ID_AA64ISAR0_EL1,
        get_and_set_icv_reg_id_aa64isar1_el1: ID_AA64ISAR1_EL1,
        get_and_set_icv_reg_id_aa64mmfr0_el1: ID_AA64MMFR0_EL1,
        get_and_set_icv_reg_id_aa64mmfr1_el1: ID_AA64MMFR1_EL1,
        get_and_set_icv_reg_id_aa64mmfr2_el1: ID_AA64MMFR2_EL1,
        get_and_set_icv_reg_sctlr_el1: SCTLR_EL1,
        get_and_set_icv_reg_cpacr_el1: CPACR_EL1,
        get_and_set_icv_reg_ttbr0_el1: TTBR0_EL1,
        get_and_set_icv_reg_ttbr1_el1: TTBR1_EL1,
        get_and_set_icv_reg_tcr_el1: TCR_EL1,
        get_and_set_icv_reg_apiakeylo_el1: APIAKEYLO_EL1,
        get_and_set_icv_reg_apiakeyhi_el1: APIAKEYHI_EL1,
        get_and_set_icv_reg_apibkeylo_el1: APIBKEYLO_EL1,
        get_and_set_icv_reg_apibkeyhi_el1: APIBKEYHI_EL1,
        get_and_set_icv_reg_apdakeylo_el1: APDAKEYLO_EL1,
        get_and_set_icv_reg_apdakeyhi_el1: APDAKEYHI_EL1,
        get_and_set_icv_reg_apdbkeylo_el1: APDBKEYLO_EL1,
        get_and_set_icv_reg_apdbkeyhi_el1: APDBKEYHI_EL1,
        get_and_set_icv_reg_apgakeylo_el1: APGAKEYLO_EL1,
        get_and_set_icv_reg_apgakeyhi_el1: APGAKEYHI_EL1,
        get_and_set_icv_reg_spsr_el1: SPSR_EL1,
        get_and_set_icv_reg_elr_el1: ELR_EL1,
        get_and_set_icv_reg_sp_el0: SP_EL0,
        get_and_set_icv_reg_afsr0_el1: AFSR0_EL1,
        get_and_set_icv_reg_afsr1_el1: AFSR1_EL1,
        get_and_set_icv_reg_esr_el1: ESR_EL1,
        get_and_set_icv_reg_far_el1: FAR_EL1,
        get_and_set_icv_reg_par_el1: PAR_EL1,
        get_and_set_icv_reg_mair_el1: MAIR_EL1,
        get_and_set_icv_reg_amair_el1: AMAIR_EL1,
        get_and_set_icv_reg_vbar_el1: VBAR_EL1,
        get_and_set_icv_reg_contextidr_el1: CONTEXTIDR_EL1,
        get_and_set_icv_reg_tpidr_el1: TPIDR_EL1,
        get_and_set_icv_reg_cntkctl_el1: CNTKCTL_EL1,
        get_and_set_icv_reg_csselr_el1: CSSELR_EL1,
        get_and_set_icv_reg_tpidr_el0: TPIDR_EL0,
        get_and_set_icv_reg_tpidrro_el0: TPIDRRO_EL0,
        get_and_set_icv_reg_cntv_ctl_el0: CNTV_CTL_EL0,
        get_and_set_icv_reg_cntv_cval_el0: CNTV_CVAL_EL0,
        get_and_set_icv_reg_sp_el1: SP_EL1,
    );

    #[cfg(feature = "macos-15-0")]
    get_and_set_sys_reg!(
        get_and_set_icv_reg_actlr_el1: ACTLR_EL1,
        // TODO: get_and_set_icv_reg_cntp_ctl_el0: CNTP_CTL_EL0,
        // TODO: get_and_set_icv_reg_cntp_cval_el0: CNTP_CVAL_EL0,
        // TODO: get_and_set_icv_reg_cntp_tval_el0: CNTP_TVAL_EL0,
        get_and_set_icv_reg_cnthctl_el2: CNTHCTL_EL2,
        // TODO: get_and_set_icv_reg_cnthp_ctl_el2: CNTHP_CTL_EL2,
        // TODO: get_and_set_icv_reg_cnthp_cval_el2: CNTHP_CVAL_EL2,
        // TODO: get_and_set_icv_reg_cnthp_tval_el2: CNTHP_TVAL_EL2,
        get_and_set_icv_reg_cntvoff_el2: CNTVOFF_EL2,
        get_and_set_icv_reg_cptr_el2: CPTR_EL2,
        get_and_set_icv_reg_elr_el2: ELR_EL2,
        get_and_set_icv_reg_esr_el2: ESR_EL2,
        get_and_set_icv_reg_far_el2: FAR_EL2,
        get_and_set_icv_reg_hcr_el2: HCR_EL2,
        get_and_set_icv_reg_hpfar_el2: HPFAR_EL2,
        get_and_set_icv_reg_mair_el2: MAIR_EL2,
        //TODO: get_and_set_icv_reg_mdcr_el2: MDCR_EL2,
        get_and_set_icv_reg_sctlr_el2: SCTLR_EL2,
        get_and_set_icv_reg_spsr_el2: SPSR_EL2,
        get_and_set_icv_reg_sp_el2: SP_EL2,
        get_and_set_icv_reg_tcr_el2: TCR_EL2,
        get_and_set_icv_reg_tpidr_el2: TPIDR_EL2,
        get_and_set_icv_reg_ttbr0_el2: TTBR0_EL2,
        get_and_set_icv_reg_ttbr1_el2: TTBR1_EL2,
        get_and_set_icv_reg_vbar_el2: VBAR_EL2,
        get_and_set_icv_reg_vmpidr_el2: VMPIDR_EL2,
        get_and_set_icv_reg_vpidr_el2: VPIDR_EL2,
        get_and_set_icv_reg_vtcr_el2: VTCR_EL2,
        get_and_set_icv_reg_vttbr_el2: VTTBR_EL2,
    );

    #[cfg(feature = "macos-15-2")]
    get_and_set_sys_reg!(
        get_and_set_icv_reg_id_aa64zfr0_el1: ID_AA64ZFR0_EL1,
        get_and_set_icv_reg_id_aa64smfr0_el1: ID_AA64SMFR0_EL1,
        get_and_set_icv_reg_smpri_el1: SMPRI_EL1,
        get_and_set_icv_reg_smcr_el1: SMCR_EL1,
        get_and_set_icv_reg_scxtnum_el1: SCXTNUM_EL1,
        get_and_set_icv_reg_tpidr2_el0: TPIDR2_EL0,
        get_and_set_icv_reg_scxtnum_el0: SCXTNUM_EL0,
    );

    macro_rules! get_and_set_simd_fp_reg {
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[test]
                #[parallel]
                fn $name() {
                    let _ = VirtualMachineStaticInstance::init();
                    let vm = VirtualMachineStaticInstance::get().unwrap();

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    #[cfg(feature = "simd-nightly")]
                    let value = simd::u8x16::from_array([
                        0xde,0xad,0xbe,0xef,
                        0xde,0xad,0xbe,0xef,
                        0xde,0xad,0xbe,0xef,
                        0xde,0xad,0xbe,0xef
                    ]);
                    #[cfg(not(feature = "simd-nightly"))]
                    let value = 0xdeadbeefdeadbeefdeadbeefdeadbeef;
                    let vcpu = vm.vcpu_create().unwrap();
                    assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::$reg, value), Ok(()));
                    assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::$reg), Ok(value));
                }
            )*
        }
    }

    get_and_set_simd_fp_reg!(
        get_and_set_simd_fp_reg_q0: Q0,
        get_and_set_simd_fp_reg_q1: Q1,
        get_and_set_simd_fp_reg_q2: Q2,
        get_and_set_simd_fp_reg_q3: Q3,
        get_and_set_simd_fp_reg_q4: Q4,
        get_and_set_simd_fp_reg_q5: Q5,
        get_and_set_simd_fp_reg_q6: Q6,
        get_and_set_simd_fp_reg_q7: Q7,
        get_and_set_simd_fp_reg_q8: Q8,
        get_and_set_simd_fp_reg_q9: Q9,
        get_and_set_simd_fp_reg_q10: Q10,
        get_and_set_simd_fp_reg_q11: Q11,
        get_and_set_simd_fp_reg_q12: Q12,
        get_and_set_simd_fp_reg_q13: Q13,
        get_and_set_simd_fp_reg_q14: Q14,
        get_and_set_simd_fp_reg_q15: Q15,
        get_and_set_simd_fp_reg_q16: Q16,
        get_and_set_simd_fp_reg_q17: Q17,
        get_and_set_simd_fp_reg_q18: Q18,
        get_and_set_simd_fp_reg_q19: Q19,
        get_and_set_simd_fp_reg_q20: Q20,
        get_and_set_simd_fp_reg_q21: Q21,
        get_and_set_simd_fp_reg_q22: Q22,
        get_and_set_simd_fp_reg_q23: Q23,
        get_and_set_simd_fp_reg_q24: Q24,
        get_and_set_simd_fp_reg_q25: Q25,
        get_and_set_simd_fp_reg_q26: Q26,
        get_and_set_simd_fp_reg_q27: Q27,
        get_and_set_simd_fp_reg_q28: Q28,
        get_and_set_simd_fp_reg_q29: Q29,
        get_and_set_simd_fp_reg_q30: Q30,
        get_and_set_simd_fp_reg_q31: Q31,
    );

    #[cfg(feature = "macos-15-2")]
    #[test]
    #[parallel]
    fn get_and_set_sme_state() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu = vm.vcpu_create().unwrap();
        let state = SmeState {
            streaming_sve_mode_enabled: true,
            za_storage_enabled: true,
        };

        assert_eq!(vcpu.set_sme_state(&state), Ok(()));
        assert_eq!(vcpu.get_sme_state(), Ok(state));
    }

    macro_rules! get_and_set_sme_z_reg {
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[cfg(feature = "macos-15-2")]
                #[test]
                #[parallel]
                fn $name() {
                    let _ = VirtualMachineStaticInstance::init();
                    let vm = VirtualMachineStaticInstance::get().unwrap();

                    let vcpu = vm.vcpu_create().unwrap();

                    // Enabling streaming mode.
                    let state = SmeState {
                        streaming_sve_mode_enabled: true,
                        za_storage_enabled: false,
                    };
                    assert_eq!(vcpu.set_sme_state(&state), Ok(()));

                    let size: usize = VirtualMachineConfig::get_max_svl_bytes().unwrap();
                    let write_data = vec![0x42u8; size];
                    let mut read_data = vec![0x0u8; size];

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    assert_eq!(
                        vcpu.get_sme_z_reg(SmeZReg::$reg, &mut vec![0; 1]),
                        Err(HypervisorError::BadArgument));
                    assert_eq!(vcpu.set_sme_z_reg(SmeZReg::$reg, &write_data), Ok(()));
                    assert_eq!(vcpu.get_sme_z_reg(SmeZReg::$reg, &mut read_data), Ok(()));
                    assert_eq!(write_data, read_data);
                }
            )*
        }
    }

    get_and_set_sme_z_reg!(
        get_and_set_sme_z_reg_z0: Z0,
        get_and_set_sme_z_reg_z1: Z1,
        get_and_set_sme_z_reg_z2: Z2,
        get_and_set_sme_z_reg_z3: Z3,
        get_and_set_sme_z_reg_z4: Z4,
        get_and_set_sme_z_reg_z5: Z5,
        get_and_set_sme_z_reg_z6: Z6,
        get_and_set_sme_z_reg_z7: Z7,
        get_and_set_sme_z_reg_z8: Z8,
        get_and_set_sme_z_reg_z9: Z9,
        get_and_set_sme_z_reg_z10: Z10,
        get_and_set_sme_z_reg_z11: Z11,
        get_and_set_sme_z_reg_z12: Z12,
        get_and_set_sme_z_reg_z13: Z13,
        get_and_set_sme_z_reg_z14: Z14,
        get_and_set_sme_z_reg_z15: Z15,
        get_and_set_sme_z_reg_z16: Z16,
        get_and_set_sme_z_reg_z17: Z17,
        get_and_set_sme_z_reg_z18: Z18,
        get_and_set_sme_z_reg_z19: Z19,
        get_and_set_sme_z_reg_z20: Z20,
        get_and_set_sme_z_reg_z21: Z21,
        get_and_set_sme_z_reg_z22: Z22,
        get_and_set_sme_z_reg_z23: Z23,
        get_and_set_sme_z_reg_z24: Z24,
        get_and_set_sme_z_reg_z25: Z25,
        get_and_set_sme_z_reg_z26: Z26,
        get_and_set_sme_z_reg_z27: Z27,
        get_and_set_sme_z_reg_z28: Z28,
        get_and_set_sme_z_reg_z29: Z29,
        get_and_set_sme_z_reg_z30: Z30,
        get_and_set_sme_z_reg_z31: Z31,
    );

    macro_rules! get_and_set_sme_p_reg {
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[cfg(feature = "macos-15-2")]
                #[test]
                #[parallel]
                fn $name() {
                    let _ = VirtualMachineStaticInstance::init();
                    let vm = VirtualMachineStaticInstance::get().unwrap();

                    let vcpu = vm.vcpu_create().unwrap();

                    // Enabling streaming mode.
                    let state = SmeState {
                        streaming_sve_mode_enabled: true,
                        za_storage_enabled: false,
                    };
                    assert_eq!(vcpu.set_sme_state(&state), Ok(()));

                    let size: usize = VirtualMachineConfig::get_max_svl_bytes().unwrap() / 8;
                    let write_data = vec![0x42u8; size];
                    let mut read_data = vec![0x0u8; size];

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    assert_eq!(vcpu.get_sme_p_reg(
                        SmePReg::$reg, &mut vec![0; 1]),
                        Err(HypervisorError::BadArgument));
                    assert_eq!(vcpu.set_sme_p_reg(SmePReg::$reg, &write_data), Ok(()));
                    assert_eq!(vcpu.get_sme_p_reg(SmePReg::$reg, &mut read_data), Ok(()));
                    assert_eq!(write_data, read_data);
                }
            )*
        }
    }

    get_and_set_sme_p_reg!(
        get_and_set_sme_p_reg_p0: P0,
        get_and_set_sme_p_reg_p1: P1,
        get_and_set_sme_p_reg_p2: P2,
        get_and_set_sme_p_reg_p3: P3,
        get_and_set_sme_p_reg_p4: P4,
        get_and_set_sme_p_reg_p5: P5,
        get_and_set_sme_p_reg_p6: P6,
        get_and_set_sme_p_reg_p7: P7,
        get_and_set_sme_p_reg_p8: P8,
        get_and_set_sme_p_reg_p9: P9,
        get_and_set_sme_p_reg_p10: P10,
        get_and_set_sme_p_reg_p11: P11,
        get_and_set_sme_p_reg_p12: P12,
        get_and_set_sme_p_reg_p13: P13,
        get_and_set_sme_p_reg_p14: P14,
        get_and_set_sme_p_reg_p15: P15,
    );

    #[cfg(feature = "macos-15-2")]
    #[test]
    #[parallel]
    fn get_and_set_za_reg() {
        let _ = VirtualMachineStaticInstance::init().unwrap();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu = vm.vcpu_create().unwrap();

        // Enabling ZA storage.
        let state = SmeState {
            streaming_sve_mode_enabled: false,
            za_storage_enabled: true,
        };
        assert_eq!(vcpu.set_sme_state(&state), Ok(()));

        let size: usize = VirtualMachineConfig::get_max_svl_bytes().unwrap();
        let size = size * size;
        let write_data = vec![0x42u8; size];
        let mut read_data = vec![0x0u8; size];

        // Set the register to an arbitrary value and check that the same one is
        // read from it.
        assert_eq!(
            vcpu.get_sme_za_reg(&mut vec![0; 1]),
            Err(HypervisorError::BadArgument)
        );
        assert_eq!(vcpu.set_sme_za_reg(&write_data), Ok(()));
        assert_eq!(vcpu.get_sme_za_reg(&mut read_data), Ok(()));
        assert_eq!(write_data, read_data);
    }

    #[cfg(feature = "macos-15-2")]
    #[test]
    #[parallel]
    fn get_and_set_zt0_reg() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu = vm.vcpu_create().unwrap();

        // Enabling ZA storage.
        let state = SmeState {
            streaming_sve_mode_enabled: true,
            za_storage_enabled: true,
        };
        assert_eq!(vcpu.set_sme_state(&state), Ok(()));

        #[cfg(feature = "simd-nightly")]
        let write_data: SmeZt0 = simd::u8x64::from_array([0x42; 64]);
        #[cfg(feature = "simd-nightly")]
        let mut read_data: SmeZt0 = simd::u8x64::from_array([0; 64]);
        #[cfg(not(feature = "simd-nightly"))]
        let write_data: SmeZt0 = [0x42; 64];
        #[cfg(not(feature = "simd-nightly"))]
        let mut read_data: SmeZt0 = [0; 64];

        // Set the register to an arbitrary value and check that the same one is
        // read from it.
        assert_eq!(vcpu.set_sme_zt0_reg(&write_data), Ok(()));
        assert_eq!(vcpu.get_sme_zt0_reg(&mut read_data), Ok(()));
        assert_eq!(write_data, read_data);
    }

    #[cfg(feature = "macos-15-0")]
    #[test]
    #[serial]
    fn get_and_set_redistributor_base() {
        vm_static_instance_reset();

        let mut vm_config = VirtualMachineConfig::default();
        vm_config.set_el2_enabled(true).unwrap();

        let mut gic_config = GicConfig::new();
        let redistributor_base = 0x2000_0000;
        gic_config
            .set_redistributor_base(redistributor_base)
            .unwrap();

        let vm = VirtualMachine::with_gic(vm_config, gic_config).unwrap();

        // Set the register to an arbitrary value and check that the same one is
        // read from it.
        let vcpu = vm.vcpu_create().unwrap();
        assert_eq!(vcpu.set_sys_reg(SysReg::MPIDR_EL1, 1), Ok(()));
        assert_eq!(vcpu.get_redistributor_base(), Ok(redistributor_base));
    }

    macro_rules! get_and_set_icc_reg{
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[cfg(feature = "macos-15-2")]
                #[test]
                #[serial]
                fn $name() {
                    vm_static_instance_reset();

                    let mut vm_config = VirtualMachineConfig::default();
                    vm_config.set_el2_enabled(true).unwrap();

                    let mut gic_config = GicConfig::new();
                    let redistributor_base = 0x2000_0000;
                    gic_config.set_redistributor_base(redistributor_base).unwrap();

                    let vm = VirtualMachine::with_gic(vm_config, gic_config).unwrap();

                    let vcpu = vm.vcpu_create().unwrap();

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    let value = vcpu.get_icc_reg(GicIccReg::$reg);
                    assert!(value.is_ok());
                    assert_eq!(vcpu.set_icc_reg(GicIccReg::$reg, value.unwrap()), Ok(()));
                }
            )*
        }
    }

    get_and_set_icc_reg!(
        get_and_set_redistributor_reg_pmr_el1: PMR_EL1,
        get_and_set_redistributor_reg_bpr0_el1: BPR0_EL1,
        get_and_set_redistributor_reg_ap0r0_el1: AP0R0_EL1,
        get_and_set_redistributor_reg_ap1r0_el1: AP1R0_EL1,
        // TODO: get_and_set_redistributor_reg_rpr_el1: RPR_EL1,
        get_and_set_redistributor_reg_bpr1_el1: BPR1_EL1,
        get_and_set_redistributor_reg_ctlr_el1: CTLR_EL1,
        get_and_set_redistributor_reg_sre_el1: SRE_EL1,
        get_and_set_redistributor_reg_igrpen0_el1: IGRPEN0_EL1,
        get_and_set_redistributor_reg_igrpen1_el1: IGRPEN1_EL1,
        get_and_set_redistributor_reg_sre_el2: SRE_EL2,
    );

    macro_rules! get_and_set_ich_reg{
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[cfg(feature = "macos-15-2")]
                #[test]
                #[serial]
                fn $name() {
                    vm_static_instance_reset();

                    let mut vm_config = VirtualMachineConfig::default();
                    vm_config.set_el2_enabled(true).unwrap();

                    let mut gic_config = GicConfig::new();
                    let redistributor_base = 0x2000_0000;
                    gic_config.set_redistributor_base(redistributor_base).unwrap();

                    let vm = VirtualMachine::with_gic(vm_config, gic_config).unwrap();

                    let vcpu = vm.vcpu_create().unwrap();

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    let value = vcpu.get_ich_reg(GicIchReg::$reg);
                    assert!(value.is_ok());
                    assert_eq!(vcpu.set_ich_reg(GicIchReg::$reg, value.unwrap()), Ok(()));
                }
            )*
        }
    }

    get_and_set_ich_reg!(
        get_and_set_icc_reg_ap0r0_el2: AP0R0_EL2,
        get_and_set_icc_reg_ap1r0_el2: AP1R0_EL2,
        get_and_set_icc_reg_hcr_el2: HCR_EL2,
        // TODO: get_and_set_icc_reg_vtr_el2: VTR_EL2,
        // TODO: get_and_set_icc_reg_misr_el2: MISR_EL2,
        // TODO: get_and_set_icc_reg_eisr_el2: EISR_EL2,
        // TODO: get_and_set_icc_reg_elrsr_el2: ELRSR_EL2,
        get_and_set_icc_reg_vmcr_el2: VMCR_EL2,
        get_and_set_icc_reg_lr0_el2: LR0_EL2,
        get_and_set_icc_reg_lr1_el2: LR1_EL2,
        get_and_set_icc_reg_lr2_el2: LR2_EL2,
        get_and_set_icc_reg_lr3_el2: LR3_EL2,
        get_and_set_icc_reg_lr4_el2: LR4_EL2,
        get_and_set_icc_reg_lr5_el2: LR5_EL2,
        get_and_set_icc_reg_lr6_el2: LR6_EL2,
        get_and_set_icc_reg_lr7_el2: LR7_EL2,
        // TODO: get_and_set_icc_reg_lr8_el2: LR8_EL2,
        // TODO: get_and_set_icc_reg_lr9_el2: LR9_EL2,
        // TODO: get_and_set_icc_reg_lr10_el2: LR10_EL2,
        // TODO: get_and_set_icc_reg_lr11_el2: LR11_EL2,
        // TODO: get_and_set_icc_reg_lr12_el2: LR12_EL2,
        // TODO: get_and_set_icc_reg_lr13_el2: LR13_EL2,
        // TODO: get_and_set_icc_reg_lr14_el2: LR14_EL2,
        // TODO: get_and_set_icc_reg_lr15_el2: LR15_EL2,
    );

    macro_rules! get_and_set_icv_reg{
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[cfg(feature = "macos-15-2")]
                #[test]
                #[serial]
                fn $name() {
                    vm_static_instance_reset();

                    let mut vm_config = VirtualMachineConfig::default();
                    vm_config.set_el2_enabled(true).unwrap();

                    let mut gic_config = GicConfig::new();
                    let redistributor_base = 0x2000_0000;
                    gic_config.set_redistributor_base(redistributor_base).unwrap();

                    let vm = VirtualMachine::with_gic(vm_config, gic_config).unwrap();

                    let vcpu = vm.vcpu_create().unwrap();

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    let value = vcpu.get_icv_reg(GicIcvReg::$reg);
                    assert!(value.is_ok());
                    assert_eq!(vcpu.set_icv_reg(GicIcvReg::$reg, value.unwrap()), Ok(()));
                }
            )*
        }
    }

    get_and_set_icv_reg!(
        get_and_set_icv_reg_pmr_el1: PMR_EL1,
        get_and_set_icv_reg_bpr0_el1: BPR0_EL1,
        get_and_set_icv_reg_ap0r0_el1: AP0R0_EL1,
        get_and_set_icv_reg_ap1r0_el1: AP1R0_EL1,
        // TODO: get_and_set_icv_reg_rpr_el1: RPR_EL1,
        get_and_set_icv_reg_bpr1_el1: BPR1_EL1,
        get_and_set_icv_reg_ctlr_el1: CTLR_EL1,
        get_and_set_icv_reg_sre_el1: SRE_EL1,
        get_and_set_icv_reg_igrpen0_el1: IGRPEN0_EL1,
        get_and_set_icv_reg_igrpen1_el1: IGRPEN1_EL1,
    );

    macro_rules! get_and_set_redistributor_reg {
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[cfg(feature = "macos-15-2")]
                #[test]
                #[serial]
                fn $name() {
                    vm_static_instance_reset();

                    let vm_config = VirtualMachineConfig::default();
                    let mut gic_config = GicConfig::new();

                    gic_config.set_distributor_base(0x1000_0000).unwrap();
                    gic_config.set_redistributor_base(0x2000_0000).unwrap();
                    gic_config.set_msi_region_base(0x3000_0000).unwrap();

                    let _ = VirtualMachineStaticInstance::init_with_gic(vm_config, gic_config);
                    let vm = VirtualMachineStaticInstance::get_gic().unwrap();

                    let vcpu = vm.vcpu_create().unwrap();

                    // Set the register to an arbitrary value and check that the same one is
                    // read from it.
                    let value = vcpu.get_redistributor_reg(GicRedistributorReg::$reg);
                    assert_eq!(value, Ok(123));
                    assert!(value.is_ok());
                    let value = value.unwrap() + 1;
                    assert_eq!(vcpu.set_redistributor_reg(GicRedistributorReg::$reg, value + 1), Ok(()));
                    assert_eq!(vcpu.get_redistributor_reg(GicRedistributorReg::$reg), Ok(value + 1));
                }
            )*
        }
    }

    get_and_set_redistributor_reg!(
        // get_and_set_redistributor_reg_typer: TYPER,
        // get_and_set_redistributor_reg_pidr2: PIDR2,
        // get_and_set_redistributor_reg_igroupr0: IGROUPR0,
        // get_and_set_redistributor_reg_isenabler0: ISENABLER0,
        // get_and_set_redistributor_reg_icenabler0: ICENABLER0,
        // get_and_set_redistributor_reg_ispendr0: ISPENDR0,
        // get_and_set_redistributor_reg_icpendr0: ICPENDR0,
        // get_and_set_redistributor_reg_isactiver0: ISACTIVER0,
        // get_and_set_redistributor_reg_icactiver0: ICACTIVER0,
        // get_and_set_redistributor_reg_ipriorityr0: IPRIORITYR0,
        // get_and_set_redistributor_reg_ipriorityr1: IPRIORITYR1,
        // get_and_set_redistributor_reg_ipriorityr2: IPRIORITYR2,
        // get_and_set_redistributor_reg_ipriorityr3: IPRIORITYR3,
        // get_and_set_redistributor_reg_ipriorityr4: IPRIORITYR4,
        // get_and_set_redistributor_reg_ipriorityr5: IPRIORITYR5,
        // get_and_set_redistributor_reg_ipriorityr6: IPRIORITYR6,
        // get_and_set_redistributor_reg_ipriorityr7: IPRIORITYR7,
        // get_and_set_redistributor_reg_icfgr0: ICFGR0,
        // get_and_set_redistributor_reg_icfgr1: ICFGR1,
    );

    #[test]
    #[parallel]
    fn get_and_set_trap_debug() {
        let _ = VirtualMachineStaticInstance::init().unwrap();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu = vm.vcpu_create().unwrap();

        assert_eq!(vcpu.set_trap_debug_exceptions(true), Ok(()));
        assert_eq!(vcpu.get_trap_debug_exceptions(), Ok(true));
        assert_eq!(vcpu.set_trap_debug_reg_accesses(true), Ok(()));
        assert_eq!(vcpu.get_trap_debug_reg_accesses(), Ok(true));
    }

    #[test]
    #[parallel]
    fn get_and_set_vtimer_info() {
        let _ = VirtualMachineStaticInstance::init().unwrap();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu = vm.vcpu_create().unwrap();
        assert_eq!(vcpu.set_vtimer_offset(0xdeadbeefdeadbeef), Ok(()));
        assert_eq!(vcpu.get_vtimer_offset(), Ok(0xdeadbeefdeadbeef));
        assert_eq!(vcpu.set_vtimer_mask(true), Ok(()));
        assert_eq!(vcpu.get_vtimer_mask(), Ok(true));
    }

    #[test]
    #[parallel]
    fn vcpu_execution_time() {
        let _ = VirtualMachineStaticInstance::init().unwrap();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        // Create the vCPU and memory region.
        let vcpu = vm.vcpu_create().unwrap();
        let addr = next_mem_addr();
        let mut mem = vm.memory_create(PAGE_SIZE).unwrap();
        mem.map(addr, MemPerms::ReadExec).unwrap();

        let buf = vec![
            0x00, 0x00, 0x80, 0xd2, 0xe1, 0xff, 0x9f, 0xd2, 0x00, 0x04, 0x00, 0x91, 0x1f, 0x00,
            0x01, 0xeb, 0xc9, 0xff, 0xff, 0x54, 0xc0, 0x03, 0x5f, 0xd6,
        ];

        mem.write(addr, &buf).unwrap();

        vcpu.set_reg(Reg::X0, 0).unwrap();
        vcpu.set_reg(Reg::X1, 0xffff).unwrap();
        vcpu.set_reg(Reg::PC, addr).unwrap();

        vcpu.run().unwrap();

        let exec_time = vcpu.get_exec_time();
        assert!(exec_time.is_ok());
        assert!(exec_time.unwrap() > 0);
    }
}
