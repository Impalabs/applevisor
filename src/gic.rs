//! Functions and objects related to the Global Interrupt Controller.

use core::ffi::c_void;
use std::hash::Hash;
use std::sync::Arc;

use applevisor_sys::*;

use crate::error::*;
use crate::hv_unsafe_call;
use crate::vm::*;

// -----------------------------------------------------------------------------------------------
// Global Interrupt Controller
// -----------------------------------------------------------------------------------------------

/// Type of an ARM GIC interrupt id.
#[cfg(feature = "macos-15-0")]
pub type GicIntId = hv_gic_intid_t;

/// Type of an ARM GIC distributor register.
#[cfg(feature = "macos-15-0")]
pub type GicDistributorReg = hv_gic_distributor_reg_t;

/// Type of an ARM GIC redistributor register.
#[cfg(feature = "macos-15-0")]
pub type GicRedistributorReg = hv_gic_redistributor_reg_t;

/// Type of an ARM GIC ICC system control register.
#[cfg(feature = "macos-15-0")]
pub type GicIccReg = hv_gic_icc_reg_t;

/// Type of an ARM GIC virtualization control system register.
#[cfg(feature = "macos-15-0")]
pub type GicIchReg = hv_gic_ich_reg_t;

/// Type of an ARM GIC ICV system control register.
#[cfg(feature = "macos-15-0")]
pub type GicIcvReg = hv_gic_icv_reg_t;

/// Type of an ARM GIC ICV system control register.
#[cfg(feature = "macos-15-0")]
pub type GicMsiReg = hv_gic_msi_reg_t;

/// Represents a global interrupt controller configuration object.
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg(feature = "macos-15-0")]
pub struct GicConfig(pub(crate) hv_gic_config_t);

#[cfg(feature = "macos-15-0")]
impl GicConfig {
    /// Creates a global interrupt controller configuration object.
    pub fn new() -> Self {
        Self(unsafe { hv_gic_config_create() })
    }

