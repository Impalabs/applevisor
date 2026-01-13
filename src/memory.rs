//! Allocation and management of memory.

#[cfg(not(feature = "macos-12-1"))]
use std::alloc;

use core::ffi::c_void;
use std::hash::Hash;
use std::ptr;
use std::sync::Arc;

use applevisor_sys::*;

use crate::error::*;
use crate::hv_unsafe_call;

// -----------------------------------------------------------------------------------------------
// Memory Management
// -----------------------------------------------------------------------------------------------

/// Represents the access permissions of a memory range.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum MemPerms {
    /// No permssion.
    None,
    /// Read permission.
    Read,
    /// Write permission.
    Write,
    /// Execute permission.
    Exec,
    /// Read and write permissions.
    ReadWrite,
    /// Read and execute permissions.
    ReadExec,
    /// Write and execute permissions.
    WriteExec,
    /// Read, write and execute permissions.
    ReadWriteExec,
}

impl std::fmt::Display for MemPerms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let perms = match *self {
            MemPerms::None => "---",
            MemPerms::R => "r--",
            MemPerms::W => "-w-",
            MemPerms::X => "--x",
            MemPerms::RW => "rw-",
            MemPerms::RX => "r-x",
            MemPerms::WX => "-wx",
            MemPerms::RWX => "rwx",
        };
        write!(f, "{}", perms)
    }
}

impl std::ops::BitOr for MemPerms {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        (Into::<u64>::into(self) | Into::<u64>::into(rhs)).into()
    }
}

impl From<u64> for MemPerms {
    fn from(value: u64) -> Self {
        match value {
            x if x == HV_MEMORY_READ => Self::Read,
            x if x == HV_MEMORY_WRITE => Self::Write,
            x if x == HV_MEMORY_EXEC => Self::Exec,
            x if x == (HV_MEMORY_READ | HV_MEMORY_WRITE) => Self::ReadWrite,
            x if x == (HV_MEMORY_READ | HV_MEMORY_EXEC) => Self::ReadExec,
            x if x == (HV_MEMORY_WRITE | HV_MEMORY_EXEC) => Self::WriteExec,
            x if x == (HV_MEMORY_READ | HV_MEMORY_WRITE | HV_MEMORY_EXEC) => Self::ReadWriteExec,
            _ => Self::None,
        }
    }
}

impl From<MemPerms> for u64 {
    fn from(val: MemPerms) -> Self {
        match val {
            MemPerms::None => HV_MEMORY_NONE,
            MemPerms::R => HV_MEMORY_READ,
            MemPerms::W => HV_MEMORY_WRITE,
            MemPerms::X => HV_MEMORY_EXEC,
            MemPerms::RW => HV_MEMORY_READ | HV_MEMORY_WRITE,
            MemPerms::RX => HV_MEMORY_READ | HV_MEMORY_EXEC,
            MemPerms::WX => HV_MEMORY_WRITE | HV_MEMORY_EXEC,
            MemPerms::RWX => HV_MEMORY_READ | HV_MEMORY_WRITE | HV_MEMORY_EXEC,
        }
    }
}

/// Permissions aliases.
impl MemPerms {
    /// Read permission alias.
    pub const R: Self = Self::Read;
    /// Write permission alias.
    pub const W: Self = Self::Write;
    /// Execute permission alias.
    pub const X: Self = Self::Exec;
    /// Read and write permissions alias.
    pub const RW: Self = Self::ReadWrite;
    /// Read and execute permissions alias.
    pub const RX: Self = Self::ReadExec;
    /// Write and execute permissions alias.
    pub const WX: Self = Self::WriteExec;
    /// Read, write and execute permissions alias.
    pub const RWX: Self = Self::ReadWriteExec;
}

/// The size of a memory page on Apple Silicon.
pub const PAGE_SIZE: usize = applevisor_sys::PAGE_SIZE;

/// Represents a host memory allocation.
#[derive(Debug)]
pub(crate) struct MemAlloc {
    /// Host address.
    addr: *const c_void,
    /// Memory layout associated with `addr`.
    #[cfg(not(feature = "macos-12-1"))]
    layout: alloc::Layout,
    /// Allocation size.
    size: usize,
}

