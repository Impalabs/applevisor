//! Rust bindings for the Apple Silicon Hypervisor.framework
//!
//! This library can be used to build Rust applications leveraging the
//! [`Hypervisor`](https://developer.apple.com/documentation/hypervisor) framework on
//! Apple Silicon.
//!
//! ### Self-Signed Binaries and Hypervisor Entitlement
//!
//! To be able to reach the Hypervisor Framework, a binary executable has to have been granted the
//! [hypervisor entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com_apple_security_hypervisor).
//!
//! You can add this entitlement to a binary located at `/path/to/binary` by using the
//! `entitlements.xml` file found at the root of the repository and the following command:
//!
//! ```
//! codesign --sign - --entitlements entitlements.xml --deep --force /path/to/binary
//! ```
//!
//! ### Compilation Workflow
//!
//! Create a Rust project and add Applevisor as a dependency in `Cargo.toml`. You can either pull
//! it from [crates.io](https://crates.io/crates/applevisor) ...
//!
//! ```toml
//! # Check which version is the latest, this part of the README might not be updated
//! # in future releases.
//! applevisor = "0.1.1"
//! ```
//!
//! ... or directly from the GitHub repository.
//!
//! ```toml
//! applevisor = { git="https://github.com/impalabs/applevisor", branch="master" }
//! ```
//!
//! Create a file called `entitlements.txt` in the project's root directory and add the following:
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
//! <plist version="1.0">
//! <dict>
//!     <key>com.apple.security.hypervisor</key>
//!     <true/>
//! </dict>
//! </plist>
//! ```
//!
//! Write code and then build the project.
//!
//! ```
//! cargo build --release
//! ```
//!
//! Sign the binary and grant the hypervisor entitlement.
//!
//! ```
//! codesign --sign - --entitlements entitlements.xml --deep --force target/release/${PROJECT_NAME}
//! ```
//!
//! Run the binary.
//!
//! ```
//! target/release/${PROJECT_NAME}
//! ```
//!
//! ### Example
//!
//! The following example:
//!
//!  * creates a virtual machine for the current process;
//!  * creates a virtual CPU;
//!  * enables the hypervisor's debug features to be able to use breakpoints later on;
//!  * creates a physical memory mapping of 0x1000 bytes and maps it at address 0x4000 with RWX
//!    permissions;
//!  * writes the instructions `mov x0, #0x42; brk #0;` at address 0x4000;
//!  * sets PC to 0x4000;
//!  * starts the vCPU and runs our program;
//!  * returns when it encounters the breakpoint.
//!
//! ```no_run
//! use applevisor::*;
//!
//! fn main() {
//!     // Creates a new virtual machine. There can be one, and only one, per process. Operations
//!     // on the virtual machine remains possible as long as this object is valid.
//!     let _vm = VirtualMachine::new().unwrap();
//!
//!     // Creates a new virtual CPU. This object abstracts operations that can be performed on
//!     // CPUs, such as starting and stopping them, changing their registers, etc.
//!     let vcpu = Vcpu::new().unwrap();
//!
//!     // Enables debug features for the hypervisor. This is optional, but it might be required
//!     // for certain features to work, such as breakpoints.
//!     assert!(vcpu.set_trap_debug_exceptions(true).is_ok());
//!     assert!(vcpu.set_trap_debug_reg_accesses(true).is_ok());
//!
//!     // Creates a mapping object that represents a 0x1000-byte physical memory range.
//!     let mut mem = Mapping::new(0x1000).unwrap();
//!
//!     // This mapping needs to be mapped to effectively allocate physical memory for the guest.
//!     // Here we map the region at address 0x4000 and set the permissions to Read-Write-Execute.
//!     // Note that physical memory page sizes on Apple Silicon are 0x4000-aligned, you might
//!     // encounter errors if you don't respect the alignment.
//!     assert_eq!(mem.map(0x4000, MemPerms::RWX), Ok(()));
//!
//!     // Writes a `mov x0, #0x42` instruction at address 0x4000.
//!     assert_eq!(mem.write_dword(0x4000, 0xd2800840), Ok(4));
//!     // Writes a `brk #0` instruction at address 0x4004.
//!     assert_eq!(mem.write_dword(0x4004, 0xd4200000), Ok(4));
//!
//!     // Sets PC to 0x4000.
//!     assert!(vcpu.set_reg(Reg::PC, 0x4000).is_ok());
//!
//!     // Starts the Vcpu. It will execute our mov and breakpoint instructions before stopping.
//!     assert!(vcpu.run().is_ok());
//!
//!     // The *exit information* can be used to used to retrieve different pieces of information
//!     // about the CPU exit status (e.g. exception type, fault address, etc.).
//!     let _exit_info = vcpu.get_exit_info();
//!
//!     // If everything went as expected, the value in X0 is 0x42.
//!     assert_eq!(vcpu.get_reg(Reg::X0), Ok(0x42));
//! }
//! ```
//!
//! Feel free to also have a look at the [Hyperpom](https://github.com/impalabs/hyperpom)
//! project's source code for a real-life example of how these bindings are used.

#![cfg_attr(feature = "simd_nightly", feature(portable_simd), feature(simd_ffi), feature(concat_idents))]

use core::ffi::c_void;
use core::ptr;
use std::alloc;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

#[cfg(feature = "simd_nightly")]
use std::simd;

#[cfg(not(feature = "simd_nightly"))]
use concat_idents::concat_idents;

use applevisor_sys::hv_cache_type_t::*;
use applevisor_sys::hv_exit_reason_t::*;
use applevisor_sys::hv_feature_reg_t::*;
use applevisor_sys::hv_interrupt_type_t::*;
use applevisor_sys::hv_reg_t::*;
use applevisor_sys::hv_simd_fp_reg_t::*;
use applevisor_sys::hv_sys_reg_t::*;
use applevisor_sys::*;

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

