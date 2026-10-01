//! ABI v1 constants. `VIVY_ABI_VERSION` is frozen at 1 by the contract
//! (backend-separation-contracts.md §4). The bundled `vivy_abi.h` is the
//! authoritative copy: `VivyLibrary::load` parses it and `Host::start`
//! asserts the library's echoed version equals it. This constant is the
//! fallback frozen value used when no header is staged (e.g. pure-unit
//! builds); `build.rs` emits a warning when VIVY_ABI_HEADER is unset.

/// Frozen ABI major version sent in `VivyInit` when no runtime header
/// overrides it.
pub const ABI_VERSION: u32 = 1;