impl MemAlloc {
    /// Creates a new memory allocation for the host using [`hv_vm_allocate`].
    #[cfg(feature = "macos-12-1")]
    pub(crate) fn new(size: usize) -> Result<Self> {
        let mut addr = ptr::null_mut();
        // Rounding up the input size to the next PAGE_SIZE multiple, with overflow checks.
        let size = size
            .checked_add((PAGE_SIZE - (size % PAGE_SIZE)) % PAGE_SIZE)
            .ok_or(HypervisorError::BadArgument)?;
        hv_unsafe_call!(hv_vm_allocate(
            &mut addr,
            size,
            applevisor_sys::hv_allocate_flags_t::HV_ALLOCATE_DEFAULT
        ))?;
        Ok(Self { addr, size })
    }

    /// Creates a new memory allocation for the host using [`std::alloc`].
    #[cfg(not(feature = "macos-12-1"))]
    pub(crate) fn new(size: usize) -> Result<Self> {
        let layout = alloc::Layout::from_size_align(size, PAGE_SIZE)?.pad_to_align();
        let addr = unsafe { alloc::alloc_zeroed(layout) } as *const c_void;
        Ok(MemAlloc {
            addr,
            layout,
            size: layout.size(),
        })
    }
}

/// Deallocates memory mapping.
impl std::ops::Drop for MemAlloc {
    fn drop(&mut self) {
        #[cfg(feature = "macos-12-1")]
        // WARN: fails silently if the memory allocation could not be cleaned up.
        let _ = hv_unsafe_call!(hv_vm_deallocate(self.addr, self.size));
        #[cfg(not(feature = "macos-12-1"))]
        unsafe {
            alloc::dealloc(self.addr as *mut u8, self.layout);
        }
    }
}

/// Represents a memory mapping between a host-allocated memory range and its corresponding
/// mapping in the hypervisor guest.
#[derive(Debug)]
pub struct Memory {
    /// Host allocation object.
    pub(crate) host_alloc: MemAlloc,
    /// The address where the object is be mapped in the guest. Contains `None` if it is unmapped.
    pub(crate) guest_addr: Option<u64>,
    /// Strong reference to the virtual machine this memory allocation belongs to.
    pub(crate) _guard_vm: Arc<()>,
}

/// Deallocates memory mapping.
impl Drop for Memory {
    fn drop(&mut self) {
        let _ = self.unmap();
    }
}

impl Memory {
    /// Maps the host allocation in the guest.
    pub fn map(&mut self, guest_addr: u64, perms: MemPerms) -> Result<()> {
        // Return an error if the mapping is already mapped.
        if self.guest_addr.is_some() {
            return Err(HypervisorError::Busy);
        }
        // Map the mapping in the guest.
        hv_unsafe_call!(hv_vm_map(
            self.host_alloc.addr,
            guest_addr,
            self.host_alloc.size,
            perms as u64,
        ))?;
        // Update the mapping object.
        self.guest_addr = Some(guest_addr);
        Ok(())
    }

    /// Unmaps the host allocation from the guest.
    pub fn unmap(&mut self) -> Result<()> {
        // Return an error if we're trying to unmap an unmapped mapping.
        let guest_addr = self.guest_addr.take().ok_or(HypervisorError::Error)?;
        // Unmap the mapping from the guest.
        hv_unsafe_call!(hv_vm_unmap(guest_addr, self.host_alloc.size))?;
        Ok(())
    }

    /// Changes the protections of the memory mapping in the guest.
    pub fn protect(&mut self, perms: MemPerms) -> Result<()> {
        // Return an error if we're trying to modify an unmapped mapping permissions.
        let guest_addr = self.guest_addr.ok_or(HypervisorError::Error)?;
        // Changes the guest mapping's protections.
        hv_unsafe_call!(hv_vm_protect(
            guest_addr,
            self.host_alloc.size,
            perms as u64,
        ))?;
        Ok(())
    }

