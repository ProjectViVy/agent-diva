fn main() {
    // Windows default PE stack (1 MiB) is too small for turso/simsimd paths used by
    // Mentle. Raise the main-thread reserve so gateway bootstrap and CLI entry
    // points do not STATUS_STACK_OVERFLOW after MemtleToolkit is live.
    #[cfg(all(windows, target_env = "msvc"))]
    {
        println!("cargo:rustc-link-arg=/STACK:16777216");
    }
    #[cfg(all(windows, target_env = "gnu"))]
    {
        println!("cargo:rustc-link-arg=-Wl,--stack,16777216");
    }
}
