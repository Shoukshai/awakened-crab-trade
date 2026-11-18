#!/usr/bin/env python3
"""Embed the validated Windows distribution in a standalone, static launcher."""
import gzip, hashlib, json, os, pathlib, shutil, struct, subprocess, tomllib, argparse

root = pathlib.Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser();parser.add_argument('--ocr',action='store_true');options=parser.parse_args()
app_id='awakened-crab-trade-ocr' if options.ocr else 'awakened-crab-trade'
version = tomllib.loads((root / 'portable/Cargo.toml').read_text())['package']['version']
source = root / f'dist/{app_id}-{version}-windows-x64'
destination = root / f'dist/{app_id}-payload-root'
if destination.exists():
    shutil.rmtree(destination)
assert json.loads((source/'edition.json').read_text())['edition']==app_id,'Payload edition does not match launcher'
manifest = json.loads((source / 'SHA256FILES.json').read_text())
for name, digest in manifest.items():
    if hashlib.sha256((source / name).read_bytes()).hexdigest() != digest:
        raise SystemExit(f'Windows distribution was modified: {name}')
shutil.copytree(source, destination)
subprocess.run(['python3', str(root / 'scripts/collect-licenses.py'),
                str(destination / 'licenses/launcher'), 'x86_64-pc-windows-gnullvm',
                str(root / 'portable/Cargo.toml')], check=True)
shutil.copy2(root / 'docs/windows-portable.md', destination / 'README-PORTABLE.md')
files = sorted(p for p in destination.rglob('*') if p.is_file() and p.name != 'SHA256FILES.json')
manifest = {p.relative_to(destination).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
(destination / 'SHA256FILES.json').write_text(json.dumps(manifest, indent=2) + '\n')
files = sorted(p for p in destination.rglob('*') if p.is_file())
seen = set()
payload = root / f'dist/{app_id}-payload.gz'
with payload.open('wb') as output, gzip.GzipFile(filename='', mode='wb', fileobj=output, compresslevel=9, mtime=0) as archive:
    archive.write(b'APTSELF1' + struct.pack('<I', len(files)))
    for file in files:
        name = file.relative_to(destination).as_posix()
        if name.casefold() in seen:
            raise SystemExit(f'Duplicate Windows filename: {name}')
        seen.add(name.casefold())
        path = name.encode('utf-8')
        content = file.read_bytes()
        archive.write(struct.pack('<IQ', len(path), len(content)))
        archive.write(hashlib.sha256(content).digest() + path + content)
identity = hashlib.sha256(payload.read_bytes()).hexdigest()
llvm = root / 'dist/windows-tools/llvm-mingw-20261104-ucrt-ubuntu-22.04-x86_64'
env = os.environ.copy()
env.update(APT_PORTABLE_PAYLOAD=str(payload), APT_PORTABLE_ID=identity,
           APT_WINDOWS_WINDRES=str(llvm / 'bin/x86_64-w64-mingw32-windres'),
           CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER=str(llvm / 'bin/x86_64-w64-mingw32-clang'),
           CARGO_TARGET_DIR=str(root / 'portable/target'),
           RUSTFLAGS='-C target-feature=+crt-static')
subprocess.run(['cargo', 'build', '--release', '--locked', '--manifest-path', str(root / 'portable/Cargo.toml'),
                '--target', 'x86_64-pc-windows-gnullvm', *(['--features','cloud-ocr'] if options.ocr else [])], env=env, check=True)
exe = root / f'dist/{app_id}-{version}-windows-x64-portable.exe'
shutil.copy2(root / 'portable/target/x86_64-pc-windows-gnullvm/release/awakened-crab-trade-portable.exe', exe)
imports = subprocess.check_output([str(llvm / 'bin/llvm-readobj'), '--coff-imports', str(exe)], text=True)
import re
names = re.findall(r'^  Name: (.+)$', imports, re.M)
system = {'kernel32.dll', 'advapi32.dll', 'bcrypt.dll', 'ntdll.dll', 'user32.dll', 'userenv.dll',
          'ws2_32.dll', 'msvcrt.dll', 'ucrtbase.dll', 'shell32.dll', 'ole32.dll', 'synchronization.dll', 'bcryptprimitives.dll'}
unexpected = [name for name in names if name.lower() not in system and not name.lower().startswith(('api-ms-win-', 'ext-ms-win-'))]
if unexpected:
    exe.unlink()
    raise SystemExit(f'Launcher must not depend on external DLLs: {unexpected}')
digest = hashlib.sha256(exe.read_bytes()).hexdigest()
exe.with_suffix('.exe.sha256').write_text(f'{digest}  {exe.name}\n')
report = {'edition':app_id,'exe': str(exe), 'sha256': digest, 'bytes': exe.stat().st_size,
          'payload_sha256': identity, 'embedded_files': len(files), 'imports': names,
          'inner_binary_sha256': manifest['awakened-crab-trade.exe']}
(root / ('dist/windows-ocr-portable-build.json' if options.ocr else 'dist/windows-portable-build.json')).write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