    /// Reads from a memory mapping in the guest at address `guest_addr`.
    pub fn read(&self, guest_addr: u64, data: &mut [u8]) -> Result<()> {
        // Return an error if we're trying to read from an unmapped mapping.
        let mapping_guest_addr = self.guest_addr.ok_or(HypervisorError::Error)?;
        // Checks the guest addr provided is in the guest memory range.
        let size = data.len();
        if guest_addr < mapping_guest_addr {
            return Err(HypervisorError::BadArgument);
        }
        if guest_addr
            .checked_add(size as u64)
            .ok_or(HypervisorError::BadArgument)?
            > mapping_guest_addr
                .checked_add(self.host_alloc.size as u64)
                .ok_or(HypervisorError::BadArgument)?
        {
            return Err(HypervisorError::BadArgument);
        }
        // Computes the corresponding host address.
        let offset = guest_addr - mapping_guest_addr;
        let host_addr = self.host_alloc.addr as u64 + offset;
        // Copies data from the memory mapping into the slice.
        unsafe {
            ptr::copy(
                host_addr as *const c_void,
                data.as_mut_ptr() as *mut c_void,
                size,
            );
        };
        Ok(())
    }

    /// Reads one byte at address `guest_addr`.
    pub fn read_u8(&self, guest_addr: u64) -> Result<u8> {
        let mut data = [0; 1];
        self.read(guest_addr, &mut data)?;
        Ok(data[0])
    }

    /// Reads one word at address `guest_addr`.
    pub fn read_u16(&self, guest_addr: u64) -> Result<u16> {
        let mut data = [0; 2];
        self.read(guest_addr, &mut data)?;
        Ok(u16::from_le_bytes(data))
    }

    /// Reads one dword at address `guest_addr`.
    pub fn read_u32(&self, guest_addr: u64) -> Result<u32> {
        let mut data = [0; 4];
        self.read(guest_addr, &mut data)?;
        Ok(u32::from_le_bytes(data))
    }

    /// Reads one qword at address `guest_addr`.
    pub fn read_u64(&self, guest_addr: u64) -> Result<u64> {
        let mut data = [0; 8];
        self.read(guest_addr, &mut data)?;
        Ok(u64::from_le_bytes(data))
    }

    /// Writes to a memory mapping in the guest at address `guest_addr`.
    pub fn write(&mut self, guest_addr: u64, data: &[u8]) -> Result<()> {
        let size = data.len();
        // Return an error if we're trying to write to an unmapped mapping.
        let mapping_guest_addr = self.guest_addr.ok_or(HypervisorError::Error)?;
        // Checks the guest addr provided is in the guest memory range.
        if guest_addr < mapping_guest_addr {
            return Err(HypervisorError::BadArgument);
        }
        if guest_addr
            .checked_add(size as u64)
            .ok_or(HypervisorError::BadArgument)?
            > mapping_guest_addr
                .checked_add(self.host_alloc.size as u64)
                .ok_or(HypervisorError::BadArgument)?
        {
            return Err(HypervisorError::BadArgument);
        }
        // Computes the corresponding host address.
        let offset = guest_addr - mapping_guest_addr;
        let host_addr = self.host_alloc.addr as u64 + offset;
        // Copies data from the input vector.
        unsafe {
            ptr::copy(
                data.as_ptr() as *const c_void,
                host_addr as *mut c_void,
                size,
            );
        };
        Ok(())
    }

    /// Writes one byte at address `guest_addr`.
    pub fn write_u8(&mut self, guest_addr: u64, data: u8) -> Result<()> {
        self.write(guest_addr, &[data])
    }

    /// Writes one word at address `guest_addr`.
    pub fn write_u16(&mut self, guest_addr: u64, data: u16) -> Result<()> {
        self.write(guest_addr, &data.to_le_bytes())
    }

    /// Writes one dword at address `guest_addr`.
    pub fn write_u32(&mut self, guest_addr: u64, data: u32) -> Result<()> {
        self.write(guest_addr, &data.to_le_bytes())
    }

    /// Writes one qword at address `guest_addr`.
    pub fn write_u64(&mut self, guest_addr: u64, data: u64) -> Result<()> {
        self.write(guest_addr, &data.to_le_bytes())
    }

    /// Returns the raw pointer to the memory mapping's host address.
    pub fn host_addr(&self) -> *mut u8 {
        self.host_alloc.addr as *mut u8
    }

    /// Returns the memory mapping's host address.
    pub fn guest_addr(&self) -> Option<u64> {
        self.guest_addr
    }

    /// Retrieves the memory mapping's size.
    pub fn size(&self) -> usize {
        self.host_alloc.size
    }
}