/// Macro that generates an enum `$dst` corresponding to the raw C enum `$src`.
/// Also generates the [`Into`] trait implementation that converts a `$dst` variant into the
/// corresponding `$src`.
macro_rules! gen_enum {
    (
        $(#[$cmt:meta])* $dst: ident,
        $src: ident,
        $prefix:ident,
        $(#[$var_cmt:meta] $variant: ident,)*
    ) => {
        $(#[$cmt])*
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub enum $dst {
            $(
                #[$var_cmt]
                $variant,
            )*
        }

        #[cfg(feature = "simd_nightly")]
        #[allow(clippy::from_over_into)]
        impl Into<$src> for $dst {
            fn into(self) -> $src {
                match self {
                    $($dst::$variant => concat_idents!($prefix, $variant),)*
                }
            }
        }

        #[cfg(not(feature = "simd_nightly"))]
        #[allow(clippy::from_over_into)]
        impl Into<$src> for $dst {
            fn into(self) -> $src {
                match self {
                    $($dst::$variant => concat_idents!(x = $prefix, $variant { x }),)*
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------------------------

gen_enum!(
    /// The type that defines feature registers.
    FeatureReg,
    hv_feature_reg_t,
    HV_FEATURE_REG_,
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
);

gen_enum!(
    /// The structure that describes an instruction or data cache element.
    CacheType,
    hv_cache_type_t,
    HV_CACHE_TYPE_,
    /// The value that describes a cached data value.
    DATA,
    /// The value that describes a cached instuction value.
    INSTRUCTION,
);

gen_enum!(
    /// The type that describes the event that triggered a guest exit to the host.
    ExitReason,
    hv_exit_reason_t,
    HV_EXIT_REASON_,
    /// The value that identifies exits requested by exit handler on the host.
    CANCELED,
    /// The value that identifies traps caused by the guest operations.
    EXCEPTION,
    /// The value that identifies when the virtual timer enters the pending state.
    VTIMER_ACTIVATED,
    /// The value that identifies unexpected exits.
    UNKNOWN,
);

impl From<hv_exit_reason_t> for ExitReason {
    fn from(src: hv_exit_reason_t) -> Self {
        match src {
            HV_EXIT_REASON_CANCELED => ExitReason::CANCELED,
            HV_EXIT_REASON_EXCEPTION => ExitReason::EXCEPTION,
            HV_EXIT_REASON_VTIMER_ACTIVATED => ExitReason::VTIMER_ACTIVATED,
            HV_EXIT_REASON_UNKNOWN => ExitReason::UNKNOWN,
        }
    }
}

gen_enum!(
    /// The type that defines the vCPU’s interrupts.
    InterruptType,
    hv_interrupt_type_t,
    HV_INTERRUPT_TYPE_,
    /// ARM Fast Interrupt Request.
    FIQ,
    /// ARM Interrupt Request.
    IRQ,
);

gen_enum!(
    /// The type that defines general registers.
    Reg,
    hv_reg_t,
    HV_REG_,
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
);

impl Reg {
    /// The value that identifies the frame pointer (FP).
    pub const FP: Self = Self::X29;
    /// The value that identifies the link register (LR).
    pub const LR: Self = Self::X30;
}

gen_enum!(
    /// The type that defines SIMD and floating-point registers.
    SimdFpReg,
    hv_simd_fp_reg_t,
    HV_SIMD_FP_REG_,
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
);

gen_enum!(
    /// The type of system registers.
    SysReg,
    hv_sys_reg_t,
    HV_SYS_REG_,
    /// The value that represents the system register DBGBVR0_EL1.
    DBGBVR0_EL1,
    /// The value that represents the system register DBGBCR0_EL1.
    DBGBCR0_EL1,
    /// The value that represents the system register DBGWVR0_EL1.
    DBGWVR0_EL1,
    /// The value that represents the system register DBGWCR0_EL1.
    DBGWCR0_EL1,
    /// The value that represents the system register DBGBVR1_EL1.
    DBGBVR1_EL1,
    /// The value that represents the system register DBGBCR1_EL1.
    DBGBCR1_EL1,
    /// The value that represents the system register DBGWVR1_EL1.
    DBGWVR1_EL1,
    /// The value that represents the system register DBGWCR1_EL1.
    DBGWCR1_EL1,
    /// The value that represents the system register MDCCINT_EL1.
    MDCCINT_EL1,
    /// The value that represents the system register MDSCR_EL1.
    MDSCR_EL1,
    /// The value that represents the system register DBGBVR2_EL1.
    DBGBVR2_EL1,
    /// The value that represents the system register DBGBCR2_EL1.
    DBGBCR2_EL1,
    /// The value that represents the system register DBGWVR2_EL1.
    DBGWVR2_EL1,
    /// The value that represents the system register DBGWCR2_EL1.
    DBGWCR2_EL1,
    /// The value that represents the system register DBGBVR3_EL1.
    DBGBVR3_EL1,
    /// The value that represents the system register DBGBCR3_EL1.
    DBGBCR3_EL1,
    /// The value that represents the system register DBGWVR3_EL1.
    DBGWVR3_EL1,
    /// The value that represents the system register DBGWCR3_EL1.
    DBGWCR3_EL1,
    /// The value that represents the system register DBGBVR4_EL1.
    DBGBVR4_EL1,
    /// The value that represents the system register DBGBCR4_EL1.
    DBGBCR4_EL1,
    /// The value that represents the system register DBGWVR4_EL1.
    DBGWVR4_EL1,
    /// The value that represents the system register DBGWCR4_EL1.
    DBGWCR4_EL1,
    /// The value that represents the system register DBGBVR5_EL1.
    DBGBVR5_EL1,
    /// The value that represents the system register DBGBCR5_EL1.
    DBGBCR5_EL1,
    /// The value that represents the system register DBGWVR5_EL1.
    DBGWVR5_EL1,
    /// The value that represents the system register DBGWCR5_EL1.
    DBGWCR5_EL1,
    /// The value that represents the system register DBGBVR6_EL1.
    DBGBVR6_EL1,
    /// The value that represents the system register DBGBCR6_EL1.
    DBGBCR6_EL1,
    /// The value that represents the system register DBGWVR6_EL1.
    DBGWVR6_EL1,
    /// The value that represents the system register DBGWCR6_EL1.
    DBGWCR6_EL1,
    /// The value that represents the system register DBGBVR7_EL1.
    DBGBVR7_EL1,
    /// The value that represents the system register DBGBCR7_EL1.
    DBGBCR7_EL1,
    /// The value that represents the system register DBGWVR7_EL1.
    DBGWVR7_EL1,
    /// The value that represents the system register DBGWCR7_EL1.
    DBGWCR7_EL1,
    /// The value that represents the system register DBGBVR8_EL1.
    DBGBVR8_EL1,
    /// The value that represents the system register DBGBCR8_EL1.
    DBGBCR8_EL1,
    /// The value that represents the system register DBGWVR8_EL1.
    DBGWVR8_EL1,
    /// The value that represents the system register DBGWCR8_EL1.
    DBGWCR8_EL1,
    /// The value that represents the system register DBGBVR9_EL1.
    DBGBVR9_EL1,
    /// The value that represents the system register DBGBCR9_EL1.
    DBGBCR9_EL1,
    /// The value that represents the system register DBGWVR9_EL1.
    DBGWVR9_EL1,
    /// The value that represents the system register DBGWCR9_EL1.
    DBGWCR9_EL1,
    /// The value that represents the system register DBGBVR10_EL1.
    DBGBVR10_EL1,
    /// The value that represents the system register DBGBCR10_EL1.
    DBGBCR10_EL1,
    /// The value that represents the system register DBGWVR10_EL1.
    DBGWVR10_EL1,
    /// The value that represents the system register DBGWCR10_EL1.
    DBGWCR10_EL1,
    /// The value that represents the system register DBGBVR11_EL1.
    DBGBVR11_EL1,
    /// The value that represents the system register DBGBCR11_EL1.
    DBGBCR11_EL1,
    /// The value that represents the system register DBGWVR11_EL1.
    DBGWVR11_EL1,
    /// The value that represents the system register DBGWCR11_EL1.
    DBGWCR11_EL1,
    /// The value that represents the system register DBGBVR12_EL1.
    DBGBVR12_EL1,
    /// The value that represents the system register DBGBCR12_EL1.
    DBGBCR12_EL1,
    /// The value that represents the system register DBGWVR12_EL1.
    DBGWVR12_EL1,
    /// The value that represents the system register DBGWCR12_EL1.
    DBGWCR12_EL1,
    /// The value that represents the system register DBGBVR13_EL1.
    DBGBVR13_EL1,
    /// The value that represents the system register DBGBCR13_EL1.
    DBGBCR13_EL1,
    /// The value that represents the system register DBGWVR13_EL1.
    DBGWVR13_EL1,
    /// The value that represents the system register DBGWCR13_EL1.
    DBGWCR13_EL1,
    /// The value that represents the system register DBGBVR14_EL1.
    DBGBVR14_EL1,
    /// The value that represents the system register DBGBCR14_EL1.
    DBGBCR14_EL1,
    /// The value that represents the system register DBGWVR14_EL1.
    DBGWVR14_EL1,
    /// The value that represents the system register DBGWCR14_EL1.
    DBGWCR14_EL1,
    /// The value that represents the system register DBGBVR15_EL1.
    DBGBVR15_EL1,
    /// The value that represents the system register DBGBCR15_EL1.
    DBGBCR15_EL1,
    /// The value that represents the system register DBGWVR15_EL1.
    DBGWVR15_EL1,
    /// The value that represents the system register DBGWCR15_EL1.
    DBGWCR15_EL1,
    /// The value that represents the system register MIDR_EL1.
    MIDR_EL1,
    /// The value that represents the system register MPIDR_EL1.
    MPIDR_EL1,
    /// The value that describes the AArch64 Processor Feature Register 0.
    ID_AA64PFR0_EL1,
    /// The value that describes the AArch64 Processor Feature Register 1.
    ID_AA64PFR1_EL1,
    /// The value that describes the AArch64 Debug Feature Register 0.
    ID_AA64DFR0_EL1,
    /// The value that describes the AArch64 Debug Feature Register 1.
    ID_AA64DFR1_EL1,
    /// The value that describes the AArch64 Instruction Set Attribute Register 0.
    ID_AA64ISAR0_EL1,
    /// The value that describes the AArch64 Instruction Set Attribute Register 1.
    ID_AA64ISAR1_EL1,
    /// The value that describes the AArch64 Memory Model Feature Register 0.
    ID_AA64MMFR0_EL1,
    /// The value that describes the AArch64 Memory Model Feature Register 1.
    ID_AA64MMFR1_EL1,
    /// The value that describes the AArch64 Memory Model Feature Register 2.
    ID_AA64MMFR2_EL1,
    /// The value that represents the system register SCTLR_EL1.
    SCTLR_EL1,
    /// The value that represents the system register CPACR_EL1.
    CPACR_EL1,
    /// The value that represents the system register TTBR0_EL1.
    TTBR0_EL1,
    /// The value that represents the system register TTBR1_EL1.
    TTBR1_EL1,
    /// The value that represents the system register TCR_EL1.
    TCR_EL1,
    /// The value that represents the system register APIAKEYLO_EL1.
    APIAKEYLO_EL1,
    /// The value that represents the system register APIAKEYHI_EL1.
    APIAKEYHI_EL1,
    /// The value that represents the system register APIBKEYLO_EL1.
    APIBKEYLO_EL1,
    /// The value that represents the system register APIBKEYHI_EL1.
    APIBKEYHI_EL1,
    /// The value that represents the system register APDAKEYLO_EL1.
    APDAKEYLO_EL1,
    /// The value that represents the system register APDAKEYHI_EL1.
    APDAKEYHI_EL1,
    /// The value that represents the system register APDBKEYLO_EL1.
    APDBKEYLO_EL1,
    /// The value that represents the system register APDBKEYHI_EL1.
    APDBKEYHI_EL1,
    /// The value that represents the system register APGAKEYLO_EL1.
    APGAKEYLO_EL1,
    /// The value that represents the system register APGAKEYHI_EL1.
    APGAKEYHI_EL1,
    /// The value that represents the system register SPSR_EL1.
    SPSR_EL1,
    /// The value that represents the system register ELR_EL1.
    ELR_EL1,
    /// The value that represents the system register SP_EL0.
    SP_EL0,
    /// The value that represents the system register AFSR0_EL1.
    AFSR0_EL1,
    /// The value that represents the system register AFSR1_EL1.
    AFSR1_EL1,
    /// The value that represents the system register ESR_EL1.
    ESR_EL1,
    /// The value that represents the system register FAR_EL1.
    FAR_EL1,
    /// The value that represents the system register PAR_EL1.
    PAR_EL1,
    /// The value that represents the system register MAIR_EL1.
    MAIR_EL1,
    /// The value that represents the system register AMAIR_EL1.
    AMAIR_EL1,
    /// The value that represents the system register VBAR_EL1.
    VBAR_EL1,
    /// The value that represents the system register CONTEXTIDR_EL1.
    CONTEXTIDR_EL1,
    /// The value that represents the system register TPIDR_EL1.
    TPIDR_EL1,
    /// The value that represents the system register CNTKCTL_EL1.
    CNTKCTL_EL1,
    /// The value that represents the system register CSSELR_EL1.
    CSSELR_EL1,
    /// The value that represents the system register TPIDR_EL0.
    TPIDR_EL0,
    /// The value that represents the system register TPIDRRO_EL0.
    TPIDRRO_EL0,
    /// The value that represents the system register CNTV_CTL_EL0.
    CNTV_CTL_EL0,
    /// The value that represents the system register CNTV_CVAL_EL0.
    CNTV_CVAL_EL0,
    /// The value that represents the system register SP_EL1.
    SP_EL1,
);

// -----------------------------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------------------------

/// Convenient Result type for hypervisor errors.
pub type Result<T> = core::result::Result<T, HypervisorError>;

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

#[allow(clippy::from_over_into)]
impl Into<hv_return_t> for HypervisorError {
    fn into(self) -> hv_return_t {
        match self {
            Self::BadArgument => hv_error_t::HV_BAD_ARGUMENT as hv_return_t,
            Self::Busy => hv_error_t::HV_BUSY as hv_return_t,
            Self::Denied => hv_error_t::HV_DENIED as hv_return_t,
            Self::Error => hv_error_t::HV_ERROR as hv_return_t,
            Self::Fault => hv_error_t::HV_FAULT as hv_return_t,
            Self::IllegalState => hv_error_t::HV_ILLEGAL_GUEST_STATE as hv_return_t,
            Self::NoDevice => hv_error_t::HV_NO_DEVICE as hv_return_t,
            Self::NoResources => hv_error_t::HV_NO_RESOURCES as hv_return_t,
            Self::Unsupported => hv_error_t::HV_UNSUPPORTED as hv_return_t,
            Self::Unknown(code) => code,
        }
    }
}

impl std::error::Error for HypervisorError {}

impl core::fmt::Display for HypervisorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} (error {:#08x})",
            self.as_str(),
            Into::<hv_return_t>::into(*self)
        )
    }
}

impl core::fmt::Debug for HypervisorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HypervisorError")
            .field("code", &Into::<hv_return_t>::into(*self))
            .field("description", &self.as_str())
            .finish()
    }
}

// -----------------------------------------------------------------------------------------------
// Virtual Machine
// -----------------------------------------------------------------------------------------------

unsafe impl Sync for VirtualMachine {}

/// Represents the unique virtual machine instance of the current process.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct VirtualMachine {
    /// The virtual machine configuration.
    config: hv_vm_config_t,
}

impl VirtualMachine {
    /// Creates a new virtual machine instance for the current process.
    pub fn new() -> Result<Self> {
        let config = ptr::null_mut();
        hv_unsafe_call!(hv_vm_create(config))?;
        Ok(Self { config })
    }
}

/// Destroys the virtual machine context of the current process.
///
/// Panics if it can't be destroyed.
impl core::ops::Drop for VirtualMachine {
    fn drop(&mut self) {
        hv_unsafe_call!(hv_vm_destroy()).expect("Could not properly destroy VM context");
    }
}

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

#[allow(clippy::from_over_into)]
impl Into<hv_memory_flags_t> for MemPerms {
    fn into(self) -> hv_memory_flags_t {
        match self {
            Self::None => 0,
            Self::R => HV_MEMORY_READ,
            Self::W => HV_MEMORY_WRITE,
            Self::X => HV_MEMORY_EXEC,
            Self::RW => HV_MEMORY_READ | HV_MEMORY_WRITE,
            Self::RX => HV_MEMORY_READ | HV_MEMORY_EXEC,
            Self::WX => HV_MEMORY_WRITE | HV_MEMORY_EXEC,
            Self::RWX => HV_MEMORY_READ | HV_MEMORY_WRITE | HV_MEMORY_EXEC,
        }
    }
}

impl core::fmt::Display for MemPerms {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let perms = match *self {
            MemPerms::None => "---",
            MemPerms::R => "R--",
            MemPerms::W => "-W-",
            MemPerms::X => "--X",
            MemPerms::RW => "RW-",
            MemPerms::RX => "R-X",
            MemPerms::WX => "-WX",
            MemPerms::RWX => "RWX",
        };
        write!(f, "{}", perms)
    }
}

impl std::ops::BitOr for MemPerms {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let raw = Into::<hv_memory_flags_t>::into(self);
        let rhs_raw = Into::<hv_memory_flags_t>::into(rhs);
        match raw | rhs_raw {
            x if x == HV_MEMORY_READ => Self::R,
            x if x == HV_MEMORY_WRITE => Self::W,
            x if x == HV_MEMORY_EXEC => Self::X,
            x if x == HV_MEMORY_READ | HV_MEMORY_WRITE => Self::RW,
            x if x == HV_MEMORY_READ | HV_MEMORY_EXEC => Self::RX,
            x if x == HV_MEMORY_WRITE | HV_MEMORY_EXEC => Self::WX,
            x if x == HV_MEMORY_READ | HV_MEMORY_WRITE | HV_MEMORY_EXEC => Self::RWX,
            _ => Self::None,
        }
    }
}

/// The size of a memory page on Apple Silicon.
pub const PAGE_SIZE: usize = 0x4000;

/// Represents a host memory allocation.
#[derive(Clone, Debug, Eq)]
pub(crate) struct MemAlloc {
    /// Host address.
    addr: *const c_void,
    /// Memory layout associated with `addr`.
    layout: alloc::Layout,
    /// Allocation size.
    size: usize,
}

impl MemAlloc {
    /// Creates a new memory allocation for the host using [`std::alloc`].
    pub(crate) fn new(size: usize) -> std::result::Result<Self, alloc::LayoutError> {
        let layout = alloc::Layout::from_size_align(size, PAGE_SIZE)?.pad_to_align();
        let addr = unsafe { alloc::alloc_zeroed(layout) } as *const c_void;
        Ok(MemAlloc {
            addr,
            layout,
            size: layout.size(),
        })
    }
}

impl PartialEq for MemAlloc {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr && self.size == other.size
    }
}

