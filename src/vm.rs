//! Creation and management of virtual machines.

use std::marker::PhantomData;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};

use applevisor_sys::*;

use crate::error::*;
#[cfg(feature = "macos-15-0")]
use crate::gic::*;
use crate::hv_unsafe_call;
use crate::memory::*;
use crate::vcpu::*;

// -----------------------------------------------------------------------------------------------
// Virtual Machine Config
// -----------------------------------------------------------------------------------------------

/// Supported intermediate physical address (IPA) granules.
#[cfg(feature = "macos-26-0")]
pub type IpaGranule = hv_ipa_granule_t;

/// Represents a virtual machine configuration object.
#[derive(Debug)]
#[cfg(feature = "macos-13-0")]
pub struct VirtualMachineConfig(hv_vm_config_t);

#[cfg(feature = "macos-13-0")]
impl Default for VirtualMachineConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "macos-13-0")]
impl Drop for VirtualMachineConfig {
    fn drop(&mut self) {
        unsafe { os_release(self.0) }
    }
}

#[cfg(feature = "macos-13-0")]
impl VirtualMachineConfig {
    /// Creates a virtual machine configuration object.
    pub fn new() -> Self {
        Self(unsafe { hv_vm_config_create() })
    }

    /// Returns the maximum intermediate physical address bit length.
    ///
    /// # Discussion
    ///
    /// The bit length is the number of valid bits from an intermediate physical address (IPA).
    /// For example, max IPA bit length of 36 means only the least significant 36 bits of an IPA
    /// are valid, and covers a 64GB range.
    pub fn get_max_ipa_size() -> Result<u32> {
        let mut ipa_bit_length = 0;
        hv_unsafe_call!(hv_vm_config_get_max_ipa_size(&mut ipa_bit_length))?;
        Ok(ipa_bit_length)
    }

    /// Returns the default intermediate physical address bit length.
    ///
    /// # Discussion
    ///
    /// This default IPA size is used if the IPA size is not set explicitly.
    pub fn get_default_ipa_size() -> Result<u32> {
        let mut ipa_bit_length = 0;
        hv_unsafe_call!(hv_vm_config_get_default_ipa_size(&mut ipa_bit_length))?;
        Ok(ipa_bit_length)
    }

    /// Returns intermediate physical address bit length in configuration.
    pub fn get_ipa_size(&self) -> Result<u32> {
        let mut ipa_bit_length = 0;
        hv_unsafe_call!(hv_vm_config_get_ipa_size(self.0, &mut ipa_bit_length))?;
        Ok(ipa_bit_length)
    }

    /// Sets intermediate physical address bit length in virtual machine configuration.
    ///
    /// # Discussion
    ///
    /// VM IPA size should be no greater than the max IPA size from
    /// [`VirtualMachineConfig::get_max_ipa_size`]
    pub fn set_ipa_size(&mut self, ipa_bit_length: u32) -> Result<()> {
        hv_unsafe_call!(hv_vm_config_set_ipa_size(self.0, ipa_bit_length))
    }

    /// Returns whether or not EL2 is supported on the current platform.
    #[cfg(feature = "macos-15-0")]
    pub fn get_el2_supported() -> Result<bool> {
        let mut el2_supported = false;
        hv_unsafe_call!(hv_vm_config_get_el2_supported(&mut el2_supported))?;
        Ok(el2_supported)
    }

    /// Returns whether or not EL2 is enabled for a VM configuration.
    #[cfg(feature = "macos-15-0")]
    pub fn get_el2_enabled(&self) -> Result<bool> {
        let mut el2_enabled = false;
        hv_unsafe_call!(hv_vm_config_get_el2_enabled(self.0, &mut el2_enabled))?;
        Ok(el2_enabled)
    }

    /// Sets whether or not EL2 is enabled for a VM configuration.
    #[cfg(feature = "macos-15-0")]
    pub fn set_el2_enabled(&mut self, el2_enabled: bool) -> Result<()> {
        hv_unsafe_call!(hv_vm_config_set_el2_enabled(self.0, el2_enabled))?;
        Ok(())
    }

    /// Returns the value of the maximum Streaming Vector Length (SVL) in bytes.
    ///
    /// # Discussion
    ///
    /// This is the maximum SVL that guests may use and separate from the effective SVL that
    /// guests may set using [`SysReg::SMCR_EL1`].
    ///
    /// Returns error [`HypervisorError::Unsupported`] if SME is not supported.
    #[cfg(feature = "macos-15-2")]
    pub fn get_max_svl_bytes() -> Result<usize> {
        let mut value = 0;
        hv_unsafe_call!(hv_sme_config_get_max_svl_bytes(&mut value))?;
        Ok(value)
    }

