#!/usr/bin/env python3
"""Build an open-source OS keyboard layout for the private Wine tests only."""
import hashlib, json, pathlib, subprocess, urllib.request

root = pathlib.Path(__file__).resolve().parents[1]
destination = root / 'dist/windows-tools/reactos-test-layout'
destination.mkdir(parents=True, exist_ok=True)
revision = '344ebfe594e4cf6a498bac08105e1996a03a2c61'
sources = [
    ('dll/keyboard/kbdus/kbdus.c', 'kbdus.c', 'f587d6bc50ecb52ed0ece4ea86437e539c2a9e9a357555f533ef6752c3e0fb75'),
    ('sdk/include/ndk/kbd.h', 'ndk/kbd.h', 'b1ee3fe0161779282d2575e984d5cb82ae0190b41a147bb6f4d8dcbbfc978f59'),
]
records = []
for source, name, digest in sources:
    url = f'https://raw.githubusercontent.com/reactos/reactos/{revision}/{source}'
    file = destination / name
    file.parent.mkdir(parents=True, exist_ok=True)
    data = file.read_bytes() if file.exists() else urllib.request.urlopen(url).read()
    if hashlib.sha256(data).hexdigest() != digest:
        raise SystemExit(f'Unexpected layout source: {source}')
    file.write_bytes(data)
    records.append({'url': url, 'file': name, 'sha256': digest})
(destination / 'sources.json').write_text(json.dumps({
    'purpose': 'Test-only US keyboard layout for private Wine prefix; never shipped in Windows ZIP',
    'license': 'LGPL-2.1-or-later', 'sources': records,
}, indent=2) + '\n')
compiler = root / 'dist/windows-tools/llvm-mingw-20261104-ucrt-ubuntu-22.04-x86_64/bin/x86_64-w64-mingw32-clang'
subprocess.run([str(compiler), '-shared', '-I', str(destination), str(destination / 'kbdus.c'),
                '-o', str(destination / 'apt-test-kbdus.dll')], check=True)
print('Private Wine test keyboard layout ready; production uses Windows OS layouts.')