impl Hash for MemAlloc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr.hash(state);
        self.size.hash(state);
    }
}

impl std::ops::Drop for MemAlloc {
    fn drop(&mut self) {
        unsafe { alloc::dealloc(self.addr as *mut u8, self.layout) }
    }
}

/// Represents a memory mapping between a host-allocated memory range and the one that
/// corresponds in the hypervisor guest.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct MappingInner {
    host_alloc: MemAlloc,
    guest_addr: Option<u64>,
    size: usize,
    perms: MemPerms,
}

/// Represents a memory range exclusive to a single thread.
///
/// **Note:** a memory mapping is available to all vCPU running in a given VM instance, but only
/// one vCPU-owning thread can access it.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Mapping {
    inner: MappingInner,
}

impl Mappable for Mapping {
    fn new(size: usize) -> std::result::Result<Self, alloc::LayoutError> {
        let host_alloc = MemAlloc::new(size)?;
        Ok(Self {
            inner: MappingInner {
                host_alloc,
                guest_addr: None,
                size,
                perms: MemPerms::None,
            },
        })
    }

    fn map(&mut self, guest_addr: u64, perms: MemPerms) -> Result<()> {
        Self::map_inner(&mut self.inner, guest_addr, perms)
    }

    fn unmap(&mut self) -> Result<()> {
        Self::unmap_inner(&mut self.inner)
    }