    /// Return the default intermediate physical address granule.
    #[cfg(feature = "macos-26-0")]
    pub fn get_default_ipa_granule() -> Result<IpaGranule> {
        let mut value = IpaGranule::HV_IPA_GRANULE_4KB;
        hv_unsafe_call!(hv_vm_config_get_default_ipa_granule(&mut value))?;
        Ok(value)
    }

    /// Return the intermediate physical address granule size in virtual machine configuration.
    #[cfg(feature = "macos-26-0")]
    pub fn get_ipa_granule(&self) -> Result<IpaGranule> {
        let mut value = IpaGranule::HV_IPA_GRANULE_4KB;
        hv_unsafe_call!(hv_vm_config_get_ipa_granule(self.0, &mut value))?;
        Ok(value)
    }

    /// Set the intermediate physical address granule size in virtual machine configuration.
    #[cfg(feature = "macos-26-0")]
    pub fn set_ipa_granule(&mut self, granule: IpaGranule) -> Result<()> {
        hv_unsafe_call!(hv_vm_config_set_ipa_granule(self.0, granule))?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------------------------
// Virtual Machine
// -----------------------------------------------------------------------------------------------

/// Namespace for method instanciating virtual machines.
pub struct VirtualMachine;

impl VirtualMachine {
    /// Creates a new virtual machine instance for the current process.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// // Creates the global instance using the configurations above.
    /// let vm = VirtualMachine::new()?;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Result<VirtualMachineInstance<GicDisabled>> {
        hv_unsafe_call!(hv_vm_create(ptr::null_mut()))?;
        Ok(VirtualMachineInstance::<GicDisabled> {
            _guard: Some(Arc::new(())),
            _phantom: PhantomData,
        })
    }

    /// Creates a new virtual machine instance for the current process using a user-provided
    /// [`VirtualMachineConfig`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// // Custom configuration for the virtual machine.
    /// let mut vm_config = VirtualMachineConfig::default();
    /// vm_config.set_el2_enabled(true)?;
    /// // Custom configuration for the GIC.
    /// let mut gic_config = GicConfig::default();
    /// gic_config.set_redistributor_base(0x2000_0000)?;
    ///
    /// // Creates the global instance using the configurations above.
    /// let vm = VirtualMachineInstance::with_config(vm_config)?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "macos-13-0")]
    pub fn with_config(
        config: VirtualMachineConfig,
    ) -> Result<VirtualMachineInstance<GicDisabled>> {
        hv_unsafe_call!(hv_vm_create(config.0))?;
        Ok(VirtualMachineInstance::<GicDisabled> {
            _guard: Some(Arc::new(())),
            _phantom: PhantomData,
        })
    }

    /// Creates a new virtual machine instance for the current process using a user-provided
    /// [`VirtualMachineConfig`], as well as an ARM Generic Interrupt Controller (GIC) v3 device.
    ///
    /// # Discussion
    ///
    /// There must only be a single GIC instance per virtual machine. The device supports a
    /// distributor, redistributors, msi and GIC CPU system registers. When EL2 is enabled, the
    /// device supports GIC hypervisor control registers which are used by the guest hypervisor for
    /// injecting interrupts to its guest.
    ///
    /// GIC v3 uses affinity based interrupt routing. vCPU's must set affinity values in their
    /// [`SysReg::MPIDR_EL1`] register. Once the virtual machine vcpus are running, its topology
    /// is considered final. Destroy vcpus only when you are tearing down the virtual machine.
    ///
    /// GIC MSI support is only provided if both an MSI region base address is configured and an
    /// MSI interrupt range is set.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// // Custom configuration for the virtual machine.
    /// let mut vm_config = VirtualMachineConfig::default();
    /// vm_config.set_el2_enabled(true)?;
    /// // Custom configuration for the GIC.
    /// let mut gic_config = GicConfig::default();
    /// gic_config.set_redistributor_base(0x2000_0000)?;
    ///
    /// // Creates the global instance using the configurations above.
    /// let vm = VirtualMachineInstance::with_gic(vm_config, gic_config)?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "macos-15-0")]
    pub fn with_gic(
        vm_config: VirtualMachineConfig,
        gic_config: GicConfig,
    ) -> Result<VirtualMachineInstance<GicEnabled>> {
        // Creating the VM using `with_config()` is deliberate. This ensures that if
        // `hv_gic_create()` fails, vm will drop and the corresponding instance will be destroyed.
        // This would not be the case with a direct call to `hv_vm_create()`.
        let vm = Self::with_config(vm_config)?;
        hv_unsafe_call!(hv_gic_create(gic_config.0))?;
        Ok(VirtualMachineInstance::<GicEnabled> {
            _guard: vm._guard.clone(),
            _phantom: PhantomData,
        })
    }
}

/// Marks a virtual machine instance configured with a GIC, thus making GIC-related APIs available.
///
/// See [`VirtualMachineInstance<GicEnabled>`].
#[derive(Clone, Debug)]
#[cfg(feature = "macos-15-0")]
pub struct GicEnabled;

/// Marks a virtual machine configured without a GIC instance. Only the base API will be available
/// through these handles.
///
/// See [`VirtualMachineInstance<Gic>`].
#[derive(Clone, Debug)]
pub struct GicDisabled;

/// Represents the unique virtual machine instance of the current process.
///
/// This object can be safely shared among threads and guarantees the VM to exist as long as this
/// handle does.
#[derive(Clone, Debug)]
pub struct VirtualMachineInstance<Gic> {
    pub(crate) _guard: Option<Arc<()>>,
    _phantom: PhantomData<Gic>,
}

/// Destroys the virtual machine context of the current process.
impl<Gic> std::ops::Drop for VirtualMachineInstance<Gic> {
    fn drop(&mut self) {
        // Safe to unwrap here, this is the only place where we modify it directly.
        let guard = self._guard.take().unwrap();
        // If this call succeeds, we know it's the last Arc reference, no other VM instance exists
        // or can be created from this object at this point. We can safely destroy the VM.
        if Arc::into_inner(guard).is_some() {
            // WARN: fails silently if the VM instance could not be cleaned up.
            let _ = hv_unsafe_call!(hv_vm_destroy());
        }
    }
}

impl<Gic> VirtualMachineInstance<Gic> {
    /// Creates a new vCPU.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// # let vm = VirtualMachine::new()?;
    /// let vcpu = vm.vcpu_create()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn vcpu_create(&self) -> Result<Vcpu> {
        self.vcpu_with_config(VcpuConfig(ptr::null_mut()))
    }

