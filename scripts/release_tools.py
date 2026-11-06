"""Versioned, explicit-allowlist packaging; never include local settings or history."""
from __future__ import annotations
import argparse
import hashlib
import html.parser
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tempfile
import tomllib

ROOT=Path(__file__).resolve().parents[1]
TARGET="x86_64-pc-windows-msvc"
NOTICES="third-party/THIRD-PARTY-NOTICES.txt"

def version(root: Path=ROOT) -> str:
    value=tomllib.loads((root/"Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+",value) or any(int(n)>65535 for n in value.split(".")):
        raise ValueError("Use a three-part release version with components between 0 and 65535.")
    return value

def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def fingerprint(root: Path=ROOT) -> str:
    paths=[root/"Cargo.toml",root/"Cargo.lock",root/"build.rs",root/".cargo/config.toml",root/"docs/user-guide.md",root/"LICENSE",root/"rust-toolchain.toml",root/NOTICES]
    paths+=list((root/"src").rglob("*.rs"))
    paths+=list((root/"locales").glob("*.json"))
    paths+=list((root/"scripts").glob("*.py"))+list((root/"scripts").glob("*.ps1"))+[root/"scripts/requirements-docs.txt"]
    paths += [p for p in (root/"third-party/license-overrides").rglob("*") if p.is_file()]
    digest=hashlib.sha256()
    for path in sorted(set(paths)):
        if path.exists():
            digest.update(path.relative_to(root).as_posix().encode());digest.update(b"\0");digest.update(path.read_bytes());digest.update(b"\0")
    return digest.hexdigest()

def release_dir(root: Path=ROOT) -> Path:
    return root/"dist"/f"AttentionFocus-{version(root)}"

def check_version(root: Path=ROOT) -> None:
    released=release_dir(root)/"release.json"
    if released.exists():
        if json.loads(released.read_bytes())["source_fingerprint"]!=fingerprint(root):
            raise ValueError("Production inputs changed for an already packaged version. Bump Cargo.toml version before publishing.")

class TextOnly(html.parser.HTMLParser):
    def __init__(self): super().__init__();self.parts=[];self.hidden=0
    def handle_starttag(self,tag,attrs):
        if tag in ("script","style"):self.hidden+=1
    def handle_endtag(self,tag):
        if tag in ("script","style"):self.hidden-=1
    def handle_data(self,data):
        if not self.hidden and data.strip():self.parts.append(data.strip())

def generate_notices() -> str:
    metadata=json.loads(subprocess.check_output(["cargo","metadata","--offline","--locked","--format-version","1","--filter-platform",TARGET],cwd=ROOT))
    nodes={n["id"]:n for n in metadata["resolve"]["nodes"]};packages={p["id"]:p for p in metadata["packages"]}
    root=metadata["resolve"]["root"];seen=set();todo=[root]
    while todo:
        item=todo.pop()
        if item in seen:continue
        seen.add(item)
        todo.extend(dep["pkg"] for dep in nodes[item]["deps"] if any(k["kind"] is None for k in dep["dep_kinds"]))
    overrides=json.loads((ROOT/"third-party/license-overrides/sources.json").read_text(encoding="utf-8"))
    license_text=(ROOT/"LICENSE").read_text(encoding="utf-8").strip()
    parts=["Attention Focus - License and Third-Party Notices", "Attention Focus is distributed under the following license:", license_text, "\n"+"="*76, "The licenses below cover the listed third-party components only. No application runtime files are supplied by this text."]
    count=0
    for item in sorted(seen,key=lambda x:(packages[x]["name"],packages[x]["version"])):
        if item==root:continue
        p=packages[item];directory=Path(p["manifest_path"]).parent
        files=[f for f in directory.iterdir() if f.is_file() and f.name.upper().startswith(("LICENSE","LICENCE","COPYING","NOTICE"))]
        if p.get("license_file"):files.append(Path(p["license_file"]))
        origin="Published crate license files"
        if not files:
            entry=next(((name,data) for name,data in overrides.items() if p["name"] in data["packages"]),None)
            if not entry:raise ValueError(f"Missing license input for {p['name']} {p['version']}")
            name,data=entry
            actual=json.loads((directory/".cargo_vcs_info.json").read_text(encoding="utf-8"))["git"]["sha1"]
            expected=data.get("package_commits",{}).get(p["name"],data["commit"])
            if actual!=expected:raise ValueError(f"License override must be refreshed for {p['name']} {p['version']}")
            files=list((ROOT/"third-party/license-overrides"/name).glob("LICENSE*"))
            origin=f"{data['repository']} at {data['commit']}"
            if data.get("provenance"):origin+=" | "+data["provenance"]
        # Font-specific OFL/UFL/Hack/icon notices are not always named LICENSE.
        if p["name"]=="epaint_default_fonts":files+=list((directory/"fonts").glob("*.txt"))
        parts.extend(["\n"+"="*76,f"{p['name']} {p['version']} | {p.get('license') or 'See license text'}",f"Repository: {p.get('repository') or 'See crate metadata'}",f"Authors: {authors(p)}",f"License source: {origin}"])
        for path in sorted(set(files)):
            if not path.is_file():raise ValueError(f"License file is missing: {p['name']} / {path.name}")
            parts.extend([f"\n--- {path.name} ---",path.read_text(encoding="utf-8-sig").replace("\r\n","\n")])
        count+=1
    sysroot=Path(subprocess.check_output(["rustc","--print","sysroot"],text=True).strip())
    rust_notice=sysroot/"share/doc/rust/COPYRIGHT-library.html"
    if not rust_notice.is_file():raise ValueError("Rust standard-library notices are missing from the installed toolchain.")
    parser=TextOnly();parser.feed(rust_notice.read_text(encoding="utf-8"))
    parts.extend(["\n"+"="*76,"Microsoft C and C++ runtime libraries",(ROOT/"third-party/microsoft-runtime-notice.txt").read_text(encoding="utf-8").strip()])
    parts.extend(["\n"+"="*76,"Rust standard library",subprocess.check_output(["rustc","--version"],text=True).strip(),"\n".join(parser.parts)])
    print(f"Notices generated for {count} Windows runtime packages, bundled fonts, and Rust standard library.")
    return notice_text(parts)

def authors(package: dict) -> str:
    """Many crates no longer list authors; say so instead of leaving the field empty."""
    return ", ".join(package.get("authors") or []) or "Not listed in the crate manifest"

def notice_text(parts: list[str]) -> str:
    """Joins the notices into one LF text without spaces or tabs at line ends. Published license
    files sometimes have them; the wording is otherwise kept exactly as published."""
    return "\n".join(line.rstrip(" \t") for line in "\n".join(parts).split("\n"))+"\n"

def update_notices() -> None:
    """The EXE embeds this tracked file, so it must be current before compiling."""
    path=ROOT/NOTICES;text=generate_notices()
    if not path.exists() or path.read_bytes()!=text.encode("utf-8"):
        path.write_bytes(text.encode("utf-8"))
        print(f"{NOTICES} changed; commit it with the dependency change.")

def check_notices() -> None:
    if (ROOT/NOTICES).read_bytes()!=generate_notices().encode("utf-8"):
        raise ValueError(f"{NOTICES} is out of date. Run: python scripts/release_tools.py update-notices")

def safe_file(path: Path,root: Path) -> None:
    if not path.absolute().is_relative_to(root.absolute()) or not path.resolve().is_relative_to(root.resolve()):raise ValueError("Package path is outside the project.")
    current=path
    while current!=root:
        if current.is_symlink() or (hasattr(current,"is_junction") and current.is_junction()):raise ValueError(f"Linked package path is unsupported: {current.name}")
        current=current.parent

def payload(root: Path=ROOT) -> list[str]:
    """What a release publishes: the program, which holds its own notices, and its guide."""
    return ["attention-focus.exe",f"User-Guide-{version(root)}.pdf"]

def manifest(root: Path=ROOT) -> dict:
    v=version(root);directory=root/"dist/AttentionFocus"
    files={}
    for name in payload(root):
        path=directory/name;safe_file(path,root)
        if not path.is_file() or path.stat().st_size==0:raise ValueError(f"Required production file is missing or empty: {name}")
        files[name]={"sha256":sha(path),"bytes":path.stat().st_size}
    built=json.loads((root/"target/production-build.json").read_bytes())
    if built!={"version":v,"source_fingerprint":fingerprint(root),"exe_sha256":files["attention-focus.exe"]["sha256"]}:
        raise ValueError("Production EXE/build inputs do not match the build stamp. Run scripts/build.ps1.")
    return {"format":2,"product":"Attention Focus","version":v,"file_version":v+".0","platform":"windows-x64","source_fingerprint":built["source_fingerprint"],"files":files}

def stamp(root: Path=ROOT) -> None:
    data={"version":version(root),"source_fingerprint":fingerprint(root),"exe_sha256":sha(root/"dist/AttentionFocus/attention-focus.exe")}
    (root/"target/production-build.json").write_text(json.dumps(data,sort_keys=True)+"\n",encoding="utf-8",newline="\n")

def pack(root: Path=ROOT) -> Path:
    """Builds the release folder aside and swaps it in, so a failure keeps the previous one."""
    check_version(root)
    data=manifest(root);source=root/"dist/AttentionFocus";target=release_dir(root)
    safe_file(target,root)
    allowed={*data["files"],"release.json"}
    if target.exists():
        unexpected=sorted(p.name for p in target.iterdir() if p.name not in allowed)
        if unexpected:raise ValueError(f"Unexpected files in the release folder: {unexpected}")
    staging=Path(tempfile.mkdtemp(prefix="release-",dir=root/"target"))
    previous=target.with_name(target.name+".previous")
    try:
        for name,entry in data["files"].items():
            shutil.copyfile(source/name,staging/name)
            if sha(staging/name)!=entry["sha256"]:raise ValueError("Copied file hash mismatch.")
        (staging/"release.json").write_bytes((json.dumps(data,indent=2,sort_keys=True)+"\n").encode())
        replacing=target.exists()
        if replacing:
            # An interrupted swap can leave this behind, and Windows will not replace it.
            if previous.exists():shutil.rmtree(previous)
            os.replace(target,previous)
        try:
            os.replace(staging,target)
        except OSError:
            if replacing:os.replace(previous,target)
            raise
        if replacing:shutil.rmtree(previous)
    finally:
        shutil.rmtree(staging,ignore_errors=True)
    print(target)
    return target

def verify(root: Path=ROOT) -> None:
    current=manifest(root);target=release_dir(root)
    data=json.loads((target/"release.json").read_bytes())
    if data!=current:raise ValueError("Release folder is stale. Run scripts/build.ps1.")
    if {p.name for p in target.iterdir()}!={*data["files"],"release.json"}:raise ValueError("Release folder contents failed validation.")
    for name,entry in data["files"].items():
        if sha(target/name)!=entry["sha256"]:raise ValueError("Release file hash mismatch.")
    print("Release folder verified: exact allowlist, current source fingerprint, file hashes, and manifest.")

def verify_icons(root: Path=ROOT) -> None:
    candidates=[p for mode in ("release","debug") for p in (root/"target"/mode/"build").glob("attention-focus-*/out/icon-version.txt") if p.read_text(encoding="utf-8").strip()==version(root)]
    if not candidates:raise ValueError("Current generated icon catalog is missing.")
    directory=max(candidates,key=lambda p:p.stat().st_mtime_ns).parent
    for executable,icon_name in [(root/"dist/AttentionFocus/attention-focus.exe","app"),(root/"target/debug/attention-focus.exe","app-development")]:
        binary=executable.read_bytes();ico=(directory/f"{icon_name}.ico").read_bytes()
        count=struct.unpack_from("<H",ico,4)[0]
        for index in range(count):
            length,offset=struct.unpack_from("<II",ico,6+index*16+8)
            if ico[offset:offset+length] not in binary:raise ValueError(f"Executable icon resource mismatch: {executable.name} / {icon_name}")
        print(f"Verified {count} neutral icon resource sizes: {icon_name}")

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("command",choices=["version","fingerprint","check-version","update-notices","check-notices","stamp","package","verify","verify-icons"])
    args=parser.parse_args()
    if args.command=="version":print(version())
    elif args.command=="fingerprint":print(fingerprint())
    elif args.command=="check-version":check_version()
    elif args.command=="update-notices":update_notices()
    elif args.command=="check-notices":check_notices()
    elif args.command=="package":pack()
    elif args.command=="stamp":stamp()
    elif args.command=="verify-icons":verify_icons()
    else:verify()