    fn protect(&mut self, perms: MemPerms) -> Result<()> {
        Self::protect_inner(&mut self.inner, perms)
    }

    fn read(&self, guest_addr: u64, data: &mut [u8]) -> Result<usize> {
        Self::read_inner(&self.inner, guest_addr, data)
    }

    fn write(&mut self, guest_addr: u64, data: &[u8]) -> Result<usize> {
        Self::write_inner(&mut self.inner, guest_addr, data)
    }

    fn get_host_addr(&self) -> *const u8 {
        self.inner.host_alloc.addr as *const u8
    }

    fn get_guest_addr(&self) -> Option<u64> {
        self.inner.guest_addr
    }

    fn get_size(&self) -> usize {
        self.inner.size
    }
}

impl std::ops::Drop for Mapping {
    fn drop(&mut self) {
        let _ = self.unmap();
    }
}

/// Represents a memory range shared among multiple threads.
///
/// **Note:** a memory mapping is available to all vCPU running in a given VM instance, but any
/// vCPU-owning thread with a reference to this mapping can access it.
#[derive(Clone, Debug)]
pub struct MappingShared {
    inner: Arc<RwLock<MappingInner>>,
}

unsafe impl Send for MappingShared {}

impl PartialEq for MappingShared {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Mappable for MappingShared {
    fn new(size: usize) -> std::result::Result<Self, alloc::LayoutError> {
        let host_alloc = MemAlloc::new(size)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(MappingInner {
                host_alloc,
                guest_addr: None,
                size,
                perms: MemPerms::None,
            })),
        })
    }

    fn map(&mut self, guest_addr: u64, perms: MemPerms) -> Result<()> {
        let mut inner = self.inner.write().unwrap();
        Self::map_inner(&mut inner, guest_addr, perms)
    }

    fn unmap(&mut self) -> Result<()> {
        let mut inner = self.inner.write().unwrap();
        Self::unmap_inner(&mut inner)
    }

    fn protect(&mut self, perms: MemPerms) -> Result<()> {
        let mut inner = self.inner.write().unwrap();
        Self::protect_inner(&mut inner, perms)
    }

    fn read(&self, guest_addr: u64, data: &mut [u8]) -> Result<usize> {
        let inner = self.inner.read().unwrap();
        Self::read_inner(&inner, guest_addr, data)
    }

    fn write(&mut self, guest_addr: u64, data: &[u8]) -> Result<usize> {
        let mut inner = self.inner.write().unwrap();
        Self::write_inner(&mut inner, guest_addr, data)
    }

    fn get_host_addr(&self) -> *const u8 {
        self.inner.read().unwrap().host_alloc.addr as *const u8
    }

    fn get_guest_addr(&self) -> Option<u64> {
        self.inner.read().unwrap().guest_addr
    }

    fn get_size(&self) -> usize {
        self.inner.read().unwrap().size
    }
}

impl Hash for MappingShared {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let inner = self.inner.read().unwrap();
        inner.hash(state);
    }
}

impl std::ops::Drop for MappingShared {
    fn drop(&mut self) {
        let _ = self.unmap();
    }
}

pub trait Mappable {
    /// Creates a new allocation object.
    fn new(size: usize) -> std::result::Result<Self, alloc::LayoutError>
    where
        Self: Sized;

    /// Maps the host allocation in the guest.
    fn map(&mut self, guest_addr: u64, perms: MemPerms) -> Result<()>;

    /// Maps the host allocation in the guest.
    fn unmap(&mut self) -> Result<()>;

    /// Changes the protections of memory mapping in the guest.
    fn protect(&mut self, perms: MemPerms) -> Result<()>;

    /// Reads from a memory mapping in the guest at address `guest_addr`.
    fn read(&self, guest_addr: u64, data: &mut [u8]) -> Result<usize>;

    /// Writes to a memory mapping in the guest at address `guest_addr`.
    fn write(&mut self, guest_addr: u64, data: &[u8]) -> Result<usize>;

    /// Retrieves the memory mapping's host address.
    fn get_host_addr(&self) -> *const u8;

    /// Retrieves the memory mapping's guest address.
    fn get_guest_addr(&self) -> Option<u64>;

    /// Retrieves the memory mapping's size.
    fn get_size(&self) -> usize;

    /// Underlying memory mapping function.
    fn map_inner(inner: &mut MappingInner, guest_addr: u64, perms: MemPerms) -> Result<()>
    where
        Self: Sized,
    {
        // Returns if the mapping is already mapped.
        if inner.guest_addr.is_some() {
            return Err(HypervisorError::Busy);
        }
        // Maps the mapping in the guest.
        hv_unsafe_call!(hv_vm_map(
            inner.host_alloc.addr,
            guest_addr,
            inner.host_alloc.size,
            Into::<hv_memory_flags_t>::into(perms)
        ))?;
        // Updates the inner mapping.
        inner.guest_addr = Some(guest_addr);
        inner.perms = perms;
        Ok(())
    }

