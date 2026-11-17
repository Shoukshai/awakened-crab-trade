#!/usr/bin/env python3
"""Assemble a portable Windows ZIP, following PE imports recursively."""
import hashlib, json, pathlib, re, shutil, subprocess, tomllib, zipfile

root = pathlib.Path(__file__).resolve().parents[1]
version = tomllib.loads((root / 'Cargo.toml').read_text())['package']['version']
tools = root / 'dist/windows-tools'
llvm = tools / 'llvm-mingw-20261104-ucrt-ubuntu-22.04-x86_64'
sysroot = tools / 'sysroot/ucrt64'
output = root / f'dist/awakened-poe-trade-rs-{version}-windows-x64'
if output.exists():
    shutil.rmtree(output)
output.mkdir()
exe = root / 'target/x86_64-pc-windows-gnullvm/release/awakened-poe-trade-rs.exe'
shutil.copy2(exe, output / exe.name)
for name in ['gdk-pixbuf-query-loaders.exe']:
    shutil.copy2(sysroot / 'bin' / name, output / name)
for name in ['lib/gdk-pixbuf-2.0/2.10.0/loaders', 'share/glib-2.0/schemas',
             'share/icons', 'share/mime', 'etc/fonts']:
    shutil.copytree(sysroot / name, output / name)
subprocess.run(['glib-compile-schemas', str(output / 'share/glib-2.0/schemas')], check=True)
# Cache files contain build-machine absolute paths. Generate loaders on first run.
for cache in output.rglob('loaders.cache'):
    cache.unlink()
system = set('advapi32 bcrypt bcryptprimitives cfgmgr32 comctl32 comdlg32 crypt32 d2d1 d3d11 dbghelp dcomp'
             ' dnsapi dwmapi dwrite dxgi gdi32 gdiplus hid imm32 iphlpapi kernel32 mpr mscms'
             ' msimg32 msvcrt ncrypt netapi32 normaliz ntdll ole32 oleaut32 opengl32 powrprof'
             ' propsys psapi rpcrt4 secur32 setupapi shell32 shlwapi synchronization ucrtbase'
             ' user32 userenv usp10 uxtheme version winhttp winmm winspool wintrust ws2_32 wtsapi32'.split())
available = {p.name.lower(): p for p in (sysroot / 'bin').glob('*.dll')}
available.update({p.name.lower(): p for p in (llvm / 'x86_64-w64-mingw32/bin').glob('*.dll') if p.name.lower() not in available})
pending = list(output.rglob('*.exe')) + list(output.rglob('*.dll'))
visited, imports = set(), {}
while pending:
    pe = pending.pop()
    if pe in visited:
        continue
    visited.add(pe)
    info = subprocess.check_output([str(llvm / 'bin/llvm-readobj'), '--coff-imports', str(pe)], text=True)
    names = re.findall(r'^  Name: (.+)$', info, re.M)
    imports[str(pe.relative_to(output))] = names
    for name in names:
        name = name.lower()
        if name == 'winspool.drv' or name.removesuffix('.dll') in system or name.startswith(('api-ms-win-', 'ext-ms-win-')):
            continue
        if name not in available:
            raise SystemExit(f'Unresolved runtime import {name} from {pe}')
        target = output / available[name].name
        if not target.exists():
            shutil.copy2(available[name], target)
            pending.append(target)
licenses = output / 'licenses'
subprocess.run(['python3', str(root / 'scripts/collect-licenses.py'), str(licenses / 'rust'), 'x86_64-pc-windows-gnullvm'], check=True)
shutil.copytree(sysroot / 'share/licenses', licenses / 'runtime')
shutil.copy2(llvm / 'LICENSE.TXT', licenses / 'LLVM-MinGW-LICENSE.TXT')
shutil.copy2(root.parent / 'LICENSE', output / 'LICENSE')
shutil.copy2(root / 'packaging/windows/runtime-lock.json', licenses / 'runtime-lock.json')
shutil.copy2(root / 'docs/windows.md', output / 'LIRE-MOI.md')
(licenses / 'runtime-sources.txt').write_text('Unmodified MSYS2 UCRT64 runtime DLLs, dynamically linked.\n'
    'Exact package versions and SHA256: runtime-lock.json.\n'
    'Package build recipes and upstream source locations: https://github.com/msys2/MINGW-packages\n'
    'Compiler/runtime sources: https://github.com/mstorsjo/llvm-mingw/releases/tag/20261104\n')
(output / 'runtime-imports.json').write_text(json.dumps(imports, indent=2) + '\n')
manifest = {str(p.relative_to(output)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(output.rglob('*')) if p.is_file()}
(output / 'SHA256FILES.json').write_text(json.dumps(manifest, indent=2) + '\n')
archive = output.parent / (output.name + '.zip')
with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for p in sorted(output.rglob('*')):
        if p.is_file():
            z.write(p, str(p.relative_to(output.parent)))
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
archive.with_suffix('.zip.sha256').write_text(f'{digest}  {archive.name}\n')
print(json.dumps({'zip': str(archive), 'bytes': archive.stat().st_size, 'sha256': digest, 'bundled_PE_files': len(imports)}, indent=2))
