from __future__ import annotations

import argparse
import shutil
import tarfile
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile


ARCH_ALIASES = {
    "x86_64": "x64",
    "amd64": "x64",
    "x64": "x64",
    "arm64": "arm64",
    "aarch64": "arm64",
}


def normalize_arch(raw_arch: str) -> str:
    normalized = raw_arch.strip().lower()
    return ARCH_ALIASES.get(normalized, normalized)


def write_manifest(
    bundle_dir: Path,
    version: str,
    os_tag: str,
    arch: str,
    binary_rel: str,
    *,
    systemd_files: list[str] | None = None,
    launchd_files: list[str] | None = None,
) -> None:
    lines = [
        "Agent DiVA Headless Bundle",
        f"version={version}",
        f"os={os_tag}",
        f"arch={arch}",
        f"binary={binary_rel}",
        f"entrypoint={binary_rel} gateway run",
    ]
    if systemd_files:
        lines.append(f"systemd_files={','.join(systemd_files)}")
    if launchd_files:
        lines.append(f"launchd_files={','.join(launchd_files)}")
    manifest = bundle_dir / "bundle-manifest.txt"
    manifest.write_text("\n".join(lines) + "\n", encoding="utf-8")


def create_archive(bundle_dir: Path, output_dir: Path, os_tag: str) -> Path:
    if os_tag == "windows":
        archive_path = output_dir / f"{bundle_dir.name}.zip"
        with ZipFile(archive_path, "w", compression=ZIP_DEFLATED) as archive:
            for path in bundle_dir.rglob("*"):
                archive.write(path, path.relative_to(bundle_dir.parent))
        return archive_path

    archive_path = output_dir / f"{bundle_dir.name}.tar.gz"
    with tarfile.open(archive_path, "w:gz") as archive:
        archive.add(bundle_dir, arcname=bundle_dir.name)
    return archive_path


def main() -> None:
    parser = argparse.ArgumentParser(description="Package Agent DiVA headless artifacts for CI.")
    parser.add_argument("--binary", required=True, help="Path to the built agent-diva binary")
    parser.add_argument("--version", required=True, help="CLI package version")
    parser.add_argument("--os", required=True, dest="os_tag", help="Target operating system tag")
    parser.add_argument("--arch", required=True, help="Target architecture")
    parser.add_argument("--output-dir", required=True, help="Directory for generated archives")
    parser.add_argument("--readme", required=True, help="README template to include in the bundle")
    args = parser.parse_args()

    binary_path = Path(args.binary)
    readme_path = Path(args.readme)
    output_dir = Path(args.output_dir)

    if not binary_path.exists():
        raise FileNotFoundError(f"Binary not found: {binary_path}")
    if not readme_path.exists():
        raise FileNotFoundError(f"README template not found: {readme_path}")

    arch = normalize_arch(args.arch)
    bundle_name = f"agent-diva-{args.version}-{args.os_tag}-{arch}"
    bundle_dir = output_dir / bundle_name

    if bundle_dir.exists():
        shutil.rmtree(bundle_dir)

    bundle_dir.mkdir(parents=True, exist_ok=True)
    bin_dir = bundle_dir / "bin"
    bin_dir.mkdir(exist_ok=True)
    shutil.copy2(binary_path, bin_dir / binary_path.name)
    shutil.copy2(readme_path, bundle_dir / "README.md")

    binary_rel = f"bin/{binary_path.name}"
    systemd_files: list[str] | None = None
    launchd_files: list[str] | None = None
    if args.os_tag == "linux":
        contrib_systemd = Path("contrib/systemd")
        if contrib_systemd.exists():
            systemd_dir = bundle_dir / "systemd"
            systemd_dir.mkdir(exist_ok=True)
            for name in ("agent-diva.service", "install.sh", "uninstall.sh"):
                src = contrib_systemd / name
                if src.exists():
                    dst = systemd_dir / name
                    shutil.copy2(src, dst)
                    if name.endswith(".sh"):
                        dst.chmod(0o755)
            systemd_files = [
                "systemd/agent-diva.service",
                "systemd/install.sh",
                "systemd/uninstall.sh",
            ]
    elif args.os_tag == "macos":
        contrib_launchd = Path("contrib/launchd")
        if contrib_launchd.exists():
            launchd_dir = bundle_dir / "launchd"
            launchd_dir.mkdir(exist_ok=True)
            for name in (
                "com.agent-diva.gateway.plist",
                "install.sh",
                "uninstall.sh",
            ):
                src = contrib_launchd / name
                if src.exists():
                    dst = launchd_dir / name
                    shutil.copy2(src, dst)
                    if name.endswith(".sh"):
                        dst.chmod(0o755)
            launchd_files = [
                "launchd/com.agent-diva.gateway.plist",
                "launchd/install.sh",
                "launchd/uninstall.sh",
            ]

    write_manifest(
        bundle_dir,
        args.version,
        args.os_tag,
        arch,
        binary_rel,
        systemd_files=systemd_files,
        launchd_files=launchd_files,
    )

    archive_path = create_archive(bundle_dir, output_dir, args.os_tag)
    print(archive_path)


if __name__ == "__main__":
    main()
