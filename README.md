# APPLEVISOR

Safe Rust bindings for the Apple Silicon [`Hypervisor.framework`](https://developer.apple.com/documentation/hypervisor).

## Table of contents

 * [Getting Started](#getting-started)
   * [Starting a New Project](#starting-a-new-project)
   * [MacOS Versions and Features](#macos-versions-and-features)
   * [Compilation Workflow](#compilation-workflow)
 * [Documentation](#documentation)
 * [Example](#example)
 * [Tests and Coverage](#tests-and-coverage)
   * [Running the Tests](#running-the-tests)
   * [Gather Coverage](#gather-coverage)
 * [Hypervisor.Framework API](#hypervisorframework-api)

## Getting Started

### Starting a New Project

Create a Rust project and add Applevisor as a dependency in `Cargo.toml`. You can either pull it from [crates.io](https://crates.io/crates/applevisor) ...

```toml,no_run
# Check which version is the latest, this part of the README might not be updated
# in future releases.
applevisor = "1.0"
```

... or directly from the [GitHub repository](https://github.com/impalabs/applevisor).

```toml,no_run
applevisor = { git="https://github.com/impalabs/applevisor", branch="master" }
```

### MacOS Versions and Features

Since the Hypervisor.Framework is still evolving, depending on your macOS version (and those you want to support), some methods won't be available.

The following features can be used to determine the API available to your project:

- `macos-26-0`: all methods available up to MacOS 26.0 (current default feature);
- `macos-15-2`: all methods available up to MacOS 15.2;
- `macos-15-0`: all methods available up to MacOS 15.0;
- `macos-13-0`: all methods available up to MacOS 13.0;
- `macos-12-1`: all methods available up to MacOS 12.1;

If you only want the base methods introduced before MacOS 12.1, you must disable `default-features` when declaring applevisor as a dependency:

```toml,no_run
applevisor = {version = "x.x.x", default-features = false}
```

Refer to the [Hypervisor.Framework API](#hypervisorframework-api) section to get the list of available methods per macOS versions.

### Compilation Workflow

To be able to reach the Hypervisor Framework, a binary executable has to have been granted the [hypervisor entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com_apple_security_hypervisor). There are multiple entitlements you can add, which are documented by Apple, but the most important one is `com.apple.security.hypervisor`.

Create a file called `entitlements.txt` in the project's root directory and add the following:

```xml,no_run
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.hypervisor</key>
    <true/>
</dict>
</plist>
```

Build your project, and then sign the binary with your entitlements:

```console,no_run
cargo build --release
codesign --sign - --entitlements entitlements.xml --deep --force target/release/${PROJECT_NAME}
```

You can now run your program:

```console,no_run
target/release/${PROJECT_NAME}
```

## Documentation

The documentation is available online at the following address: [https://docs.rs/applevisor](https://docs.rs/applevisor)

Alternatively, you can generate the documentation using `cargo`:

```console,no_run
cargo doc --open
```

## Example

The following example:

 * creates a virtual machine for the current process;
 * creates a virtual CPU;
 * enables the hypervisor's debug features to be able to use breakpoints later on;
 * creates a physical memory mapping of 0x1000 bytes and maps it at address 0x4000 with RWX
   permissions;
 * writes the instructions `mov x0, #0x42; brk #0;` at address 0x4000;
 * sets PC to 0x4000;
 * starts the vCPU and runs our program;
 * returns when it encounters the breakpoint.

```rust,no_run
use applevisor::prelude::*;

fn main() -> Result<()> {
    // Creates a new virtual machine. There can be one, and only one, per process. Operations
    // on the virtual machine remains possible as long as this object is valid.
    let vm = VirtualMachine::new()?;

    // Creates a new virtual CPU. This object abstracts operations that can be performed on
    // CPUs, such as starting and stopping them, changing their registers, etc.
    let vcpu = vm.vcpu_create()?;

    // Enables debug features for the hypervisor. This is optional, but it might be required
    // for certain features to work, such as breakpoints.
    vcpu.set_trap_debug_exceptions(true)?;
    vcpu.set_trap_debug_reg_accesses(true)?;

    // Creates a mapping object that represents a 0x1000-byte physical memory range.
    let mut mem = vm.memory_create(0x1000)?;

    // This mapping needs to be mapped to effectively allocate physical memory for the guest.
    // Here we map the region at address 0x4000 and set the permissions to Read-Write-Execute.
    mem.map(0x4000, MemPerms::RWX)?;
    // Writes a `mov x0, #0x42` instruction at address 0x4000.
    mem.write_u32(0x4000, 0xd2800840)?;
    // Writes a `brk #0` instruction at address 0x4004.
    mem.write_u32(0x4004, 0xd4200000)?;

    // Sets PC to 0x4000.
    vcpu.set_reg(Reg::PC, 0x4000)?;

    // Starts the Vcpu. It will execute our mov and breakpoint instructions before stopping.
    vcpu.run()?;

    // The *exit information* can be used to used to retrieve different pieces of
    // information about the CPU exit status (e.g. exception type, fault address, etc.).
    let exit_info = vcpu.get_exit_info();

    // If everything went as expected, the value in X0 is 0x42...
    assert_eq!(vcpu.get_reg(Reg::X0), Ok(0x42));
    // ... the vcpu has stopped because of an exception ...
    assert_eq!(exit_info.reason, ExitReason::EXCEPTION);
    // ... and the exception syndrome corresponds to a breakpoint exception (which would
    // have been a different value without the call to `set_trap_debug_exceptions()`).
    assert_eq!(exit_info.exception.syndrome >> 26, 0b111100);

    Ok(())
}
```

## Tests and Coverage

### Running the tests

To run tests using the `Makefile` provided with the project, you'll first need to install [`jq`](https://stedolan.github.io/jq/download/). You can do so using `brew`:

```console,no_run
brew install jq
```

You can then run the tests with the provided `Makefile` using the following command:

```console,no_run
# To run all tests
make tests-all

# To run stable-only tests
make tests-stable-all

# To run nightly-only tests
make tests-nightly-all
```

## Gather Coverage

To get the tests coverage, make sure LLVM is installed, and `llvm-cov` and `llvm-profdata` are in your `PATH`.

You can then get the coverage with the provided `Makefile` using the following command:

```console,no_run
# Coverage for the latest macos version
make coverage-macos-26-0

# Other targets are available in the Makefile to get the coverage for previous macos versions.
make coverage-macos-$(major)-$(minor)
```

## Hypervisor.Framework API

- **MacOS >=26.0**
    - Use feature `macos-26-0`.
    - **Available methods:**
        - [`hv_vm_config_get_default_ipa_granule`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_get_default_ipa_granule%28_:%29)
        - [`hv_vm_config_get_ipa_granule`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_get_ipa_granule%28_:_:%29)
        - [`hv_vm_config_set_ipa_granule`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_set_ipa_granule%28_:_:%29)

- **MacOS >=15.2 and <26.0**
    - Use feature `macos-15-2`.
    - **Available methods:**
        - [`hv_sme_config_get_max_svl_bytes`](https://developer.apple.com/documentation/hypervisor/hv_sme_config_get_max_svl_bytes%28_:%29)
        - [`hv_vcpu_get_sme_state`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_sme_state%28_:_:%29)
        - [`hv_vcpu_set_sme_state`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_sme_state%28_:_:%29)
        - [`hv_vcpu_get_sme_z_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_sme_z_reg%28_:_:_:_:%29)
        - [`hv_vcpu_set_sme_z_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_sme_z_reg%28_:_:_:_:%29)
        - [`hv_vcpu_get_sme_p_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_sme_p_reg%28_:_:_:_:%29)
        - [`hv_vcpu_set_sme_p_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_sme_p_reg%28_:_:_:_:%29)
        - [`hv_vcpu_get_sme_za_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_sme_za_reg%28_:_:_:%29)
        - [`hv_vcpu_set_sme_za_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_sme_za_reg%28_:_:_:%29)
        - [`hv_vcpu_get_sme_zt0_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_sme_zt0_reg%28_:_:%29)
        - [`hv_vcpu_set_sme_zt0_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_sme_zt0_reg%28_:_:%29)

- **MacOS >=15.0 and <15.2**
    - Use feature `macos-15-0`.
    - **Available methods:**
        - [`hv_gic_create`](https://developer.apple.com/documentation/hypervisor/hv_gic_create%28_:%29)
        - [`hv_gic_set_spi`](https://developer.apple.com/documentation/hypervisor/hv_gic_set_spi%28_:_:%29)
        - [`hv_gic_send_msi`](https://developer.apple.com/documentation/hypervisor/hv_gic_send_msi%28_:_:%29)
        - [`hv_gic_get_distributor_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_distributor_reg%28_:_:%29)
        - [`hv_gic_set_distributor_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_set_distributor_reg%28_:_:%29)
        - [`hv_gic_get_redistributor_base`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_redistributor_base%28_:_:%29)
        - [`hv_gic_get_redistributor_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_redistributor_reg%28_:_:_:%29)
        - [`hv_gic_set_redistributor_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_set_redistributor_reg%28_:_:_:%29)
        - [`hv_gic_get_icc_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_icc_reg%28_:_:_:%29)
        - [`hv_gic_set_icc_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_set_icc_reg%28_:_:_:%29)
        - [`hv_gic_get_ich_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_ich_reg%28_:_:_:%29)
        - [`hv_gic_set_ich_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_set_ich_reg%28_:_:_:%29)
        - [`hv_gic_get_icv_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_icv_reg%28_:_:_:%29)
        - [`hv_gic_set_icv_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_set_icv_reg%28_:_:_:%29)
        - [`hv_gic_get_msi_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_msi_reg%28_:_:%29)
        - [`hv_gic_set_msi_reg`](https://developer.apple.com/documentation/hypervisor/hv_gic_set_msi_reg%28_:_:%29)
        - [`hv_gic_set_state`](https://developer.apple.com/documentation/hypervisor/hv_gic_set_state%28_:_:%29)
        - [`hv_gic_reset`](https://developer.apple.com/documentation/hypervisor/hv_gic_reset%28%29)
        - [`hv_gic_config_create`](https://developer.apple.com/documentation/hypervisor/hv_gic_config_create%28%29)
        - [`hv_gic_config_set_distributor_base`](https://developer.apple.com/documentation/hypervisor/hv_gic_config_set_distributor_base%28_:_:%29)
        - [`hv_gic_config_set_redistributor_base`](https://developer.apple.com/documentation/hypervisor/hv_gic_config_set_redistributor_base%28_:_:%29)
        - [`hv_gic_config_set_msi_region_base`](https://developer.apple.com/documentation/hypervisor/hv_gic_config_set_msi_region_base%28_:_:%29)
        - [`hv_gic_config_set_msi_interrupt_range`](https://developer.apple.com/documentation/hypervisor/hv_gic_config_set_msi_interrupt_range%28_:_:_:%29)
        - [`hv_gic_get_distributor_size`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_distributor_size%28_:%29)
        - [`hv_gic_get_distributor_base_alignment`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_distributor_base_alignment%28_:%29)
        - [`hv_gic_get_redistributor_region_size`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_redistributor_region_size%28_:%29)
        - [`hv_gic_get_redistributor_size`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_redistributor_size%28_:%29)
        - [`hv_gic_get_redistributor_base_alignment`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_redistributor_base_alignment%28_:%29)
        - [`hv_gic_get_msi_region_size`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_msi_region_size%28_:%29)
        - [`hv_gic_get_msi_region_base_alignment`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_msi_region_base_alignment%28_:%29)
        - [`hv_gic_get_spi_interrupt_range`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_spi_interrupt_range%28_:_:%29)
        - [`hv_gic_get_intid`](https://developer.apple.com/documentation/hypervisor/hv_gic_get_intid%28_:_:%29)
        - [`hv_gic_state_create`](https://developer.apple.com/documentation/hypervisor/hv_gic_state_create%28%29)
        - [`hv_gic_state_get_size`](https://developer.apple.com/documentation/hypervisor/hv_gic_state_get_size%28_:_:%29)
        - [`hv_gic_state_get_data`](https://developer.apple.com/documentation/hypervisor/hv_gic_state_get_data%28_:_:%29)
        - [`hv_vm_config_get_el2_supported`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_get_el2_supported%28_:%29)
        - [`hv_vm_config_get_el2_enabled`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_get_el2_enabled%28_:_:%29)
        - [`hv_vm_config_set_el2_enabled`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_set_el2_enabled%28_:_:%29)

- **MacOS >=13.0 and <15.0**
    - Use feature `macos-13-0`.
    - **Available methods:**
        - [`hv_vm_config_create`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_create%28%29)
        - [`hv_vm_config_get_max_ipa_size`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_get_max_ipa_size%28_:%29)
        - [`hv_vm_config_get_default_ipa_size`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_get_default_ipa_size%28_:%29)
        - [`hv_vm_config_set_ipa_size`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_set_ipa_size%28_:_:%29)
        - [`hv_vm_config_get_ipa_size`](https://developer.apple.com/documentation/hypervisor/hv_vm_config_get_ipa_size%28_:_:%29)

- **MacOS >=12.1 and <13.0**
    - Use feature `macos-12-1`.
    - **Available methods:**
        - [`hv_vm_allocate`](https://developer.apple.com/documentation/hypervisor/hv_vm_allocate%28_:_:_:%29)
        - [`hv_vm_deallocate`](https://developer.apple.com/documentation/hypervisor/hv_vm_deallocate%28_:_:%29)

- **MacOS >=11.0 and <12.1**
    - Use `--no-default-features` or disable `default-features`.
    - **Available methods:**
        - [`hv_vcpu_create`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_create%28_:_:_:%29)
        - [`hv_vcpu_destroy`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_destroy%28_:%29)
        - [`hv_vcpu_get_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_reg%28_:_:_:%29)
        - [`hv_vcpu_set_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_reg%28_:_:_:%29)
        - [`hv_vcpu_get_simd_fp_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_simd_fp_reg%28_:_:_:%29)
        - [`hv_vcpu_set_simd_fp_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_simd_fp_reg%28_:_:_:%29)
        - [`hv_vcpu_get_sys_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_sys_reg%28_:_:_:%29)
        - [`hv_vcpu_set_sys_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_sys_reg%28_:_:_:%29)
        - [`hv_vcpu_get_pending_interrupt`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_pending_interrupt%28_:_:_:%29)
        - [`hv_vcpu_set_pending_interrupt`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_pending_interrupt%28_:_:_:%29)
        - [`hv_vcpu_get_trap_debug_exceptions`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_trap_debug_exceptions%28_:_:%29)
        - [`hv_vcpu_set_trap_debug_exceptions`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_trap_debug_exceptions%28_:_:%29)
        - [`hv_vcpu_get_trap_debug_reg_accesses`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_trap_debug_reg_accesses%28_:_:%29)
        - [`hv_vcpu_set_trap_debug_reg_accesses`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_trap_debug_reg_accesses%28_:_:%29)
        - [`hv_vcpu_run`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_run%28_:%29)
        - [`hv_vcpus_exit`](https://developer.apple.com/documentation/hypervisor/hv_vcpus_exit%28_:_:%29)
        - [`hv_vcpu_get_exec_time`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_exec_time%28_:_:%29)
        - [`hv_vcpu_get_vtimer_mask`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_vtimer_mask%28_:_:%29)
        - [`hv_vcpu_set_vtimer_mask`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_vtimer_mask%28_:_:%29)
        - [`hv_vcpu_get_vtimer_offset`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_get_vtimer_offset%28_:_:%29)
        - [`hv_vcpu_set_vtimer_offset`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_set_vtimer_offset%28_:_:%29)
        - [`hv_vcpu_config_create`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_config_create%28%29)
        - [`hv_vcpu_config_get_feature_reg`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_config_get_feature_reg%28_:_:_:%29)
        - [`hv_vcpu_config_get_ccsidr_el1_sys_reg_values`](https://developer.apple.com/documentation/hypervisor/hv_vcpu_config_get_ccsidr_el1_sys_reg_values%28_:_:_:%29)
        - [`hv_vm_get_max_vcpu_count`](https://developer.apple.com/documentation/hypervisor/hv_vm_get_max_vcpu_count%28_:%29)
        - [`hv_vm_create`](https://developer.apple.com/documentation/hypervisor/hv_vm_create%28_:%29)
        - [`hv_vm_destroy`](https://developer.apple.com/documentation/hypervisor/hv_vm_destroy%28%29)
        - [`hv_vm_map`](https://developer.apple.com/documentation/hypervisor/hv_vm_map%28_:_:_:_:%29)
        - [`hv_vm_unmap`](https://developer.apple.com/documentation/hypervisor/hv_vm_unmap%28_:_:%29)
        - [`hv_vm_protect`](https://developer.apple.com/documentation/hypervisor/hv_vm_protect%28_:_:_:%29)