    /// Set the GIC distributor region base address.
    pub fn set_distributor_base(&mut self, distributor_base_address: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_config_set_distributor_base(
            self.0,
            distributor_base_address,
        ))?;
        Ok(())
    }

    /// Set the GIC redistributor region base address.
    ///
    /// # Discussion
    ///
    /// Guest physical address for redistributor base aligned to byte value returned by
    /// [`GicConfig::get_redistributor_base_alignment`]. The redistributor region will contain
    /// redistributors for all vCPUs supported by the virtual machine.
    pub fn set_redistributor_base(&mut self, redistributor_base_address: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_config_set_redistributor_base(
            self.0,
            redistributor_base_address,
        ))?;
        Ok(())
    }

    /// Set the GIC MSI region base address.
    pub fn set_msi_region_base(&mut self, msi_region_base_address: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_config_set_msi_region_base(
            self.0,
            msi_region_base_address,
        ))?;
        Ok(())
    }

    /// Sets the range of MSIs supported.
    pub fn set_msi_interrupt_range(
        &mut self,
        msi_intid_base: u32,
        msi_intid_count: u32,
    ) -> Result<()> {
        hv_unsafe_call!(hv_gic_config_set_msi_interrupt_range(
            self.0,
            msi_intid_base,
            msi_intid_count,
        ))?;
        Ok(())
    }

    /// Gets the size in bytes of the GIC distributor region.
    pub fn get_distributor_size() -> Result<usize> {
        let mut distributor_size = 0;
        hv_unsafe_call!(hv_gic_get_distributor_size(&mut distributor_size))?;
        Ok(distributor_size)
    }

    /// Gets the alignment in bytes for the base address of the GIC distributor region.
    pub fn get_distributor_base_alignment() -> Result<usize> {
        let mut distributor_base_alignment = 0;
        hv_unsafe_call!(hv_gic_get_distributor_base_alignment(
            &mut distributor_base_alignment
        ))?;
        Ok(distributor_base_alignment)
    }

    /// Gets the total size in bytes of the GIC redistributor region.
    pub fn get_redistributor_region_size() -> Result<usize> {
        let mut redistributor_region_size = 0;
        hv_unsafe_call!(hv_gic_get_redistributor_region_size(
            &mut redistributor_region_size
        ))?;
        Ok(redistributor_region_size)
    }

    /// Gets the size in bytes of a single GIC redistributor.
    pub fn get_redistributor_size() -> Result<usize> {
        let mut redistributor_size = 0;
        hv_unsafe_call!(hv_gic_get_redistributor_size(&mut redistributor_size))?;
        Ok(redistributor_size)
    }

    /// Gets the alignment in bytes for the base address of the GIC redistributor region.
    pub fn get_redistributor_base_alignment() -> Result<usize> {
        let mut redistributor_base_alignment = 0;
        hv_unsafe_call!(hv_gic_get_redistributor_base_alignment(
            &mut redistributor_base_alignment
        ))?;
        Ok(redistributor_base_alignment)
    }

    /// Gets the size in bytes of the GIC MSI region.
    pub fn get_msi_region_size() -> Result<usize> {
        let mut msi_region_size = 0;
        hv_unsafe_call!(hv_gic_get_msi_region_size(&mut msi_region_size))?;
        Ok(msi_region_size)
    }

    /// Gets the alignment in bytes for the base address of the GIC MSI region.
    pub fn get_msi_region_base_alignment() -> Result<usize> {
        let mut msi_region_base_alignment = 0;
        hv_unsafe_call!(hv_gic_get_msi_region_base_alignment(
            &mut msi_region_base_alignment
        ))?;
        Ok(msi_region_base_alignment)
    }

    /// Gets the range of SPIs supported.
    pub fn get_spi_interrupt_range() -> Result<(u32, u32)> {
        let mut base = 0;
        let mut count = 0;
        hv_unsafe_call!(hv_gic_get_spi_interrupt_range(&mut base, &mut count))?;
        Ok((base, count))
    }

    /// Gets the interrupt id for reserved interrupts.
    pub fn get_intid(interrupt: GicIntId) -> Result<u32> {
        let mut intid = 0;
        hv_unsafe_call!(hv_gic_get_intid(interrupt, &mut intid))?;
        Ok(intid)
    }
}

#[cfg(feature = "macos-15-0")]
impl Default for GicConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "macos-15-0")]
impl Drop for GicConfig {
    fn drop(&mut self) {
        unsafe { os_release(self.0) }
    }
}

/// Represents a global interrupt controller state object.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg(feature = "macos-15-0")]
pub struct GicState {
    handle: hv_gic_state_t,
    /// Strong reference to the virtual machine this vCPU belongs to.
    pub(crate) _guard_vm: Arc<()>,
}

#[cfg(feature = "macos-15-0")]
impl GicState {
    pub fn size(&self) -> Result<usize> {
        let mut size = 0;
        hv_unsafe_call!(hv_gic_state_get_size(self.handle, &mut size))?;
        Ok(size)
    }

    pub fn get(&mut self, data: &mut [u8]) -> Result<()> {
        if data.len() < self.size()? {
            return Err(HypervisorError::BadArgument);
        }
        hv_unsafe_call!(hv_gic_state_get_data(
            self.handle,
            data.as_mut_ptr() as *mut c_void,
        ))
    }

    /// Set state for GIC device to be restored.
    pub fn set(&self, data: &[u8]) -> Result<()> {
        hv_unsafe_call!(hv_gic_set_state(data.as_ptr() as *const c_void, data.len()))
    }
}

#[cfg(feature = "macos-15-0")]
impl std::ops::Drop for GicState {
    fn drop(&mut self) {
        unsafe { os_release(self.handle) }
    }
}

#[cfg(feature = "macos-15-0")]
impl VirtualMachineInstance<GicEnabled> {
    /// Resets the GIC device.
    ///
    /// # Discussion
    ///
    /// When the virtual machine is being reset, call this function to reset the
    /// GIC distributor, redistributor registers and the internal state of the
    /// device.
    pub fn gic_reset(&self) -> Result<()> {
        hv_unsafe_call!(hv_gic_reset())?;
        Ok(())
    }