    /// Creates a new vCPU with a user-provided config.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// # let vm = VirtualMachine::new()?;
    /// let vcpu_config = VcpuConfig::default();
    /// let vcpu = vm.vcpu_with_config(vcpu_config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn vcpu_with_config(&self, config: VcpuConfig) -> Result<Vcpu> {
        let mut vcpu = 0;
        let mut exit = ptr::null_mut() as *const hv_vcpu_exit_t;
        hv_unsafe_call!(hv_vcpu_create(&mut vcpu, &mut exit, config.0))?;
        Ok(Vcpu {
            vcpu,
            exit,
            _guard_self: Arc::new(()),
            // Safe to unwrap here, it is only empty when the VM object is dropped.
            _guard_vm: Arc::clone(self._guard.as_ref().unwrap()),
            _phantom: PhantomData,
        })
    }

    /// Stops all vCPUs corresponding to the [`VcpuHandle`]s of the `vcpu` input array.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    /// # use std::sync::mpsc;
    /// # use std::thread;
    ///
    /// # fn main() -> Result<()> {
    /// # let vm = VirtualMachine::new()?;
    /// let thread_count = 3;
    /// let (tx, rx) = mpsc::channel();
    ///
    /// thread::scope(|s| {
    ///     // Each thread will have a Vcpu looping indefinitely.
    ///     for i in 0..thread_count {
    ///         let vm_thread = vm.clone();
    ///         let tx_thread = tx.clone();
    ///         s.spawn(move || {
    ///             // Create the vCPU and memory region that will hold the infinite loop
    ///             // instruction.
    ///             let vcpu = vm_thread.vcpu_create().unwrap();
    ///
    ///             // Write the instruction that while loop indefinitely.
    ///             let mut mem = vm_thread.memory_create(PAGE_SIZE).unwrap();
    ///             let addr = (PAGE_SIZE * i) as u64;
    ///             mem.map(addr, MemPerms::ReadWriteExec).unwrap();
    ///             mem.write_u32(addr, 0x14000000).unwrap();
    ///
    ///             // Set PC to the loop address.
    ///             vcpu.set_reg(Reg::PC, 0x10000).unwrap();
    ///
    ///             // Sending the vCPU handle back to the main thread.
    ///             let handle = vcpu.get_handle();
    ///             tx_thread.send(handle).unwrap();
    ///
    ///             // Starting the VCPU, we should loop indefinitely here.
    ///             vcpu.run().unwrap();
    ///         });
    ///     }
    ///     // Wait for the vCPU handles from each thread.
    ///     let mut handles = vec![];
    ///     for _ in 0..thread_count {
    ///         handles.push(rx.recv().unwrap());
    ///     }
    ///     // Make the vCPU of each thread exit.
    ///     vm.vcpus_exit(&handles).unwrap();
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn vcpus_exit(&self, vcpus: &[VcpuHandle]) -> Result<()> {
        let mut guards = Vec::with_capacity(vcpus.len());
        let mut ids = Vec::with_capacity(vcpus.len());

        for vcpu in vcpus {
            // Hold strong refs to keep vCPUs alive during the call.
            if let Some(strong) = vcpu.take_ref() {
                ids.push(vcpu.id());
                guards.push(strong);
            }
        }

        hv_unsafe_call!(hv_vcpus_exit(ids.as_ptr(), ids.len() as u32))?;

        Ok(())
    }

    /// Creates a memory object.
    ///
    /// # Discussion
    ///
    /// The size provided must be aligned to [`PAGE_SIZE`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// let vm = VirtualMachine::new()?;
    /// let mem = vm.memory_create(PAGE_SIZE * 10)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn memory_create(&self, size: usize) -> Result<Memory> {
        let host_alloc = MemAlloc::new(size)?;
        Ok(Memory {
            host_alloc,
            guest_addr: None,
            // Safe to unwrap here, it is only empty when the VM object is dropped.
            _guard_vm: Arc::clone(self._guard.as_ref().unwrap()),
        })
    }
}

