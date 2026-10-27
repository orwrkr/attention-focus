"""Versioned, explicit-allowlist packaging; never include local settings or history."""
from __future__ import annotations
import argparse
import hashlib
import html.parser
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import tempfile
import tomllib
import zipfile

ROOT=Path(__file__).resolve().parents[1]
TARGET="x86_64-pc-windows-msvc"

def version(root: Path=ROOT) -> str:
    value=tomllib.loads((root/"Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+",value) or any(int(n)>65535 for n in value.split(".")):
        raise ValueError("Use a three-part release version with components between 0 and 65535.")
    return value

def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def fingerprint(root: Path=ROOT) -> str:
    paths=[root/"Cargo.toml",root/"Cargo.lock",root/"build.rs",root/".cargo/config.toml",root/"docs/user-guide.md"]
    paths+=list((root/"src").rglob("*.rs"))
    paths+=list((root/"scripts").glob("*.py"))+list((root/"scripts").glob("*.ps1"))+[root/"scripts/requirements-docs.txt"]
    paths += [p for p in (root/"third-party/license-overrides").rglob("*") if p.is_file()]
    digest=hashlib.sha256()
    for path in sorted(set(paths)):
        if path.exists():
            digest.update(path.relative_to(root).as_posix().encode());digest.update(b"\0");digest.update(path.read_bytes());digest.update(b"\0")
    return digest.hexdigest()

def archive_path(root: Path=ROOT) -> Path:
    return root/"dist"/f"AttentionFocus-{version(root)}-windows-x64.zip"

def check_version(root: Path=ROOT) -> None:
    archive=archive_path(root)
    if archive.exists():
        with zipfile.ZipFile(archive) as z:
            old=json.loads(z.read(f"AttentionFocus-{version(root)}/release.json"))
        if old["source_fingerprint"]!=fingerprint(root):
            raise ValueError("Production inputs changed for an already packaged version. Bump Cargo.toml version before publishing.")

class TextOnly(html.parser.HTMLParser):
    def __init__(self): super().__init__();self.parts=[];self.hidden=0
    def handle_starttag(self,tag,attrs):
        if tag in ("script","style"):self.hidden+=1
    def handle_endtag(self,tag):
        if tag in ("script","style"):self.hidden-=1
    def handle_data(self,data):
        if not self.hidden and data.strip():self.parts.append(data.strip())

def notices(output: Path) -> None:
    metadata=json.loads(subprocess.check_output(["cargo","metadata","--offline","--locked","--format-version","1","--filter-platform",TARGET],cwd=ROOT))
    nodes={n["id"]:n for n in metadata["resolve"]["nodes"]};packages={p["id"]:p for p in metadata["packages"]}
    root=metadata["resolve"]["root"];seen=set();todo=[root]
    while todo:
        item=todo.pop()
        if item in seen:continue
        seen.add(item)
        todo.extend(dep["pkg"] for dep in nodes[item]["deps"] if any(k["kind"] is None for k in dep["dep_kinds"]))
    overrides=json.loads((ROOT/"third-party/license-overrides/sources.json").read_text(encoding="utf-8"))
    parts=[f"Attention Focus {version()} - Third-Party Notices", "These licenses cover the listed components only. No application runtime files are supplied by this text."]
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
        parts.extend(["\n"+"="*76,f"{p['name']} {p['version']} | {p.get('license') or 'See license text'}",f"Repository: {p.get('repository') or 'See crate metadata'}",f"Authors: {', '.join(p.get('authors',[]))}",f"License source: {origin}"])
        for path in sorted(set(files)):
            if not path.is_file():raise ValueError(f"License file is missing: {p['name']} / {path.name}")
            parts.extend([f"\n--- {path.name} ---",path.read_text(encoding="utf-8-sig").replace("\r\n","\n")])
        count+=1
    sysroot=Path(subprocess.check_output(["rustc","--print","sysroot"],text=True).strip())
    rust_notice=sysroot/"share/doc/rust/COPYRIGHT-library.html"
    if not rust_notice.is_file():raise ValueError("Rust standard-library notices are missing from the installed toolchain.")
    parser=TextOnly();parser.feed(rust_notice.read_text(encoding="utf-8"))
    parts.extend(["\n"+"="*76,"Rust standard library",subprocess.check_output(["rustc","--version"],text=True).strip(),"\n".join(parser.parts)])
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text("\n".join(parts)+"\n",encoding="utf-8",newline="\n")
    print(f"Notices generated for {count} Windows runtime packages, bundled fonts, and Rust standard library.")

def safe_file(path: Path,root: Path) -> None:
    if not path.absolute().is_relative_to(root.absolute()) or not path.resolve().is_relative_to(root.resolve()):raise ValueError("Package path is outside the project.")
    current=path
    while current!=root:
        if current.is_symlink() or (hasattr(current,"is_junction") and current.is_junction()):raise ValueError(f"Linked package path is unsupported: {current.name}")
        current=current.parent

def manifest(root: Path=ROOT) -> dict:
    v=version(root);directory=root/"dist/AttentionFocus"
    names=["attention-focus.exe",f"User-Guide-{v}.pdf","THIRD-PARTY-NOTICES.txt"]
    files={}
    for name in names:
        path=directory/name;safe_file(path,root)
        if not path.is_file() or path.stat().st_size==0:raise ValueError(f"Required production file is missing or empty: {name}")
        files[name]={"sha256":sha(path),"bytes":path.stat().st_size}
    built=json.loads((root/"target/production-build.json").read_bytes())
    if built!={"version":v,"source_fingerprint":fingerprint(root),"exe_sha256":files["attention-focus.exe"]["sha256"]}:
        raise ValueError("Production EXE/build inputs do not match the build stamp. Run scripts/build.ps1.")
    return {"format":1,"product":"Attention Focus","version":v,"file_version":v+".0","platform":"windows-x64","source_fingerprint":built["source_fingerprint"],"files":files}

def stamp(root: Path=ROOT) -> None:
    data={"version":version(root),"source_fingerprint":fingerprint(root),"exe_sha256":sha(root/"dist/AttentionFocus/attention-focus.exe")}
    (root/"target/production-build.json").write_text(json.dumps(data,sort_keys=True)+"\n",encoding="utf-8",newline="\n")

def pack(root: Path=ROOT) -> Path:
    check_version(root)
    data=manifest(root);v=data["version"];directory=root/"dist/AttentionFocus";archive=archive_path(root)
    safe_file(archive,root);safe_file(directory/"release.json",root)
    content=(json.dumps(data,indent=2,sort_keys=True)+"\n").encode()
    fd,temporary=tempfile.mkstemp(prefix="package-",suffix=".zip",dir=root/"target")
    os.close(fd)
    try:
        with zipfile.ZipFile(temporary,"w",compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
            for name in sorted(data["files"]):
                info=zipfile.ZipInfo(f"AttentionFocus-{v}/{name}",date_time=(2020,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
                z.writestr(info,(directory/name).read_bytes())
            info=zipfile.ZipInfo(f"AttentionFocus-{v}/release.json",date_time=(2020,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,content)
        with zipfile.ZipFile(temporary) as z:
            if z.testzip():raise ValueError("Archive integrity check failed.")
            expected={f"AttentionFocus-{v}/{name}" for name in [*data["files"],"release.json"]}
            if set(z.namelist())!=expected:raise ValueError("Unexpected archive members.")
            for name,entry in data["files"].items():
                if hashlib.sha256(z.read(f"AttentionFocus-{v}/{name}")).hexdigest()!=entry["sha256"]:raise ValueError("Archive hash mismatch.")
        os.replace(temporary,archive)
        (directory/"release.json").write_bytes(content)
    finally:
        Path(temporary).unlink(missing_ok=True)
    print(archive)
    return archive

def verify(root: Path=ROOT) -> None:
    current=manifest(root)
    with zipfile.ZipFile(archive_path(root)) as z:
        data=json.loads(z.read(f"AttentionFocus-{version(root)}/release.json"))
        if data!=current:raise ValueError("Production archive is stale. Rebuild the package.")
        expected={f"AttentionFocus-{version(root)}/{name}" for name in [*data["files"],"release.json"]}
        if set(z.namelist())!=expected or z.testzip():raise ValueError("Archive contents failed validation.")
        for name,entry in data["files"].items():
            if hashlib.sha256(z.read(f"AttentionFocus-{version(root)}/{name}")).hexdigest()!=entry["sha256"]:raise ValueError("Archive hash mismatch.")
    if json.loads((root/"dist/AttentionFocus/release.json").read_bytes())!=current:raise ValueError("Loose release manifest is stale.")
    print("Production ZIP verified: exact allowlist, current source fingerprint, member hashes, and release manifest.")

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
    parser=argparse.ArgumentParser();parser.add_argument("command",choices=["version","fingerprint","check-version","notices","stamp","package","verify","verify-icons"]);parser.add_argument("--output",type=Path)
    args=parser.parse_args()
    if args.command=="version":print(version())
    elif args.command=="fingerprint":print(fingerprint())
    elif args.command=="check-version":check_version()
    elif args.command=="notices":
        if args.output is None:parser.error("notices requires --output")
        notices(args.output)
    elif args.command=="package":pack()
    elif args.command=="stamp":stamp()
    elif args.command=="verify-icons":verify_icons()
    else:verify()