    /// Creates a global interrupt controller configuration object.
    pub fn gic_state_create(&self) -> Result<GicState> {
        let handle = unsafe { hv_gic_state_create() };
        if handle.is_null() {
            return Err(HypervisorError::Error);
        }
        Ok(GicState {
            handle,
            // Safe to unwrap here, it is only emptied when the VM object is dropped.
            _guard_vm: Arc::clone(self._guard.as_ref().unwrap()),
        })
    }

    /// Trigger a Shared Peripheral Interrupt (SPI).
    pub fn gic_set_spi(&self, intid: u32, level: bool) -> Result<()> {
        hv_unsafe_call!(hv_gic_set_spi(intid, level))?;
        Ok(())
    }

    /// Send a Message Signaled Interrupt (MSI).
    pub fn gic_send_msi(&self, address: hv_ipa_t, intid: u32) -> Result<()> {
        hv_unsafe_call!(hv_gic_send_msi(address, intid))?;
        Ok(())
    }

    /// Read a GIC distributor register.
    pub fn gic_get_distributor_reg(&self, reg: GicDistributorReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_gic_get_distributor_reg(reg, &mut value))?;
        Ok(value)
    }

    /// Write a GIC distributor register.
    pub fn gic_set_distributor_reg(&self, reg: GicDistributorReg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_set_distributor_reg(reg, value))?;
        Ok(())
    }

    /// Read a GIC distributor MSI register.
    pub fn gic_get_msi_reg(&self, reg: GicMsiReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_gic_get_msi_reg(reg, &mut value))?;
        Ok(value)
    }

    /// Write a GIC distributor MSI register.
    pub fn gic_set_msi_reg(&self, reg: GicMsiReg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_gic_set_msi_reg(reg, value))?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serial_test::*;

    use crate::vm::*;

    use super::*;

    #[test]
    #[parallel]
    fn create_and_use_a_gic_configuration() {
        let mut config = GicConfig::default();

        // Set the distributor, redistributor, and msi region base addresses.
        assert_eq!(config.set_distributor_base(0x1000_0000), Ok(()));
        assert_eq!(config.set_redistributor_base(0x2000_0000), Ok(()));
        assert_eq!(config.set_msi_region_base(0x3000_0000), Ok(()));

        // Range of SPI interrupt number.
        let range = GicConfig::get_spi_interrupt_range();
        assert!(range.is_ok());
        let (base, count) = range.unwrap();

        // MSI interrupts should be within the SPI range.
        assert_eq!(config.set_msi_interrupt_range(base, count), Ok(()));

        // Get various configuration values.
        assert!(GicConfig::get_distributor_size().is_ok());
        assert!(GicConfig::get_distributor_base_alignment().is_ok());
        assert!(GicConfig::get_redistributor_region_size().is_ok());
        assert!(GicConfig::get_redistributor_size().is_ok());
        assert!(GicConfig::get_redistributor_base_alignment().is_ok());
        assert!(GicConfig::get_msi_region_size().is_ok());
        assert!(GicConfig::get_msi_region_base_alignment().is_ok());

        // Check the reserved interrupts.
        assert!(GicConfig::get_intid(GicIntId::MAINTENANCE).is_ok());
        assert!(GicConfig::get_intid(GicIntId::PERFORMANCE_MONITOR).is_ok());
        assert!(GicConfig::get_intid(GicIntId::EL1_VIRTUAL_TIMER).is_ok());
        assert!(GicConfig::get_intid(GicIntId::EL1_PHYSICAL_TIMER).is_ok());
        assert!(GicConfig::get_intid(GicIntId::EL2_PHYSICAL_TIMER).is_ok());
    }

    #[test]
    #[parallel]
    fn try_to_create_a_gic_configuration_with_out_of_bounds_values() {
        let mut config = GicConfig::default();

        let distributor_size = GicConfig::get_distributor_size().unwrap() as u64;
        let distributor_base_alignment =
            GicConfig::get_distributor_base_alignment().unwrap() as u64;
        // Base address that would make the region end address overflow.
        assert_eq!(
            config.set_distributor_base(u64::MAX - distributor_size + 1),
            Err(HypervisorError::BadArgument)
        );
        assert_eq!(
            config
                .set_distributor_base(u64::MAX - distributor_size - distributor_base_alignment + 1),
            Ok(())
        );

        let redistributor_size = GicConfig::get_redistributor_region_size().unwrap() as u64;
        let redistributor_base_alignment =
            GicConfig::get_redistributor_base_alignment().unwrap() as u64;
        // Base address that would make the region end address overflow.
        assert_eq!(
            config.set_redistributor_base(u64::MAX - redistributor_size + 1),
            Err(HypervisorError::BadArgument)
        );
        assert_eq!(
            config.set_redistributor_base(
                u64::MAX - redistributor_size - redistributor_base_alignment + 1
            ),
            Ok(())
        );

        let msi_region_size = GicConfig::get_msi_region_size().unwrap() as u64;
        let msi_region_base_alignment = GicConfig::get_msi_region_base_alignment().unwrap() as u64;
        // Base address that would make the region end address overflow.
        assert_eq!(
            config.set_msi_region_base(u64::MAX - msi_region_size + 1),
            Err(HypervisorError::BadArgument)
        );
        assert_eq!(
            config.set_msi_region_base(u64::MAX - msi_region_size - msi_region_base_alignment + 1),
            Ok(())
        );

        // MSI interrupt IDs outside of the SPI interrupts range.
        assert_eq!(
            config.set_msi_interrupt_range(0, u32::MAX),
            Err(HypervisorError::BadArgument)
        );
    }

    #[test]
    #[serial]
    fn create_and_change_gic_state() {
        vm_static_instance_reset();

        // Configure a VM with a GIC.
        let vm_config = VirtualMachineConfig::default();
        let mut gic_config = GicConfig::default();
        gic_config.set_distributor_base(0x2000_0000).unwrap();
        let vm = VirtualMachine::with_gic(vm_config, gic_config).unwrap();

        let state = vm.gic_state_create();
        assert!(state.is_ok());
        let mut state = state.unwrap();

        let state_size = state.size();
        assert!(state_size.is_ok());
        let state_size = state_size.unwrap();

        let mut data = vec![0u8; state_size];
        assert_eq!(state.get(&mut data), Ok(()));
        assert_eq!(state.set(&data), Ok(()));

        // Trying to set an invalid state.
        data[0] = 0;
        assert_eq!(state.set(&data), Err(HypervisorError::BadArgument));

        // Trying to get and set a state with a buffer too small.
        let mut data = vec![0u8; state_size - 0x10];
        assert_eq!(state.get(&mut data), Err(HypervisorError::BadArgument));
        assert_eq!(state.set(&data), Err(HypervisorError::BadArgument));
    }

    macro_rules! get_and_set_distributor_reg {
        ($($name:ident: $reg:ident,)*) => {
            $(
                #[cfg(feature = "macos-15-2")]
                #[test]
                #[serial]
                fn $name() {
                    vm_static_instance_reset();

                    let vm_config = VirtualMachineConfig::default();
                    let mut gic_config = GicConfig::new();
                    let redistributor_base = 0x2000_0000;
                    gic_config.set_redistributor_base(redistributor_base).unwrap();

                    let vm = VirtualMachine::with_gic(vm_config, gic_config).unwrap();

                    let _ = vm.vcpu_create().unwrap();

                    let value = vm.gic_get_distributor_reg(GicDistributorReg::$reg);
                    assert!(value.is_ok());
                    let value = value.unwrap() + 1;
                    assert_eq!(vm.gic_set_distributor_reg(GicDistributorReg::$reg, value), Ok(()));
                }
            )*
        }
    }

    get_and_set_distributor_reg!(
        get_and_set_distributor_reg_typer: TYPER,
        get_and_set_distributor_reg_pidr2: PIDR2,
        get_and_set_distributor_reg_igroupr0: IGROUPR0,
        get_and_set_distributor_reg_isenabler0: ISENABLER0,
        get_and_set_distributor_reg_icenabler0: ICENABLER0,
        get_and_set_distributor_reg_ispendr0: ISPENDR0,
        get_and_set_distributor_reg_icpendr0: ICPENDR0,
        get_and_set_distributor_reg_isactiver0: ISACTIVER0,
        get_and_set_distributor_reg_icactiver0: ICACTIVER0,
        get_and_set_distributor_reg_ipriorityr0: IPRIORITYR0,
        get_and_set_distributor_reg_ipriorityr1: IPRIORITYR1,
        get_and_set_distributor_reg_ipriorityr2: IPRIORITYR2,
        get_and_set_distributor_reg_ipriorityr3: IPRIORITYR3,
        get_and_set_distributor_reg_ipriorityr4: IPRIORITYR4,
        get_and_set_distributor_reg_ipriorityr5: IPRIORITYR5,
        get_and_set_distributor_reg_ipriorityr6: IPRIORITYR6,
        get_and_set_distributor_reg_ipriorityr7: IPRIORITYR7,
        get_and_set_distributor_reg_icfgr0: ICFGR0,
        get_and_set_distributor_reg_icfgr1: ICFGR1,
    );

    macro_rules! get_and_set_msi_reg {
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
                    let (base, count) = GicConfig::get_spi_interrupt_range().unwrap();
                    gic_config.set_msi_interrupt_range(base, count).unwrap();

                    let vm = VirtualMachine::with_gic(vm_config, gic_config).unwrap();

                    let _ = vm.vcpu_create().unwrap();

                    let value = vm.gic_get_msi_reg(GicMsiReg::$reg);
                    assert!(value.is_ok());
                    let value = value.unwrap() + 1;
                    assert_eq!(vm.gic_set_msi_reg(GicMsiReg::$reg, value), Ok(()));
                }
            )*
        }
    }

    get_and_set_msi_reg!(
        get_and_set_msi_reg_typer: TYPER,
        get_and_set_msi_reg_set_spi_nsr: SET_SPI_NSR,
    );

    #[test]
    #[serial]
    fn gic_operations() {
        vm_static_instance_reset();

        let vm_config = VirtualMachineConfig::default();
        let mut gic_config = GicConfig::default();
        gic_config.set_distributor_base(0x1000_0000).unwrap();
        gic_config.set_redistributor_base(0x2000_0000).unwrap();
        gic_config.set_msi_region_base(0x3000_0000).unwrap();

        // The first half of the SPI range is set as MSIs.
        let (base, count) = GicConfig::get_spi_interrupt_range().unwrap();
        gic_config.set_msi_interrupt_range(base, count / 2).unwrap();

        let vm = VirtualMachine::with_gic(vm_config, gic_config).unwrap();

        assert_eq!(vm.gic_reset(), Ok(()));
        let addr = 0x3000_0000 + GicMsiReg::SET_SPI_NSR as u64;

        // Sending MSIs.
        let msi_start = base;
        let msi_end = base + count / 2;
        assert_eq!(
            vm.gic_send_msi(addr, msi_start - 1),
            Err(HypervisorError::BadArgument)
        );
        for id in msi_start..msi_end {
            assert_eq!(vm.gic_send_msi(addr, id), Ok(()));
        }

        // Triggering SPIs.
        let spi_start = base + count / 2;
        let spi_end = base + count;
        assert_eq!(
            vm.gic_set_spi(spi_start - 1, false),
            Err(HypervisorError::BadArgument)
        );
        for id in spi_start..spi_end {
            assert_eq!(vm.gic_set_spi(id, false), Ok(()));
        }
    }
}
