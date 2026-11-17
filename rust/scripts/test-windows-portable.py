#!/usr/bin/env python3
"""Validate the single-file entry point in the guarded private Wine prefix."""
import fcntl, hashlib, json, os, pathlib, shutil, subprocess, time

r = pathlib.Path(__file__).resolve().parents[1]
s = json.loads((r / 'dist/windows-sandbox.json').read_text())
root = pathlib.Path(s['root'])
assert root.name.startswith('apt-windows-desktop-') and (root / 'fixture.guard').is_file()
env = os.environ.copy()
env.update(s['env'])
assert env['DISPLAY'] != os.environ.get('DISPLAY') and pathlib.Path(env['WINEPREFIX']).parent == root
lock = (root / 'test.lock').open('w')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
# Build helpers from this checkout rather than relying on previous local runs.
llvm = r / 'dist/windows-tools/llvm-mingw-20261104-ucrt-ubuntu-22.04-x86_64'
subprocess.run(['sh', str(r / 'scripts/cargo-windows.sh'), 'build', '--locked',
                '--features', 'windows-test-fixture', '--bin', 'apt-win32-fixture', '-j', '3'], check=True)
unit_env = os.environ.copy()
unit_env.update(APT_WINDOWS_WINDRES=str(llvm / 'bin/x86_64-w64-mingw32-windres'),
                CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER=str(llvm / 'bin/x86_64-w64-mingw32-clang'),
                CARGO_TARGET_DIR=str(r / 'portable/target'), RUSTFLAGS='-C target-feature=+crt-static')
subprocess.run(['cargo', 'test', '--locked', '--no-run', '--manifest-path', str(r / 'portable/Cargo.toml'),
                '--target', 'x86_64-pc-windows-gnullvm'], env=unit_env, check=True)
build = json.loads((r / 'dist/windows-portable-build.json').read_text())
folder = root / 'Un seul fichier déplacé'
folder.mkdir(exist_ok=True)
app = folder / pathlib.Path(build['exe']).name
shutil.copy2(build['exe'], app)
assert [p.name for p in folder.iterdir()] == [app.name]
wp = lambda p: 'Z:' + str(p.resolve()).replace('/', '\\')

def run(arguments, label):
    stdout, stderr = root / (label + '.out'), root / (label + '.err')
    start = time.monotonic()
    with stdout.open('w') as out, stderr.open('w') as err:
        result = subprocess.run([s['wine'], wp(app), *arguments], env=env, stdout=out, stderr=err, timeout=45)
    assert result.returncode == 0, (label, result.returncode, stderr.read_text())
    return stdout.read_text().strip(), round((time.monotonic() - start) * 1000)

def unix(path):
    if path.startswith('C:'):
        return root / 'prefix/drive_c' / path[3:].replace('\\', '/')
    if path.startswith('Z:'):
        return pathlib.Path('/' + path[3:].replace('\\', '/'))
    raise AssertionError(path)

report = {'scope': 'Single EXE under private Wine; no real game input or Steam prefix.', 'build': build}
path, report['cold_extract_ms'] = run(['--extract-only'], 'portable-cold')
cache = unix(path)
assert cache.resolve().is_relative_to(root.resolve())
manifest = json.loads((cache / 'SHA256FILES.json').read_text())
for name, digest in manifest.items():
    assert hashlib.sha256((cache / name).read_bytes()).hexdigest() == digest, name
report['extracted_file_hashes_verified'] = len(manifest)
snapshot = {p.relative_to(cache).as_posix(): p.stat().st_mtime_ns for p in cache.rglob('*') if p.is_file()}
again, report['warm_extract_ms'] = run(['--extract-only'], 'portable-warm')
assert again == path
assert snapshot == {p.relative_to(cache).as_posix(): p.stat().st_mtime_ns for p in cache.rglob('*') if p.is_file()}
report['warm_launch_does_not_rewrite_cache'] = True

# A partial cache is replaced atomically; simultaneous launches serialize recovery.
(cache / '.ready').unlink()
(cache / 'incomplete-sentinel').write_text('interrupted extraction')
handles, workers = [], []
for index in range(2):
    out = (root / f'portable-concurrent-{index}.out').open('w')
    err = (root / f'portable-concurrent-{index}.err').open('w')
    handles.extend([out, err])
    workers.append(subprocess.Popen([s['wine'], wp(app), '--extract-only'], env=env, stdout=out, stderr=err))
for worker in workers:
    assert worker.wait(timeout=45) == 0
for handle in handles:
    handle.close()
assert (cache / '.ready').read_text() == build['payload_sha256']
assert not (cache / 'incomplete-sentinel').exists()
assert not list(cache.parent.glob('.*.tmp'))
report['concurrent_launches_recover_incomplete_cache'] = True
assert [p.name for p in folder.iterdir()] == [app.name]
report['only_one_file_next_to_the_portable_exe'] = True

# Native Windows unit tests cover file locking, cache reuse and archive rejection.
unit = max((r / 'portable/target/x86_64-pc-windows-gnullvm/debug/deps').glob('awakened_poe_trade_portable-*.exe'), key=lambda p:p.stat().st_mtime_ns)
shutil.copy2(unit, root / 'portable-unit-tests.exe')
with (root / 'portable-unit.log').open('w') as out:
    result = subprocess.run([s['wine'], wp(root / 'portable-unit-tests.exe'), '--test-threads=2'], env=env, stdout=out, stderr=subprocess.STDOUT, timeout=30)
assert result.returncode == 0, (root / 'portable-unit.log').read_text()
assert '4 passed; 0 failed' in (root / 'portable-unit.log').read_text()
report['native_Windows_launcher_unit_tests'] = 4

shutil.copy2(r / 'target/x86_64-pc-windows-gnullvm/debug/apt-win32-fixture.exe', root / 'apt-win32-fixture.exe')
shutil.copy2(llvm / 'x86_64-w64-mingw32/bin/libunwind.dll', root / 'libunwind.dll')
fcntl.flock(lock, fcntl.LOCK_UN)
with (r / 'dist/windows-portable-ui.log').open('w') as out:
    subprocess.run(['python3', str(r / 'scripts/test-windows.py'), str(app)], stdout=out, stderr=subprocess.STDOUT, check=True)
report['price_checker_UI'] = json.loads((r / 'dist/windows-private-report.json').read_text())
assert [p.name for p in folder.iterdir()] == [app.name]
(r / 'dist/windows-portable-validation.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