/// Transformes a `GicEnabled` instance into a `GicDisabled` one.
/// The underlying object still has a GIC instance, but related APIs can't be called.
#[cfg(feature = "macos-15-0")]
impl From<VirtualMachineInstance<GicEnabled>> for VirtualMachineInstance<GicDisabled> {
    fn from(value: VirtualMachineInstance<GicEnabled>) -> Self {
        VirtualMachineInstance::<GicDisabled> {
            _guard: value._guard.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Static global VM instance.
#[cfg(not(test))]
static _VM_INSTANCE: OnceLock<VirtualMachineStaticInstance> = OnceLock::new();

/// Static global VM instance, but mutable, for testing purposes only.
#[cfg(test)]
static mut _VM_INSTANCE: OnceLock<VirtualMachineStaticInstance> = OnceLock::new();

/// Global lock making sure only one thread performs [`_VM_INSTANCE`] initialization.
static _VM_INIT_LOCK: Mutex<()> = Mutex::new(());

/// Returns the pointer to the global VM instance.
macro_rules! vm_static_instance {
    () => {{
        #[cfg(test)]
        unsafe {
            // Get the raw pointer.
            let ptr: *const OnceLock<VirtualMachineStaticInstance> = &raw const _VM_INSTANCE;
            // Convert the raw pointer to a reference.
            &*ptr
        }
        #[cfg(not(test))]
        &_VM_INSTANCE
    }};
}

/// Resets the mutable global VM instance used for testing.
#[cfg(test)]
pub(crate) fn vm_static_instance_reset() {
    // Making sure there's no static instance when this test starts running.
    // SAFETY: as long as the test it's used in is marked as serial, no other threads has a
    // reference to the vm instance, we can therefore change it however we want.
    unsafe {
        let ptr = &mut *&raw mut _VM_INSTANCE;
        std::ptr::drop_in_place(ptr);
        std::ptr::write(
            ptr as *mut OnceLock<VirtualMachineStaticInstance>,
            OnceLock::new(),
        );
    }
}

/// Wrapper for the static global virtual machine instance.
///
/// Once this instance is set, it cannot be changed.
#[derive(Debug)]
pub enum VirtualMachineStaticInstance {
    /// Container for a virtual machine instance configured without a GIC.
    NoGic(VirtualMachineInstance<GicDisabled>),
    /// Container for a virtual machine instance configured with a GIC.
    #[cfg(feature = "macos-15-0")]
    Gic(VirtualMachineInstance<GicEnabled>),
}

impl VirtualMachineStaticInstance {
    /// Initializes the global vm instance with a [`VirtualMachine`] created with a default config.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// VirtualMachineStaticInstance::init()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn init() -> Result<()> {
        // If the instance has already been created, we can simply return.
        if vm_static_instance!().get().is_some() {
            return Ok(());
        }
        // Otherwise, we take the global lock to perform the initialization.
        let _guard = _VM_INIT_LOCK.lock().unwrap();
        // We make sure that another thread didn't perform the initialization while we waited for
        // the lock.
        if vm_static_instance!().get().is_some() {
            return Ok(());
        }
        let vm = VirtualMachine::new()?;
        // We can safely unwrap here, we have exclusive control of `_VM_INSTANCE` and we made sure
        // that it was uninitialized.
        vm_static_instance!()
            .set(VirtualMachineStaticInstance::NoGic(vm))
            .unwrap();
        Ok(())
    }

    /// Initializes the global vm instance with a [`VirtualMachine`] created with a custom config.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// // Custom configuration for the virtual machine.
    /// let mut config = VirtualMachineConfig::default();
    /// config.set_el2_enabled(true)?;
    ///
    /// // Creates the global instance using the configuration above.
    /// let _ = VirtualMachineStaticInstance::init_with_config(config)?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "macos-13-0")]
    pub fn init_with_config(vm_config: VirtualMachineConfig) -> Result<()> {
        // If the instance has already been created, we can simply return.
        if vm_static_instance!().get().is_some() {
            return Ok(());
        }
        // Otherwise, we take the global lock to perform the initialization.
        let _guard = _VM_INIT_LOCK.lock().unwrap();
        // We make sure that another thread didn't perform the initialization while we waited for
        // the lock.
        if vm_static_instance!().get().is_some() {
            return Ok(());
        }
        let vm = VirtualMachine::with_config(vm_config)?;
        // We can safely unwrap here, we have exclusive control of `_VM_INSTANCE` and we made sure
        // that it was uninitialized.
        vm_static_instance!()
            .set(VirtualMachineStaticInstance::NoGic(vm))
            .unwrap();
        Ok(())
    }

    /// Initializes the global vm instance with a [`VirtualMachine`] created with a custom config
    /// and a GIC.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # #[cfg(feature="macos-15-0")]
    /// # fn main() -> Result<()> {
    /// // Custom configuration for the virtual machine.
    /// let mut vm_config = VirtualMachineConfig::default();
    /// vm_config.set_el2_enabled(true)?;
    /// // Custom configuration for the GIC.
    /// let mut gic_config = GicConfig::default();
    /// gic_config.set_redistributor_base(0x2000_0000)?;
    ///
    /// // Creates the global instance using the configurations above.
    /// let _ = VirtualMachineStaticInstance::init_with_gic(vm_config, gic_config)?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "macos-15-0")]
    pub fn init_with_gic(vm_config: VirtualMachineConfig, gic_config: GicConfig) -> Result<()> {
        // If the instance has already been created, we can simply return.
        if vm_static_instance!().get().is_some() {
            return Ok(());
        }
        // Otherwise, we take the global lock to perform the initialization.
        let _guard = _VM_INIT_LOCK.lock().unwrap();
        // We make sure that another thread didn't perform the initialization while we waited for
        // the lock.
        if vm_static_instance!().get().is_some() {
            return Ok(());
        }
        let vm = VirtualMachine::with_gic(vm_config, gic_config)?;
        // We can safely unwrap here, we have exclusive control of `_VM_INSTANCE` and we made sure
        // that it was uninitialized.
        vm_static_instance!()
            .set(VirtualMachineStaticInstance::Gic(vm))
            .unwrap();
        Ok(())
    }

    /// Retrieves a [`GicDisabled`] handle to the current VM global instance, even if the current
    /// instance has a GIC configured.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # fn main() -> Result<()> {
    /// assert!(VirtualMachineStaticInstance::get().is_none());
    ///
    /// let _ = VirtualMachineStaticInstance::init()?;
    /// // The VM instance can be retrieved once it has been initialized.
    /// assert!(VirtualMachineStaticInstance::get().is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn get() -> Option<VirtualMachineInstance<GicDisabled>> {
        match vm_static_instance!().get() {
            Some(VirtualMachineStaticInstance::NoGic(vm)) => Some(vm.clone()),
            #[cfg(feature = "macos-15-0")]
            Some(VirtualMachineStaticInstance::Gic(vm)) => Some(Into::<
                VirtualMachineInstance<GicDisabled>,
            >::into(vm.clone())),
            _ => None,
        }
    }

    /// Retrieves a [`GicEnabled`] handle to the current VM global instance, only if the current
    /// instance has a GIC configured.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use applevisor::prelude::*;
    ///
    /// # #[cfg(feature = "macos-15-0")]
    /// # fn main() -> Result<()> {
    /// # let mut vm_config = VirtualMachineConfig::default();
    /// # vm_config.set_el2_enabled(true)?;
    /// # let mut gic_config = GicConfig::default();
    /// # gic_config.set_redistributor_base(0x2000_0000)?;
    /// // No instance has been created yet.
    /// assert!(VirtualMachineStaticInstance::get_gic().is_none());
    ///
    /// let _ = VirtualMachineStaticInstance::init_with_gic(vm_config, gic_config)?;
    /// // The VM instance can be retrieved once it has been initialized.
    /// assert!(VirtualMachineStaticInstance::get_gic().is_some());
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "macos-15-0")]
    pub fn get_gic() -> Option<VirtualMachineInstance<GicEnabled>> {
        match vm_static_instance!().get() {
            Some(VirtualMachineStaticInstance::Gic(vm)) => Some(vm.clone()),
            _ => None,
        }
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
    use crate::next_mem_addr;

    use super::*;

    #[cfg(feature = "macos-13-0")]
    #[test]
    #[parallel]
    fn config_ipa_size() {
        let mut config = VirtualMachineConfig::default();

        let max_ipa_size = VirtualMachineConfig::get_max_ipa_size();
        assert!(max_ipa_size.is_ok());
        let max_ipa_size = max_ipa_size.unwrap();

        let default_ipa_size = VirtualMachineConfig::get_default_ipa_size();
        assert!(default_ipa_size.is_ok());
        let default_ipa_size = default_ipa_size.unwrap();

        assert_eq!(config.set_ipa_size(0), Ok(()));
        assert_eq!(config.get_ipa_size(), Ok(0));
        assert_eq!(config.set_ipa_size(default_ipa_size - 1), Ok(()));
        assert_eq!(config.get_ipa_size(), Ok(default_ipa_size - 1));
        assert_eq!(config.set_ipa_size(default_ipa_size), Ok(()));
        assert_eq!(config.get_ipa_size(), Ok(default_ipa_size));
        assert_eq!(config.set_ipa_size(max_ipa_size - 1), Ok(()));
        assert_eq!(config.get_ipa_size(), Ok(max_ipa_size - 1));
        assert_eq!(config.set_ipa_size(max_ipa_size), Ok(()));
        assert_eq!(config.get_ipa_size(), Ok(max_ipa_size));
        assert_eq!(
            config.set_ipa_size(max_ipa_size + 1),
            Err(HypervisorError::Unsupported)
        );
        assert_eq!(
            config.set_ipa_size(u32::MAX),
            Err(HypervisorError::Unsupported)
        );
    }

    #[cfg(feature = "macos-15-0")]
    #[test]
    #[parallel]
    fn config_el2() {
        let mut config = VirtualMachineConfig::default();

        let el2_supported = VirtualMachineConfig::get_el2_supported();
        assert!(el2_supported.is_ok());

        assert_eq!(config.set_el2_enabled(false), Ok(()));
        assert_eq!(config.get_el2_enabled(), Ok(false));
        assert_eq!(config.set_el2_enabled(true), Ok(()));
        assert_eq!(config.get_el2_enabled(), Ok(true));
    }

    #[cfg(feature = "macos-15-2")]
    #[test]
    #[parallel]
    fn config_max_svl_bytes() {
        let max_svl_bytes_status = match VirtualMachineConfig::get_max_svl_bytes() {
            Ok(_) | Err(HypervisorError::Unsupported) => Ok(()),
            Err(x) => Err(x),
        };
        assert!(max_svl_bytes_status.is_ok())
    }

    #[cfg(feature = "macos-26-0")]
    #[test]
    #[parallel]
    fn config_ipa_granule() {
        let mut config = VirtualMachineConfig::default();

        let granule_size = VirtualMachineConfig::get_default_ipa_granule();
        assert!(granule_size.is_ok());

        assert_eq!(
            config.set_ipa_granule(IpaGranule::HV_IPA_GRANULE_4KB),
            Ok(())
        );
        assert_eq!(config.get_ipa_granule(), Ok(IpaGranule::HV_IPA_GRANULE_4KB));
        assert_eq!(
            config.set_ipa_granule(IpaGranule::HV_IPA_GRANULE_16KB),
            Ok(())
        );
        assert_eq!(
            config.get_ipa_granule(),
            Ok(IpaGranule::HV_IPA_GRANULE_16KB)
        );
    }

    #[test]
    #[serial]
    fn create_a_default_vm() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        let vm = VirtualMachine::new();
        assert!(vm.is_ok());
    }

    #[test]
    #[serial]
    fn create_a_default_static_vm_instance() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        let ret = VirtualMachineStaticInstance::init();
        assert!(ret.is_ok());
        let vm = VirtualMachineStaticInstance::get();
        assert!(vm.is_some());
    }

    #[cfg(feature = "macos-13-0")]
    #[test]
    #[serial]
    fn create_a_vm_with_a_custom_config() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        let mut config = VirtualMachineConfig::default();

        #[cfg(feature = "macos-15-0")]
        {
            if VirtualMachineConfig::get_el2_supported().unwrap() {
                config.set_el2_enabled(true).unwrap();
            }
        }

        let max_ipa_size = VirtualMachineConfig::get_max_ipa_size().unwrap();
        config.set_ipa_size(max_ipa_size).unwrap();

        let vm = VirtualMachine::with_config(config);
        assert!(vm.is_ok());
    }

    #[cfg(feature = "macos-13-0")]
    #[test]
    #[serial]
    fn create_a_static_vm_instance_with_a_custom_config() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        let mut config = VirtualMachineConfig::default();

        #[cfg(feature = "macos-15-0")]
        {
            if VirtualMachineConfig::get_el2_supported().unwrap() {
                config.set_el2_enabled(true).unwrap();
            }
        }

        let max_ipa_size = VirtualMachineConfig::get_max_ipa_size().unwrap();
        config.set_ipa_size(max_ipa_size).unwrap();

        let ret = VirtualMachineStaticInstance::init_with_config(config);
        assert!(ret.is_ok());
        let vm = VirtualMachineStaticInstance::get();
        assert!(vm.is_some());
    }

    #[cfg(feature = "macos-15-0")]
    #[test]
    #[serial]
    fn create_a_vm_with_a_gic() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        // A default GIC config will fail.
        {
            let vm_config = VirtualMachineConfig::default();
            let gic_config = GicConfig::new();
            let vm = VirtualMachine::with_gic(vm_config, gic_config);
            assert!(matches!(vm, Err(HypervisorError::BadArgument)));
        }

        // We need to at least provide the redistributor for the GIC to work.
        {
            let vm_config = VirtualMachineConfig::default();
            let mut gic_config = GicConfig::new();
            assert!(gic_config.set_redistributor_base(0x20000).is_ok());
            let vm = VirtualMachine::with_gic(vm_config, gic_config);
            assert!(vm.is_ok());
        }
    }

    #[cfg(feature = "macos-15-0")]
    #[test]
    #[serial]
    fn create_a_static_vm_instance_with_a_gic() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        let vm_config = VirtualMachineConfig::default();
        let mut gic_config = GicConfig::new();
        assert!(gic_config.set_redistributor_base(0x20000).is_ok());

        let ret = VirtualMachineStaticInstance::init_with_gic(vm_config, gic_config);
        assert!(ret.is_ok());
        let vm = VirtualMachineStaticInstance::get();
        assert!(vm.is_some());
    }

    #[test]
    #[serial]
    fn create_multiple_vm_instances_from_a_single_thread() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        {
            // Creating a first VM instance should work!
            let vm1 = VirtualMachine::new();
            assert!(vm1.is_ok());
            // Creating a second instance should fail.
            let vm2 = VirtualMachine::new();
            assert!(matches!(vm2, Err(HypervisorError::Busy)));
            // But cloning the first instance should work.
            let vm3 = vm1.clone();
            drop(vm1);
            assert!(vm3.is_ok());
            // And creating a new instance should still fail.
            let vm4 = VirtualMachine::new();
            assert!(matches!(vm4, Err(HypervisorError::Busy)));
            // Then, if we drop all VM instances created until now...
        }
        // ... creating a brand new instance should work.
        let vm5 = VirtualMachine::new();
        assert!(vm5.is_ok());
    }

    #[test]
    #[serial]
    fn use_one_vm_instance_from_multiple_threads() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        let barrier = Barrier::new(2);
        let vm = VirtualMachine::new().unwrap();

        thread::scope(|s| {
            // We create one thread holding a reference to the VM instance.
            let vm_thread = vm.clone();
            s.spawn(|| {
                let _ = vm_thread;
                barrier.wait();
            });

            // At this stage, we have two references, we drop one of them.
            drop(vm);

            // Our thread is still waiting on the barrier, so it should be holding the last
            // reference for now. Trying to create a new VM instance will thus fail.
            let vm2 = VirtualMachine::new();
            assert!(matches!(vm2, Err(HypervisorError::Busy)));

            // Our thread will now return, the last reference will drop, and the VM will be
            // destroyed.
            barrier.wait();
        });

        // Now we should be able to create a new VM instance.
        let vm3 = VirtualMachine::new();
        assert!(vm3.is_ok());
    }

    #[test]
    #[parallel]
    fn create_a_vcpu_from_a_vm_instance() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu = vm.vcpu_create();
        assert!(vcpu.is_ok());
    }

    #[test]
    #[parallel]
    fn create_a_vcpu_with_a_custom_config_from_a_vm_instance() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let vcpu_config = VcpuConfig::default();
        let vcpu = vm.vcpu_with_config(vcpu_config);
        assert!(vcpu.is_ok());
    }

    #[cfg(feature = "macos-15-0")]
    #[test]
    #[serial]
    fn downgrade_a_gicenabled_vm_instance_to_a_gicdisabled_vm_instance() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        let vm_config = VirtualMachineConfig::default();
        let mut gic_config = GicConfig::new();
        assert!(gic_config.set_redistributor_base(0x20000).is_ok());

        let vm = VirtualMachine::with_gic(vm_config, gic_config);
        assert!(vm.is_ok());
        let vm = vm.unwrap();
        let vm_no_gic = Into::<VirtualMachineInstance<GicDisabled>>::into(vm);
        let vcpu = vm_no_gic.vcpu_create();
        assert!(vcpu.is_ok());
    }

    #[test]
    #[parallel]
    fn exit_running_vcpus() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let thread_count = 4;
        let (tx, rx) = mpsc::channel();

        thread::scope(|s| {
            // Each thread will have a Vcpu looping indefinitely.
            for _ in 0..thread_count {
                let vm_thread = vm.clone();
                let tx_thread = tx.clone();
                s.spawn(move || {
                    // Create the vCPU and memory region that will hold the infinite loop
                    // instruction.
                    let vcpu = vm_thread.vcpu_create().unwrap();
                    let addr = next_mem_addr();
                    let mut mem = vm_thread.memory_create(PAGE_SIZE).unwrap();
                    mem.map(addr, MemPerms::ReadWriteExec).unwrap();
                    // \x00\x00\x00\x14 corresponds to `b .+0`.
                    mem.write_u32(addr, 0x14000000).unwrap();
                    vcpu.set_reg(Reg::PC, addr).unwrap();
                    // Sending the vCPU handle back to the main thread.
                    let handle = vcpu.get_handle();
                    tx_thread.send(handle).unwrap();
                    // Starting the VCPU, we should loop indefinitely here.
                    vcpu.run().unwrap();
                });
            }
            // Wait for the vCPU handles from each thread.
            let mut handles = vec![];
            for _ in 0..thread_count {
                handles.push(rx.recv().unwrap());
            }
            // Make the vCPU of each thread exit.
            vm.vcpus_exit(&handles).unwrap();
        });
    }

    #[test]
    #[serial]
    fn making_sure_vm_static_instances_behave_correctly() {
        // Making sure there's no static instance when this test starts running.
        vm_static_instance_reset();

        // Trying to get the instance when none exists returns None.
        assert!(VirtualMachineStaticInstance::get().is_none());
        #[cfg(feature = "macos-15-0")]
        assert!(VirtualMachineStaticInstance::get_gic().is_none());

        // After creating a new standard instance ...
        assert_eq!(VirtualMachineStaticInstance::init(), Ok(()));
        // ... we can get the standard one ...
        assert!(VirtualMachineStaticInstance::get().is_some());
        // ... but not a GIC-configured one.
        #[cfg(feature = "macos-15-0")]
        assert!(VirtualMachineStaticInstance::get_gic().is_none());

        #[cfg(feature = "macos-15-0")]
        {
            // Removing the static instance.
            vm_static_instance_reset();

            // After creating a new GIC instance ...
            let vm_config = VirtualMachineConfig::default();
            let mut gic_config = GicConfig::new();
            gic_config.set_redistributor_base(0x20000).unwrap();
            assert_eq!(
                VirtualMachineStaticInstance::init_with_gic(vm_config, gic_config),
                Ok(())
            );
            // ... we can get a standard one ...
            assert!(VirtualMachineStaticInstance::get().is_some());
            // ... as well as a GIC one.
            assert!(VirtualMachineStaticInstance::get_gic().is_some());
        }

        // Removing the static instance.
        vm_static_instance_reset();

        // It's ok if we try to recreate the instance when one already exists.
        assert_eq!(VirtualMachineStaticInstance::init(), Ok(()));
        assert_eq!(VirtualMachineStaticInstance::init(), Ok(()));

        #[cfg(feature = "macos-13-0")]
        {
            let vm_config = VirtualMachineConfig::default();
            assert_eq!(
                VirtualMachineStaticInstance::init_with_config(vm_config),
                Ok(())
            );
        }

        #[cfg(feature = "macos-15-0")]
        {
            let vm_config = VirtualMachineConfig::default();
            let mut gic_config = GicConfig::new();
            gic_config.set_redistributor_base(0x20000).unwrap();
            assert_eq!(
                VirtualMachineStaticInstance::init_with_gic(vm_config, gic_config),
                Ok(())
            );
        }
    }
}
