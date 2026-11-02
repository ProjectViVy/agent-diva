fn main() {
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new("binaries");
    println!("cargo:rerun-if-changed=binaries/metadata.json");
    println!("cargo:rerun-if-changed=binaries/diva-core-x86_64-pc-windows-msvc.exe");
    let bytes = std::fs::read(root.join("metadata.json")).expect(
        "Explicit matching core artifact required: run npm run import-core -- <artifact directory>",
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&bytes).expect("core artifact metadata JSON");
    assert_eq!(metadata["schema"], 1, "unsupported artifact schema");
    assert_eq!(
        metadata["target"], "x86_64-pc-windows-msvc",
        "unsupported core target"
    );
    assert_eq!(
        metadata["rig_commit"], "810d7dfd783c59f309892bbfeb1d73dbf65d51b6",
        "unexpected Rig revision"
    );
    for (name, path) in [
        (
            "protocol.ts",
            std::path::Path::new("../src/generated/protocol.ts"),
        ),
        (
            "licenses/dependencies.json",
            std::path::Path::new("core-notices/dependencies.json"),
        ),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = std::fs::read(path)
            .expect("matching core types and notices required; import the complete artifact");
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            metadata["files"][name].as_str().expect("artifact hash"),
            "core contract/notice mismatch"
        );
    }
    assert_eq!(
        metadata["protocol_version"], 1,
        "incompatible core protocol"
    );
    let binary = std::fs::read(root.join("diva-core-x86_64-pc-windows-msvc.exe"))
        .expect("core binary required");
    assert_eq!(
        format!("{:x}", Sha256::digest(&binary)),
        metadata["files"]["diva-core-x86_64-pc-windows-msvc.exe"]
            .as_str()
            .expect("binary hash"),
        "core artifact checksum mismatch"
    );
    tauri_build::build()
}
