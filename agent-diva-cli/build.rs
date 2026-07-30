fn main() {
    // The desktop gateway assembles several deeply nested provider, channel,
    // SQLite and HTTP stacks on its main thread. Reserve enough stack for a
    // deterministic Windows bootstrap.
    #[cfg(all(windows, target_env = "msvc"))]
    println!("cargo:rustc-link-arg=/STACK:16777216");

    #[cfg(all(windows, target_env = "gnu"))]
    println!("cargo:rustc-link-arg=-Wl,--stack,16777216");
}