    /// Underlying memory unmapping function.
    fn unmap_inner(inner: &mut MappingInner) -> Result<()>
    where
        Self: Sized,
    {
        // Returns if the mapping is not mapped.
        let guest_addr = inner.guest_addr.ok_or(HypervisorError::Error)?;
        // Unmaps the mapping from the guest.
        hv_unsafe_call!(hv_vm_unmap(guest_addr, inner.host_alloc.size))?;
        // Updates the inner mapping.
        inner.guest_addr = None;
        Ok(())
    }

    /// Underlying memory protection function.
    fn protect_inner(inner: &mut MappingInner, perms: MemPerms) -> Result<()>
    where
        Self: Sized,
    {
        // Returns if the mapping is not mapped.
        let guest_addr = inner.guest_addr.ok_or(HypervisorError::Error)?;
        // Changes the guest mapping's protections.
        hv_unsafe_call!(hv_vm_protect(
            guest_addr,
            inner.host_alloc.size,
            Into::<hv_memory_flags_t>::into(perms)
        ))?;
        // Updates the inner mapping.
        inner.perms = perms;
        Ok(())
    }

    /// Underlying memory read function.
    fn read_inner(inner: &MappingInner, guest_addr: u64, data: &mut [u8]) -> Result<usize>
    where
        Self: Sized,
    {
        // Returns if the mapping is not mapped.
        let inner_guest_addr = inner.guest_addr.ok_or(HypervisorError::Error)?;
        // Checks the guest addr provided is in the guest memory range.
        let size = data.len();
        if guest_addr < inner_guest_addr
            || guest_addr.checked_add(size as u64).unwrap()
                > inner_guest_addr
                    .checked_add(inner.host_alloc.size as u64)
                    .unwrap()
        {
            return Err(HypervisorError::BadArgument);
        }
        // Computes the corresponding host address.
        let offset = guest_addr - inner_guest_addr;
        let host_addr = inner.host_alloc.addr as u64 + offset;
        // Copies data from the memory mapping into the slice.
        unsafe {
            ptr::copy(
                host_addr as *const c_void,
                data.as_mut_ptr() as *mut c_void,
                size,
            );
        };
        Ok(size)
    }

    /// Reads one byte at address `guest_addr`.
    #[inline]
    fn read_byte(&self, guest_addr: u64) -> Result<u8> {
        let mut data = [0; 1];
        assert_eq!(self.read(guest_addr, &mut data)?, 1);
        Ok(data[0])
    }

    /// Reads one word at address `guest_addr`.
    #[inline]
    fn read_word(&self, guest_addr: u64) -> Result<u16> {
        let mut data = [0; 2];
        assert_eq!(self.read(guest_addr, &mut data)?, 2);
        Ok(u16::from_le_bytes(data[..2].try_into().unwrap()))
    }

    /// Reads one dword at address `guest_addr`.
    #[inline]
    fn read_dword(&self, guest_addr: u64) -> Result<u32> {
        let mut data = [0; 4];
        assert_eq!(self.read(guest_addr, &mut data)?, 4);
        Ok(u32::from_le_bytes(data[..4].try_into().unwrap()))
    }

    /// Reads one qword at address `guest_addr`.
    #[inline]
    fn read_qword(&self, guest_addr: u64) -> Result<u64> {
        let mut data = [0; 8];
        assert_eq!(self.read(guest_addr, &mut data)?, 8);
        Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
    }

    /// Underlying memory write function.
    fn write_inner(inner: &mut MappingInner, guest_addr: u64, data: &[u8]) -> Result<usize>
    where
        Self: Sized,
    {
        let size = data.len();
        // Returns if the mapping is not mapped.
        let inner_guest_addr = inner.guest_addr.ok_or(HypervisorError::Error)?;
        // Checks the guest addr provided is in the guest memory range.
        if guest_addr < inner_guest_addr
            || guest_addr.checked_add(size as u64).unwrap()
                > inner_guest_addr
                    .checked_add(inner.host_alloc.size as u64)
                    .unwrap()
        {
            return Err(HypervisorError::BadArgument);
        }
        // Computes the corresponding host address.
        let offset = guest_addr - inner_guest_addr;
        let host_addr = inner.host_alloc.addr as u64 + offset;
        // Copies data from the input vector.
        unsafe {
            ptr::copy(
                data.as_ptr() as *const c_void,
                host_addr as *mut c_void,
                size,
            );
        };
        Ok(size)
    }

    /// Writes one byte at address `guest_addr`.
    #[inline]
    fn write_byte(&mut self, guest_addr: u64, data: u8) -> Result<usize> {
        self.write(guest_addr, &[data])
    }

    /// Writes one word at address `guest_addr`.
    #[inline]
    fn write_word(&mut self, guest_addr: u64, data: u16) -> Result<usize> {
        self.write(guest_addr, &data.to_le_bytes())
    }

    /// Writes one dword at address `guest_addr`.
    #[inline]
    fn write_dword(&mut self, guest_addr: u64, data: u32) -> Result<usize> {
        self.write(guest_addr, &data.to_le_bytes())
    }

    /// Writes one qword at address `guest_addr`.
    #[inline]
    fn write_qword(&mut self, guest_addr: u64, data: u64) -> Result<usize> {
        self.write(guest_addr, &data.to_le_bytes())
    }
}

// -----------------------------------------------------------------------------------------------
// vCPU Management - Configuration
// -----------------------------------------------------------------------------------------------

/// Represents a vCPU configuration.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct VcpuConfig(hv_vcpu_config_t);

impl Default for VcpuConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl VcpuConfig {
    /// Instanciates a new configuration.
    pub fn new() -> Self {
        let config = unsafe { hv_vcpu_config_create() };
        VcpuConfig(config)
    }

    /// Instanciates a new empty configuration.
    pub fn empty() -> Self {
        VcpuConfig(ptr::null_mut())
    }

    /// Retrieves the value of a feature register.
    pub fn get_feature_reg(&self, reg: FeatureReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_config_get_feature_reg(
            self.0,
            Into::<hv_feature_reg_t>::into(reg),
            &mut value
        ))?;
        Ok(value)
    }

    /// Returns the Cache Size ID Register (CCSIDR_EL1) values for the vCPU configuration and
    /// cache type you specify.
    pub fn get_ccsidr_el1_sys_reg_values(&self, cache_type: CacheType) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_config_get_ccsidr_el1_sys_reg_values(
            self.0,
            Into::<hv_cache_type_t>::into(cache_type),
            &mut value
        ))?;
        Ok(value)
    }
}

// -----------------------------------------------------------------------------------------------
// vCPU
// -----------------------------------------------------------------------------------------------

/// Represents a vCPU instance.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct VcpuInstance(hv_vcpu_t);

pub type VcpuExitException = hv_vcpu_exit_exception_t;

/// Represents vCPU exit info.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VcpuExit {
    pub reason: ExitReason,
    pub exception: VcpuExitException,
}

impl From<hv_vcpu_exit_t> for VcpuExit {
    fn from(exit: hv_vcpu_exit_t) -> Self {
        VcpuExit {
            reason: ExitReason::from(exit.reason),
            exception: exit.exception,
        }
    }
}

impl std::fmt::Display for VcpuExit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.reason {
            ExitReason::EXCEPTION => {
                writeln!(f,
                "EXCEPTION => [syndrome: {:016x}, virtual addr: {:016x}, physical addr: {:016x}]",
                self.exception.syndrome, self.exception.virtual_address,
                self.exception.physical_address)
            }
            ExitReason::CANCELED => writeln!(f, "CANCELED"),
            ExitReason::VTIMER_ACTIVATED => writeln!(f, "VTIMER_ACTIVATED"),
            ExitReason::UNKNOWN => writeln!(f, "UNKNOWN"),
        }
    }
}