// -----------------------------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serial_test::*;

    use crate::{next_mem_addr, vm::*};

    use super::*;

    #[test]
    #[parallel]
    fn checking_memperms_operations_coherence() {
        assert_eq!(MemPerms::from(HV_MEMORY_NONE), MemPerms::None);
        assert_eq!(MemPerms::from(HV_MEMORY_READ), MemPerms::R);
        assert_eq!(MemPerms::from(HV_MEMORY_WRITE), MemPerms::W);
        assert_eq!(MemPerms::from(HV_MEMORY_EXEC), MemPerms::X);

        assert_eq!(MemPerms::R | MemPerms::W, MemPerms::RW);
        assert_eq!(MemPerms::R | MemPerms::X, MemPerms::RX);
        assert_eq!(MemPerms::R | MemPerms::WX, MemPerms::RWX);
        assert_eq!(MemPerms::W | MemPerms::X, MemPerms::WX);
        assert_eq!(MemPerms::W | MemPerms::RX, MemPerms::RWX);
        assert_eq!(MemPerms::X | MemPerms::RW, MemPerms::RWX);
        assert_eq!(MemPerms::RWX | MemPerms::None, MemPerms::RWX);

        assert_eq!(format!("{}", MemPerms::None), "---");
        assert_eq!(format!("{}", MemPerms::R), "r--");
        assert_eq!(format!("{}", MemPerms::W), "-w-");
        assert_eq!(format!("{}", MemPerms::X), "--x");
        assert_eq!(format!("{}", MemPerms::RW), "rw-");
        assert_eq!(format!("{}", MemPerms::RX), "r-x");
        assert_eq!(format!("{}", MemPerms::WX), "-wx");
        assert_eq!(format!("{}", MemPerms::RWX), "rwx");
    }

    #[test]
    #[parallel]
    fn making_an_allocation_with_a_size_overflowing_when_aligned() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        #[cfg(feature = "macos-12-1")]
        assert!(matches!(
            vm.memory_create(0xffff_ffff_ffff_fabc),
            Err(HypervisorError::BadArgument)
        ));

        #[cfg(not(feature = "macos-12-1"))]
        assert!(matches!(
            vm.memory_create(0xffff_ffff_ffff_fabc),
            Err(HypervisorError::LayoutError)
        ));
    }

    #[test]
    #[parallel]
    fn basic_operations_on_a_memory_mapping() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        let mut mem = vm.memory_create(PAGE_SIZE).unwrap();
        assert_eq!(mem.size(), PAGE_SIZE);

        let addr = next_mem_addr();

        // Trying to perform operations while the page is unmapped.
        assert_eq!(mem.guest_addr(), None);
        assert!(matches!(
            mem.protect(MemPerms::None),
            Err(HypervisorError::Error)
        ));
        assert!(matches!(
            mem.read(addr, &mut vec![1]),
            Err(HypervisorError::Error)
        ));
        assert!(matches!(
            mem.write(addr, &vec![1]),
            Err(HypervisorError::Error)
        ));

        // Mapping the page in the virtual machine.
        mem.map(addr, MemPerms::ReadWriteExec).unwrap();
        assert_eq!(mem.guest_addr(), Some(addr));

        // Remapping a memory object returns an error.
        assert!(matches!(
            mem.map(addr, MemPerms::ReadWriteExec),
            Err(HypervisorError::Busy)
        ));

        mem.protect(MemPerms::Read).unwrap();

        // Unmapping twice results in an error.
        mem.unmap().unwrap();
        assert!(matches!(mem.unmap(), Err(HypervisorError::Error)));
    }

    #[test]
    #[parallel]
    fn accessing_memory_through_raw_pointers() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        // Reading
        let addr = next_mem_addr();
        let value = 0xdeadbeefcafec0c0;
        let mut mem = vm.memory_create(PAGE_SIZE).unwrap();
        mem.map(addr, MemPerms::ReadWriteExec).unwrap();
        mem.write_u64(addr + 0x1238, value).unwrap();
        unsafe {
            let base_ptr = mem.host_addr();
            let value_ptr = base_ptr.add(0x1238) as *const u64;
            assert_eq!(*value_ptr, value);
        }

        // Writing
        let addr = next_mem_addr();
        let value = 0xdeadbeefcafec0c0;
        let mut mem = vm.memory_create(PAGE_SIZE).unwrap();
        mem.map(addr, MemPerms::ReadWriteExec).unwrap();
        unsafe {
            let base_ptr = mem.host_addr();
            let value_ptr = base_ptr.add(0x2348) as *mut u64;
            *value_ptr = value;
        }
        assert_eq!(mem.read_u64(addr + 0x2348), Ok(value));
    }

    #[test]
    #[parallel]
    fn reading_writing_memory_out_of_bounds() {
        let _ = VirtualMachineStaticInstance::init();
        let vm = VirtualMachineStaticInstance::get().unwrap();

        // Mapping our page.
        let addr = next_mem_addr();
        let mut mem = vm.memory_create(PAGE_SIZE).unwrap();
        mem.map(addr, MemPerms::ReadWriteExec).unwrap();

        let mut data = vec![0; 0x10];

        // Reading one byte before the buffer.
        let read_addr = addr - 1;
        assert_eq!(
            mem.read(read_addr, &mut data),
            Err(HypervisorError::BadArgument)
        );
        // Reading one byte after the buffer.
        let read_addr = addr + PAGE_SIZE as u64 - data.len() as u64 + 1;
        assert_eq!(
            mem.read(read_addr, &mut data),
            Err(HypervisorError::BadArgument)
        );
        // Reading from an address that would overflow.
        let read_addr = u64::MAX - data.len() as u64 + 1;
        assert_eq!(
            mem.read(read_addr, &mut data),
            Err(HypervisorError::BadArgument)
        );

        // Writing one byte before the buffer.
        let write_addr = addr - 1;
        assert_eq!(
            mem.write(write_addr, &data),
            Err(HypervisorError::BadArgument)
        );
        // Writing one byte after the buffer.
        let write_addr = addr + PAGE_SIZE as u64 - data.len() as u64 + 1;
        assert_eq!(
            mem.write(write_addr, &data),
            Err(HypervisorError::BadArgument)
        );
        // Writing to an address that would overflow.
        let write_addr = u64::MAX - data.len() as u64 + 1;
        assert_eq!(
            mem.write(write_addr, &data),
            Err(HypervisorError::BadArgument)
        );
    }

    macro_rules! reading_writing_memory_macro {
        ($($name:ident: ($type:ident, $read_fn:ident, $write_fn:ident),)*) => {
            $(
                #[test]
                #[parallel]
                fn $name() {
                    let _ = VirtualMachineStaticInstance::init();
                    let vm = VirtualMachineStaticInstance::get().unwrap();

                    // Mapping a page to write to.
                    let addr = next_mem_addr();
                    let mut mem = vm.memory_create(PAGE_SIZE).unwrap();
                    mem.map(addr, MemPerms::ReadWrite).unwrap();

                    let count = 0x400;
                    let step = PAGE_SIZE / count;
                    let type_size = std::mem::size_of::<$type>();
                    let mut data: Vec<$type> = vec![0; PAGE_SIZE / type_size];

                    // Reading and writing 8-bit values.
                    for i in 0..count {
                        let value = i.wrapping_mul(0x1111_1111_1111_1111) as $type;
                        let index = step * i;
                        let write_addr = addr + index as u64;
                        assert_eq!(mem.$write_fn(write_addr, value), Ok(()));
                        data[index / type_size] = value;
                    }

                    // Checking that the underlying buffer is the same as our reference buffer.
                    assert_eq!(&data, unsafe {
                        std::slice::from_raw_parts(mem.host_addr() as *const $type, PAGE_SIZE / type_size)
                    });

                    // Mapping a page to read from.
                    let addr = next_mem_addr();
                    let mut mem = vm.memory_create(PAGE_SIZE).unwrap();
                    mem.map(addr, MemPerms::Read).unwrap();

                    // Writing our previous buffer to the new page.
                    unsafe {
                        std::ptr::copy(data.as_ptr(), mem.host_addr() as *mut $type, PAGE_SIZE / type_size);
                    }

                    // Reading and writing 8-bit values.
                    for i in 0..count {
                        let value = i.wrapping_mul(0x1111_1111_1111_1111) as $type;
                        let index = step * i;
                        let read_addr = addr + index as u64;
                        assert_eq!(mem.$read_fn(read_addr), Ok(value));
                    }
                }
            )*
        }
    }

    reading_writing_memory_macro!(
        reading_writing_memory_u8: (u8, read_u8, write_u8),
        reading_writing_memory_u16: (u16, read_u16, write_u16),
        reading_writing_memory_u32: (u32, read_u32, write_u32),
        reading_writing_memory_u64: (u64, read_u64, write_u64),
    );
}
