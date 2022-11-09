<p align="center">
  <b style="font-size: 2em">APPLEVISOR</b>
  <br/>
  <span style="font-size: 1.5em">Rust bindings for the Apple Silicon Hypervisor Framework</b>
</p>

<hr/>

<p align="center">
  <img src="https://img.shields.io/github/license/impalabs/applevisor?style=for-the-badge&color=ff9900" alt="shields.io license" />
  <img src="https://img.shields.io/github/v/release/impalabs/applevisor?style=for-the-badge&color=f38700" alt="shields.io version" />
  <img src="https://img.shields.io/badge/platform-MacOS%20on%20Apple%20Silicon-e77600?style=for-the-badge" alt="shields.io platform" />
  <br/>
  <img src="https://img.shields.io/badge/rust-nightly-da6500?style=for-the-badge" alt="shields.io rust version" />
  <a href="https://crates.io/crates/applevisor"><img src="https://img.shields.io/crates/v/applevisor?color=cd5300&style=for-the-badge" alt="shields.io crates.io" /></a>
  <a href="https://docs.rs/applevisor"><img src="https://img.shields.io/badge/docs.rs-rustdoc-bf4200?style=for-the-badge" alt="shields.io crates.io" /></a>
</p>

<hr/>

## Table of contents

 * [Getting Started](#getting-started)
   * [Self-Signed Binaries and Hypervisor Entitlement](#self-signed-binaries-and-hypervisor-entitlement)
   * [Compilation Workflow](#compilation-workflow)
 * [Documentation](#documentation)
 * [Example](#example)
 * [Running the Tests](#running-the-tests)
 * [Author](#author)


This library can be used to build Rust applications leveraging the `Hypervisor` framework on Apple Silicon. It was mainly built for [Hyperpom](https://github.com/impalabs/hyperpom), but it can serve more general purposes.

## Getting Started

### Self-Signed Binaries and Hypervisor Entitlement

To be able to reach the Hypervisor Framework, a binary executable has to have been granted the [hypervisor entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com_apple_security_hypervisor).

#### Certificate Chain

To add this entitlement to your project, you'll first need a certificate chain to sign your binaries, which can be created by following the instructions below.

 - Open the *Keychain Access* application.
 - Go to **Keychain Access** > **Certificate Assistant** > **Create a Certificate**.
 - Fill out the **Name** field, this value will be used later on to identify the certificate we want to sign with and will be referred to as `${CERT_NAME}`.
 - Set **Identity Type** to `Self-Signed Root`.
 - Set **Certificate Type** to `Code Signing`.
 - Click on **Create**.

You can now sign binaries and add entitlements using the following command:

```
codesign --entitlements entitlements.xml -s ${CERT_NAME} /path/to/binary
```

**Note:** The `entitlements.xml` file is available at the root of the Applevisor repository.

### Compilation Workflow

Create a Rust project and add Applevisor as a dependency in `Cargo.toml`. You can either pull it from [crates.io](TODO) ...

```toml
# Check which version is the latest, this part of the README might not be updated
# in future releases.
applevisor = "0.1.0"
```

... or directly from the GitHub repository.

```toml
applevisor = { git="https://github.com/impalabs/applevisor", branch="master" }
```

Create a file called `entitlements.txt` in the project's root directory and add the following:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.hypervisor</key>
    <true/>
</dict>
</plist>
```

Write code and then build the project.

```
cargo build --release
```

Sign the binary and grant the hypervisor entitlement.

```
codesign --entitlements entitlements.xml -s ${CERT_NAME} target/release/${PROJECT_NAME}
```

Run the binary.

```
target/release/${PROJECT_NAME}
```

## Documentation

The documentation can be generated and opened using:

```
cargo doc --open
```

## Example

The following example:

 * creates a virtual machine for the current process;
 * creates a virtual CPU;
 * enables the hypervisor's debug features to be able to use breakpoints later on;
 * creates a physical memory mapping of 0x1000 bytes and maps it at address 0x10000 with RWX
   permissions;
 * writes the instructions `mov x0, #0x42; brk #0;` at address 0x10000;
 * sets PC to 0x10000;
 * starts the vCPU and runs our program;
 * returns when it encounters the breakpoint.

```rust
use applevisor::*;

fn main() {
    // Creates a new virtual machine. There can be one, and only one, per process. Operations
    // on the virtual machine remains possible as long as this object is valid.
    let _vm = VirtualMachine::new().unwrap();

    // Creates a new virtual CPU. This object abstracts operations that can be performed on
    // CPUs, such as starting and stopping them, changing their registers, etc.
    let vcpu = Vcpu::new().unwrap();

    // Enables debug features for the hypervisor. This is optional, but it might be required
    // for certain features to work, such as breakpoints.
    assert!(vcpu.set_trap_debug_exceptions(true).is_ok());
    assert!(vcpu.set_trap_debug_reg_accesses(true).is_ok());

    // Creates a mapping object that represents a 0x10000-byte physical memory range.
    let mut mem = Mapping::new(0x1000).unwrap();

    // This mapping needs to be mapped to effectively allocate physical memory for the guest.
    // Here we map the map the region at address 0x10000 and set the permissions to
    // Read-Write-Execute.
    assert_eq!(mem.map(0x10000, MemPerms::RWX), Ok(()));

    // Writes a `mov x0, #0x42` instruction at address 0x10000.
    assert_eq!(mem.write_dword(0x10000, 0xd2800840), Ok(4));
    // Writes a `brk #0` instruction at address 0x10004.
    assert_eq!(mem.write_dword(0x10004, 0xd4200000), Ok(4));

    // Sets PC to 0x10000.
    assert!(vcpu.set_reg(Reg::PC, 0x10000).is_ok());

    // Starts the Vcpu. It will execute our mov and breakpoint instructions before stopping.
    assert!(vcpu.run().is_ok());

    // The *exit information* can be used to used to retrieve different pieces of
    // information about the CPU exit status (e.g. exception type, fault address, etc.).
    let _exit_info = vcpu.get_exit_info();

    // If everything went as expected, the value in X0 is 0x42.
    assert_eq!(vcpu.get_reg(Reg::X0), Ok(0x42));
}
```

Feel free to also have a look at the [Hyperpom](https://github.com/impalabs/hyperpom) project's source code for a real-life example of how these bindings are used.

## Running the Tests

To run tests using the `Makefile` provided with the project, you'll first need to install [`jq`](https://stedolan.github.io/jq/download/). You can do so using `brew`:

```
brew install jq
```

You can then run the tests with the provided `Makefile` using the following command:

```
CERT_KEYCHAIN=${CERT_NAME} make tests
```

## Author

* [**Maxime Peterlin**](https://twitter.com/lyte__) - hyperpom@impalabs.com