/// Represents a Virtual CPU.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Vcpu {
    vcpu: VcpuInstance,
    config: VcpuConfig,
    exit: *const hv_vcpu_exit_t,
}

impl Vcpu {
    /// Creates a new vCPU.
    pub fn new() -> Result<Self> {
        Vcpu::with_config(VcpuConfig::empty())
    }

    /// Creates a new vCPU with a user-provided config.
    pub fn with_config(config: VcpuConfig) -> Result<Self> {
        let mut vcpu = VcpuInstance(0);
        let mut exit = ptr::null_mut() as *const hv_vcpu_exit_t;
        hv_unsafe_call!(hv_vcpu_create(&mut vcpu.0, &mut exit, config.0))?;
        Ok(Self { vcpu, exit, config })
    }

    /// Returns the [`VcpuInstance`] associated with the Vcpu.
    pub fn get_instance(&self) -> VcpuInstance {
        self.vcpu
    }

    /// Returns the Vcpu ID (the integer associated to the corresponding [`VcpuInstance`]).
    pub fn get_id(&self) -> u64 {
        self.vcpu.0
    }

    /// Returns the maximum number of vCPUs that can be created by the hypervisor.
    pub fn get_max_count() -> Result<u32> {
        let mut count = 0;
        hv_unsafe_call!(hv_vm_get_max_vcpu_count(&mut count))?;
        Ok(count)
    }

    /// Starts the vCPU.
    pub fn run(&self) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_run(self.vcpu.0))
    }

    /// Stops all vCPUs in the input array.
    pub fn stop(vcpus: &[VcpuInstance]) -> Result<()> {
        let vcpus = vcpus.iter().map(|v| v.0).collect::<Vec<hv_vcpu_t>>();
        hv_unsafe_call!(hv_vcpus_exit(vcpus.as_ptr(), vcpus.len() as u32))
    }

    /// Gets vCPU exit info.
    pub fn get_exit_info(&self) -> VcpuExit {
        VcpuExit::from(unsafe { *self.exit })
    }

    /// Gets pending interrupts for a vCPU.
    pub fn get_pending_interrupt(&self, intr: InterruptType) -> Result<bool> {
        let mut pending = false;
        hv_unsafe_call!(hv_vcpu_get_pending_interrupt(
            self.vcpu.0,
            Into::<hv_interrupt_type_t>::into(intr),
            &mut pending
        ))?;
        Ok(pending)
    }

    /// Sets pending interrupts for a vCPU.
    pub fn set_pending_interrupt(&self, intr: InterruptType, pending: bool) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_pending_interrupt(
            self.vcpu.0,
            Into::<hv_interrupt_type_t>::into(intr),
            pending
        ))
    }

    /// Gets the value of a vCPU general purpose register.
    pub fn get_reg(&self, reg: Reg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_get_reg(
            self.vcpu.0,
            Into::<hv_reg_t>::into(reg),
            &mut value
        ))?;
        Ok(value)
    }

    /// Sets the value of a vCPU general purpose register.
    pub fn set_reg(&self, reg: Reg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_reg(
            self.vcpu.0,
            Into::<hv_reg_t>::into(reg),
            value
        ))
    }

    #[cfg(feature = "simd_nightly")]
    /// Gets the value of a vCPU floating point register
    pub fn get_simd_fp_reg(&self, reg: SimdFpReg) -> Result<simd::i8x16> {
        let mut value = simd::i8x16::from_array([0; 16]);
        hv_unsafe_call!(hv_vcpu_get_simd_fp_reg(
            self.vcpu.0,
            Into::<hv_simd_fp_reg_t>::into(reg),
            &mut value
        ))?;
        Ok(value)
    }

    #[cfg(feature = "simd_nightly")]
    /// Sets the value of a vCPU floating point register
    pub fn set_simd_fp_reg(&self, reg: SimdFpReg, value: simd::i8x16) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_simd_fp_reg(
            self.vcpu.0,
            Into::<hv_simd_fp_reg_t>::into(reg),
            value
        ))
    }

    #[cfg(not(feature = "simd_nightly"))]
    /// Gets the value of a vCPU floating point register
    pub fn get_simd_fp_reg(&self, reg: SimdFpReg) -> Result<u128> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_get_simd_fp_reg(
            self.vcpu.0,
            Into::<hv_simd_fp_reg_t>::into(reg),
            &mut value
        ))?;
        Ok(value)
    }

    #[cfg(not(feature = "simd_nightly"))]
    /// Sets the value of a vCPU floating point register
    pub fn set_simd_fp_reg(&self, reg: SimdFpReg, value: u128) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_simd_fp_reg(
            self.vcpu.0,
            Into::<hv_simd_fp_reg_t>::into(reg),
            value
        ))
    }

    /// Gets the value of a vCPU system register.
    pub fn get_sys_reg(&self, reg: SysReg) -> Result<u64> {
        let mut value = 0;
        hv_unsafe_call!(hv_vcpu_get_sys_reg(
            self.vcpu.0,
            Into::<hv_sys_reg_t>::into(reg),
            &mut value
        ))?;
        Ok(value)
    }

    /// Sets the value of a vCPU general purpose register.
    pub fn set_sys_reg(&self, reg: SysReg, value: u64) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_sys_reg(
            self.vcpu.0,
            Into::<hv_sys_reg_t>::into(reg),
            value
        ))
    }

    /// Gets whether debug exceptions exit the guest.
    pub fn get_trap_debug_exceptions(&self) -> Result<bool> {
        let mut value = false;
        hv_unsafe_call!(hv_vcpu_get_trap_debug_exceptions(self.vcpu.0, &mut value))?;
        Ok(value)
    }

    /// Sets whether debug exceptions exit the guest.
    pub fn set_trap_debug_exceptions(&self, value: bool) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_trap_debug_exceptions(self.vcpu.0, value))
    }

    /// Gets whether debug-register accesses exit the guest.
    pub fn get_trap_debug_reg_accesses(&self) -> Result<bool> {
        let mut value = false;
        hv_unsafe_call!(hv_vcpu_get_trap_debug_reg_accesses(self.vcpu.0, &mut value))?;
        Ok(value)
    }

    /// Sets whether debug-register accesses exit the guest.
    pub fn set_trap_debug_reg_accesses(&self, value: bool) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_trap_debug_reg_accesses(self.vcpu.0, value))
    }

    /// Returns the cumulative execution time of a vCPU, in nanoseconds.
    pub fn get_exec_time(&self) -> Result<u64> {
        let mut time = 0;
        hv_unsafe_call!(hv_vcpu_get_exec_time(self.vcpu.0, &mut time))?;
        Ok(time)
    }

    /// Gets the virtual timer mask.
    pub fn get_vtimer_mask(&self) -> Result<bool> {
        let mut vtimer_is_masked = false;
        hv_unsafe_call!(hv_vcpu_get_vtimer_mask(self.vcpu.0, &mut vtimer_is_masked))?;
        Ok(vtimer_is_masked)
    }

    /// Sets or clears the virtual timer mask.
    pub fn set_vtimer_mask(&self, vtimer_is_masked: bool) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_vtimer_mask(self.vcpu.0, vtimer_is_masked))
    }

    /// Returns the vTimer offset for the vCPU ID you specify.
    pub fn get_vtimer_offset(&self) -> Result<u64> {
        let mut vtimer_offset = 0;
        hv_unsafe_call!(hv_vcpu_get_vtimer_offset(self.vcpu.0, &mut vtimer_offset))?;
        Ok(vtimer_offset)
    }

    /// Sets the vTimer offset to a value that you provide.
    pub fn set_vtimer_offset(&self, vtimer_offset: u64) -> Result<()> {
        hv_unsafe_call!(hv_vcpu_set_vtimer_offset(self.vcpu.0, vtimer_offset))
    }
}

