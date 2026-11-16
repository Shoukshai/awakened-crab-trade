#!/usr/bin/env python3
"""Bundle exact registry license files for the host's locked dependency graph.

Includes build/test dependencies. Two published GTK binding crates omit their
workspace MIT text; pinned upstream copies live in packaging/license-supplements.
No network access or credential lookup is performed by this script.
"""
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parents[1]
output = pathlib.Path(sys.argv[1]).resolve()
output.mkdir(parents=True, exist_ok=True)
compiler = subprocess.check_output(['rustc', '-vV'], text=True)
host = next(line.split(': ', 1)[1] for line in compiler.splitlines() if line.startswith('host: '))
metadata = json.loads(subprocess.check_output([
    'cargo', 'metadata', '--locked', '--offline', '--format-version', '1',
    '--filter-platform', host, '--manifest-path', str(root / 'Cargo.toml'),
], text=True))
nodes = {node['id']: node for node in metadata['resolve']['nodes']}
reachable = set()
pending = [metadata['resolve']['root']]
while pending:
    node = pending.pop()
    if node not in reachable:
        reachable.add(node)
        pending.extend(dep['pkg'] for dep in nodes[node]['deps'])

supplement_dir = root / 'packaging/license-supplements'
supplements = json.loads((supplement_dir / 'sources.json').read_text())
prefixes = ('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT')
index = {'host': host, 'compiler': compiler, 'includes_build_and_test_dependencies': True, 'packages': []}
for package in sorted(metadata['packages'], key=lambda package: (package['name'], package['version'])):
    if package['id'] not in reachable or package['source'] is None:
        continue
    base = pathlib.Path(package['manifest_path']).parent
    texts = []
    for entry in base.iterdir():
        if not entry.name.upper().startswith(prefixes):
            continue
        texts.extend([entry] if entry.is_file() else [file for file in entry.rglob('*') if file.is_file()])
    if package['license_file']:
        path = pathlib.Path(package['license_file'])
        path = path if path.is_absolute() else base / path
        if path.is_file() and path.resolve().is_relative_to(base.resolve()) and path not in texts:
            texts.append(path)
    extra = supplements.get(f"{package['name']}@{package['version']}")
    if not texts and extra:
        texts.append(supplement_dir / extra['file'])
    if not texts:
        raise SystemExit(f"Missing license text: {package['name']} {package['version']}")
    record = {key: package[key] for key in ('name', 'version', 'license', 'authors', 'repository')}
    record['files'] = []
    for source in sorted(set(texts)):
        relative = source.relative_to(base) if source.is_relative_to(base) else pathlib.Path(source.name)
        destination = output / f"{package['name']}-{package['version']}" / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        record['files'].append({
            'path': str(destination.relative_to(output)),
            'sha256': hashlib.sha256(destination.read_bytes()).hexdigest(),
            'source': extra['url'] if extra and source.parent == supplement_dir else f"https://crates.io/crates/{package['name']}/{package['version']}",
        })
    index['packages'].append(record)

sysroot = pathlib.Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip())
stdlib = sysroot / 'share/doc/rust/COPYRIGHT-library.html'
if not stdlib.is_file():
    raise SystemExit('Rust standard-library notice is missing from the build toolchain: COPYRIGHT-library.html')
stdlib_dest = output / 'rust-standard-library/COPYRIGHT-library.html'
stdlib_dest.parent.mkdir(exist_ok=True)
shutil.copyfile(stdlib, stdlib_dest)
index['standard_library'] = {'path': str(stdlib_dest.relative_to(output)), 'sha256': hashlib.sha256(stdlib_dest.read_bytes()).hexdigest()}
(output / 'index.json').write_text(json.dumps(index, indent=2, ensure_ascii=False) + '\n')
lines = ['# Rust dependency notices', '', f'Locked host graph: `{host}`. Includes build and test dependencies.', '',
         'The standard-library notice is copied from the exact build toolchain.',
         'GTK and other system shared libraries are supplied by the distribution.', '']
for package in index['packages']:
    lines.append(f"- {package['name']} {package['version']} — {package['license']}")
    lines.extend(f"  - [{file['path']}]({file['path']})" for file in package['files'])
(output / 'README.md').write_text('\n'.join(lines) + '\n')
print(f"Bundled license texts for {len(index['packages'])} host-graph crates and the Rust standard library")
