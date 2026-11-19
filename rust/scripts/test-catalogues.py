#!/usr/bin/env python3
"""Exercise catalogue recovery on both Windows editions in the guarded Wine prefix."""
import fcntl, hashlib, json, os, pathlib, subprocess, sys, time

r = pathlib.Path(__file__).resolve().parents[1]
s = json.loads((r / 'dist/windows-sandbox.json').read_text())
root = pathlib.Path(s['root'])
assert root.name.startswith('apt-windows-desktop-') and (root / 'fixture.guard').is_file()
lock = (root / 'test.lock').open('w')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
env = os.environ.copy()
env.update(s['env'])
assert env['DISPLAY'] != os.environ.get('DISPLAY')
assert pathlib.Path(env['WINEPREFIX']).parent == root
env.update(HTTPS_PROXY='http://127.0.0.1:9', HTTP_PROXY='http://127.0.0.1:9')
wp = lambda p: 'Z:' + str(pathlib.Path(p).resolve()).replace('/', '\\')

def unix(p):
    if p.startswith('Z:'):
        result = pathlib.Path('/' + p[3:].replace('\\', '/'))
    elif p.startswith('C:'):
        result = root / 'prefix/drive_c' / p[3:].replace('\\', '/')
    else:
        raise AssertionError(p)
    assert result.resolve().is_relative_to(root.resolve())
    return result

report = {'scope': 'Guarded private Wine prefix, offline references, no game input', 'cases': []}
for argument in sys.argv[1:]:
    app = pathlib.Path(argument).resolve()
    assert app.is_file()
    env['LOCALAPPDATA'] = wp(root / 'catalogue-library-tests')
    env['APPDATA'] = wp(root / 'config')

    def run(offline=True):
        start = time.monotonic()
        p = subprocess.run([s['wine'], wp(app), 'catalogue', *(['--offline'] if offline else [])],
                           env=env, capture_output=True, text=True, timeout=30)
        assert p.returncode == 0, (p.returncode, p.stdout, p.stderr)
        return json.loads(p.stdout), round((time.monotonic() - start) * 1000)

    data, cold = run()
    directory = unix(data['directory'])
    active = directory / 'active.json'
    before = active.read_bytes()
    manifest = json.loads(before)
    generation = directory / manifest['generation']
    for name, key in [('items.json', 'items_sha256'), ('mods.json', 'mods_sha256')]:
        assert hashlib.sha256((generation / name).read_bytes()).hexdigest() == manifest[key]
    assert data['items'] >= 6000 and data['modifier_records'] >= 18000
    again, warm = run()
    assert data['directory'] == again['directory'] and active.read_bytes() == before

    (generation / 'mods.json').write_bytes(b'[]')
    recovered, repair = run()
    current = json.loads(active.read_text())
    assert current['generation'] != manifest['generation']
    assert recovered['items'] == data['items']
    assert len(json.loads((directory / current['generation'] / 'mods.json').read_text())) >= 18000
    # A fresh cached snapshot must not attempt a request through the dead proxy.
    current['checked_at'] = int(time.time())
    active.write_text(json.dumps(current))
    fresh = active.read_bytes()
    run(offline=False)
    assert active.read_bytes() == fresh
    report['cases'].append({'edition': data['edition'], 'binary_sha256': hashlib.sha256(app.read_bytes()).hexdigest(),
                            'cold_ms': cold, 'warm_ms': warm, 'recovery_ms': repair,
                            'offline_seed': True, 'hashes_verified': True, 'warm_cache_unchanged': True,
                            'corruption_recovered': True, 'fresh_cache_needs_no_network': True})
assert len({case['edition'] for case in report['cases']}) == len(report['cases'])
out = r / 'dist/catalog-windows-report.json'
out.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