impl std::ops::Drop for Vcpu {
    fn drop(&mut self) {
        hv_unsafe_call!(hv_vcpu_destroy(self.vcpu.0))
            .expect("Could not properly destroy vCPU instance");
    }
}

impl std::fmt::Display for Vcpu {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "EL0:")?;
        writeln!(
            f,
            "     X0: {:016x}    X1: {:016x}     X2: {:016x}     X3: {:016x}",
            self.get_reg(Reg::X0).unwrap(),
            self.get_reg(Reg::X1).unwrap(),
            self.get_reg(Reg::X2).unwrap(),
            self.get_reg(Reg::X3).unwrap()
        )?;
        writeln!(
            f,
            "     X4: {:016x}    X5: {:016x}     X6: {:016x}     X7: {:016x}",
            self.get_reg(Reg::X4).unwrap(),
            self.get_reg(Reg::X5).unwrap(),
            self.get_reg(Reg::X6).unwrap(),
            self.get_reg(Reg::X7).unwrap()
        )?;
        writeln!(
            f,
            "     X8: {:016x}    X9: {:016x}    X10: {:016x}    X11: {:016x}",
            self.get_reg(Reg::X8).unwrap(),
            self.get_reg(Reg::X9).unwrap(),
            self.get_reg(Reg::X10).unwrap(),
            self.get_reg(Reg::X11).unwrap()
        )?;
        writeln!(
            f,
            "    X12: {:016x}   X13: {:016x}    X14: {:016x}    X15: {:016x}",
            self.get_reg(Reg::X12).unwrap(),
            self.get_reg(Reg::X13).unwrap(),
            self.get_reg(Reg::X14).unwrap(),
            self.get_reg(Reg::X15).unwrap()
        )?;
        writeln!(
            f,
            "    X16: {:016x}   X17: {:016x}    X18: {:016x}    X19: {:016x}",
            self.get_reg(Reg::X16).unwrap(),
            self.get_reg(Reg::X17).unwrap(),
            self.get_reg(Reg::X18).unwrap(),
            self.get_reg(Reg::X19).unwrap()
        )?;
        writeln!(
            f,
            "    X20: {:016x}   X21: {:016x}    X22: {:016x}    X23: {:016x}",
            self.get_reg(Reg::X20).unwrap(),
            self.get_reg(Reg::X21).unwrap(),
            self.get_reg(Reg::X22).unwrap(),
            self.get_reg(Reg::X23).unwrap()
        )?;
        writeln!(
            f,
            "    X24: {:016x}   X25: {:016x}    X26: {:016x}    X27: {:016x}",
            self.get_reg(Reg::X24).unwrap(),
            self.get_reg(Reg::X25).unwrap(),
            self.get_reg(Reg::X26).unwrap(),
            self.get_reg(Reg::X27).unwrap()
        )?;
        writeln!(
            f,
            "    X28: {:016x}   X29: {:016x}     LR: {:016x}     PC: {:016x}",
            self.get_reg(Reg::X28).unwrap(),
            self.get_reg(Reg::X29).unwrap(),
            self.get_reg(Reg::LR).unwrap(),
            self.get_reg(Reg::PC).unwrap()
        )?;
        writeln!(
            f,
            "     SP: {:016x}",
            self.get_sys_reg(SysReg::SP_EL0).unwrap()
        )?;
        writeln!(f, "EL1:")?;
        writeln!(
            f,
            "  SCTLR: {:016x}    SP: {:016x}",
            self.get_sys_reg(SysReg::SCTLR_EL1).unwrap(),
            self.get_sys_reg(SysReg::SP_EL1).unwrap()
        )?;
        writeln!(
            f,
            "   CPSR: {:016x}  SPSR: {:016x}",
            self.get_reg(Reg::CPSR).unwrap(),
            self.get_sys_reg(SysReg::SPSR_EL1).unwrap()
        )?;
        writeln!(
            f,
            "    FAR: {:016x}   PAR: {:016x}",
            self.get_sys_reg(SysReg::FAR_EL1).unwrap(),
            self.get_sys_reg(SysReg::PAR_EL1).unwrap()
        )?;
        writeln!(
            f,
            "    ESR: {:016x}   ELR: {:016x}",
            self.get_sys_reg(SysReg::ESR_EL1).unwrap(),
            self.get_sys_reg(SysReg::ELR_EL1).unwrap()
        )
    }
}

