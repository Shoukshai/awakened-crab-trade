#!/usr/bin/env python3
"""Fetch the pinned, checksum-verified UCRT cross compiler and GTK sysroot."""
import hashlib, json, pathlib, subprocess, urllib.request

root = pathlib.Path(__file__).resolve().parents[1]
tools = root / 'dist/windows-tools'
tools.mkdir(parents=True, exist_ok=True)
compiler = 'llvm-mingw-20261104-ucrt-ubuntu-22.04-x86_64'
archive = compiler + '.tar.xz'

def fetch(url, path, sha):
    if not path.exists():
        temporary = path.with_suffix(path.suffix + '.part')
        urllib.request.urlretrieve(url, temporary)
        temporary.replace(path)
    if hashlib.sha256(path.read_bytes()).hexdigest() != sha:
        raise SystemExit(f'Checksum mismatch: {path}')

fetch(f'https://github.com/mstorsjo/llvm-mingw/releases/download/20261104/{archive}',
      tools / archive, '919380403de87dca3c581c2053230cd298af5e891286628ba0613b7fed370926')
if not (tools / compiler).is_dir():
    subprocess.run(['tar', '-xJf', str(tools / archive), '-C', str(tools)], check=True)
lock = json.loads((root / 'packaging/windows/runtime-lock.json').read_text())
packages = tools / 'packages'
packages.mkdir(exist_ok=True)
sysroot = tools / 'sysroot'
sysroot.mkdir(exist_ok=True)
for package in lock['packages']:
    path = packages / package['file']
    fetch(lock['repository'] + package['file'], path, package['sha256'])
    entries = subprocess.check_output(['tar', '--zstd', '-tf', str(path)], text=True).splitlines()
    if any(pathlib.PurePosixPath(p).is_absolute() or '..' in pathlib.PurePosixPath(p).parts for p in entries):
        raise SystemExit(f'Unexpected archive paths: {path}')
    subprocess.run(['tar', '--zstd', '-xf', str(path), '-C', str(sysroot), '--exclude=.?*'], check=True)
ocr = json.loads((root / 'packaging/windows/ocr-data.json').read_text())
for url, sha, path in [
    (ocr['url'], ocr['sha256'], sysroot / 'ucrt64/share/tessdata/eng.traineddata'),
    (ocr['license_url'], ocr['license_sha256'], sysroot / 'ucrt64/share/licenses/tessdata-fast/LICENSE'),
]:
    path.parent.mkdir(parents=True, exist_ok=True)
    fetch(url, path, sha)
subprocess.run(['rustup', 'target', 'add' , 'x86_64-pc-windows-gnullvm'], check=True)
print('Pinned Windows build inputs ready.')