// -----------------------------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------------------------
    // Virtual Machine

    #[test]
    fn vm_create_destroy() {
        {
            // Creating a first VM instance should work!
            let vm1 = VirtualMachine::new();
            assert!(vm1.is_ok());
            // Creating a second instance should fail.
            let vm2 = VirtualMachine::new();
            assert_eq!(vm2, Err(HypervisorError::Busy));
            // Dropping the process vm instance...
        }
        // ... now creating a new instance should work.
        let vm3 = VirtualMachine::new();
        assert!(vm3.is_ok());
    }

    // -------------------------------------------------------------------------------------------
    // Memory Management

    #[test]
    fn memory_map_unmap() {
        let _vm = VirtualMachine::new().unwrap();
        // Creating a new mapping of size 0x1000.
        let mut mem = Mapping::new(0x1000).unwrap();
        // Mapping it at a non-page-aligned address in the guest should not work...
        assert_eq!(
            mem.map(0x1000, MemPerms::RW),
            Err(HypervisorError::BadArgument)
        );
        // ... but a page-aligned address should.
        assert_eq!(mem.map(0x4000, MemPerms::RW), Ok(()));
        // Unmapping it should also work.
        assert_eq!(mem.unmap(), Ok(()));
        // Mapping it twice should not work though.
        assert_eq!(mem.map(0x4000, MemPerms::RW), Ok(()));
        assert_eq!(mem.map(0x4000, MemPerms::RW), Err(HypervisorError::Busy));
        // Creating a second mapping of size 0x1000.
        let mut mem2 = Mapping::new(0x1000).unwrap();
        // Mapping it at the location of the first one should not work.
        assert_eq!(mem2.map(0x4000, MemPerms::RW), Err(HypervisorError::Error));
    }

    #[test]
    fn memory_map_same_address() {
        let _vm = VirtualMachine::new().unwrap();
        // Creating two mappings of size 0x1000.
        let mut mem1 = Mapping::new(0x1000).unwrap();
        let mut mem2 = Mapping::new(0x1000).unwrap();
        // Maps the two mappings at the same address.
        assert_eq!(mem1.map(0x4000, MemPerms::RW), Ok(()));
        assert_eq!(mem2.map(0x4000, MemPerms::RW), Err(HypervisorError::Error));

        let mut mem3 = Mapping::new(0x1000).unwrap();
        assert_eq!(mem3.map(0x20000, MemPerms::RW), Ok(()));
    }

    #[test]
    fn memory_read_write_protect() {
        let _vm = VirtualMachine::new().unwrap();
        let mut mem = Mapping::new(0x1000).unwrap();
        // Mapping memory as Read/Write
        assert_eq!(mem.map(0x10000, MemPerms::RW), Ok(()));
        // Writing 0xdeadbeef in the guest allocated memory.
        assert_eq!(mem.write_dword(0x12345, 0xdeadbeef), Ok(4));
        // Reading at the same location and making sure we're reading 0xdeadbeef.
        assert_eq!(mem.read_dword(0x12345), Ok(0xdeadbeef));
        // Testing all write functions
        assert_eq!(mem.write(0x10000, &vec![0x10, 0x11, 0x12, 0x13]), Ok(4));
        assert_eq!(mem.write_byte(0x10010, 0x41), Ok(1));
        assert_eq!(mem.write_word(0x10020, 0x4242), Ok(2));
        assert_eq!(mem.write_dword(0x10030, 0x43434343), Ok(4));
        assert_eq!(mem.write_qword(0x10040, 0x4444444444444444), Ok(8));
        // Testing all read functions
        let mut data = [0; 4];
        let ret = mem.read(0x10000, &mut data);
        assert_eq!(ret, Ok(4));
        assert_eq!(data, [0x10, 0x11, 0x12, 0x13]);
        assert_eq!(mem.read_byte(0x10010), Ok(0x41));
        assert_eq!(mem.read_word(0x10020), Ok(0x4242));
        assert_eq!(mem.read_dword(0x10030), Ok(0x43434343));
        assert_eq!(mem.read_qword(0x10040), Ok(0x4444444444444444));
        // Changing the mapping permissions
        assert_eq!(mem.protect(MemPerms::R), Ok(()));
    }

    #[test]
    #[ignore]
    fn memory_map_unmap_threads() {
        let mut mem1 = MappingShared::new(0x1000).unwrap();
        mem1.map(0, MemPerms::RW).expect("could not map memory");
        let mem2 = mem1.clone();
        let mut mem3 = mem1.clone();

        let t1 = std::thread::spawn(move || {
            println!(
                "write val 0xdeadbeef = {:?}",
                mem1.write_dword(0, 0xdeadbeef)
            );
            std::thread::sleep(std::time::Duration::from_millis(5000));
        });

        let t2 = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(2000));
            println!("read val = {:?}", mem2.read_dword(0));
            std::thread::sleep(std::time::Duration::from_millis(2000));
            println!("read val = {:?}", mem2.read_dword(0));
        });

        let t3 = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(3000));
            println!("write val 0 = {:?}", mem3.write_dword(0, 0));
            std::thread::sleep(std::time::Duration::from_millis(7000));
        });

        t1.join().expect("could not join 1st thread");
        t2.join().expect("could not join 2nd thread");
        t3.join().expect("could not join 3rd thread");
    }

    // -------------------------------------------------------------------------------------------
    // Vcpu

    #[test]
    fn vcpu_config_create_get_values() {
        let config = VcpuConfig::new();
        // Reading feature reg from the config.
        assert!(config.get_feature_reg(FeatureReg::ID_AA64DFR0_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::ID_AA64DFR1_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::ID_AA64ISAR0_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::ID_AA64ISAR1_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::ID_AA64MMFR0_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::ID_AA64MMFR1_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::ID_AA64MMFR2_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::ID_AA64PFR0_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::ID_AA64PFR1_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::CTR_EL0).is_ok());
        assert!(config.get_feature_reg(FeatureReg::CLIDR_EL1).is_ok());
        assert!(config.get_feature_reg(FeatureReg::DCZID_EL0).is_ok());
        // Reading the Cache Size ID Register.
        assert!(config
            .get_ccsidr_el1_sys_reg_values(CacheType::DATA)
            .is_ok());
        assert!(config
            .get_ccsidr_el1_sys_reg_values(CacheType::INSTRUCTION)
            .is_ok());
    }

    #[test]
    fn vcpu_get_count() {
        // let vm = VirtualMachine::new();
        assert!(Vcpu::get_max_count().is_ok());
    }

    #[test]
    fn vcpu_create_destroy() {
        let _vm = VirtualMachine::new().unwrap();
        let mut mem = Mapping::new(0x1000).unwrap();
        // Creating a vCPU in the main thread should work.
        let vcpu1 = Vcpu::new();
        assert!(vcpu1.is_ok());
        // Creating a second one should fail.
        let vcpu2 = Vcpu::new();
        assert_eq!(vcpu2, Err(HypervisorError::Busy));
        mem.map(0, MemPerms::RW).expect("could not map memory");
        let t = std::thread::spawn(move || {
            assert!(Vcpu::new().is_ok());
        });
        t.join().expect("could not join thread");
    }

    #[test]
    fn vcpu_get_set_registers() {
        let _vm = VirtualMachine::new().unwrap();
        let vcpu = Vcpu::new().unwrap();
        // Setting GP registers
        assert_eq!(vcpu.set_reg(Reg::X0, 0x01010101), Ok(()));
        assert_eq!(vcpu.set_reg(Reg::X1, 0x12121212), Ok(()));
        assert_eq!(vcpu.set_reg(Reg::X2, 0x23232323), Ok(()));
        assert_eq!(vcpu.set_reg(Reg::X3, 0x34343434), Ok(()));
        assert_eq!(vcpu.set_reg(Reg::X4, 0x45454545), Ok(()));
        // Getting GP registers' values
        assert_eq!(vcpu.get_reg(Reg::X0), Ok(0x01010101));
        assert_eq!(vcpu.get_reg(Reg::X1), Ok(0x12121212));
        assert_eq!(vcpu.get_reg(Reg::X2), Ok(0x23232323));
        assert_eq!(vcpu.get_reg(Reg::X3), Ok(0x34343434));
        assert_eq!(vcpu.get_reg(Reg::X4), Ok(0x45454545));

        #[cfg(not(feature = "simd_nightly"))]
        {
            // Setting floating point registers
            let simd1 = u128::from_le_bytes([0x1; 16]);
            let simd2 = u128::from_le_bytes([0x2; 16]);
            let simd3 = u128::from_le_bytes([0x3; 16]);
            let simd4 = u128::from_le_bytes([0x4; 16]);
            let simd5 = u128::from_le_bytes([0x5; 16]);
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q0, simd1), Ok(()));
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q1, simd2), Ok(()));
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q2, simd3), Ok(()));
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q3, simd4), Ok(()));
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q4, simd5), Ok(()));
            // Getting floating point registers' values
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q0), Ok(simd1));
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q1), Ok(simd2));
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q2), Ok(simd3));
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q3), Ok(simd4));
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q4), Ok(simd5));
        }
        #[cfg(feature = "simd_nightly")]
        {
            // Setting floating point registers
            let simd1 = simd::i8x16::from_array([0x1; 16]);
            let simd2 = simd::i8x16::from_array([0x2; 16]);
            let simd3 = simd::i8x16::from_array([0x3; 16]);
            let simd4 = simd::i8x16::from_array([0x4; 16]);
            let simd5 = simd::i8x16::from_array([0x5; 16]);
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q0, simd1), Ok(()));
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q1, simd2), Ok(()));
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q2, simd3), Ok(()));
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q3, simd4), Ok(()));
            assert_eq!(vcpu.set_simd_fp_reg(SimdFpReg::Q4, simd5), Ok(()));
            // Getting floating point registers' values
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q0), Ok(simd1));
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q1), Ok(simd2));
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q2), Ok(simd3));
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q3), Ok(simd4));
            assert_eq!(vcpu.get_simd_fp_reg(SimdFpReg::Q4), Ok(simd5));
        }
    }

    #[test]
    fn vcpu_run() {
        let _vm = VirtualMachine::new().unwrap();
        let vcpu = Vcpu::new().unwrap();
        let mut mem = Mapping::new(0x1000).unwrap();
        assert_eq!(mem.map(0x4000, MemPerms::RWX), Ok(()));
        // Writes a `mov x0, #0x42` instruction at address 0x4000.
        assert_eq!(mem.write_dword(0x4000, 0xd2800840), Ok(4));
        // Writes a `brk #0` instruction at address 0x4004.
        assert_eq!(mem.write_dword(0x4004, 0xd4200000), Ok(4));
        // Sets PC to 0x4000.
        assert!(vcpu.set_reg(Reg::PC, 0x4000).is_ok());
        // Starts the Vcpu.
        assert!(vcpu.run().is_ok());
        let _exit_info = vcpu.get_exit_info();
        assert_eq!(vcpu.get_reg(Reg::X0), Ok(0x42));
    }
}
