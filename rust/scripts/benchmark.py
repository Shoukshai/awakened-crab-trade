#!/usr/bin/env python3
"""Compare two native diagnostic binaries without sending keys or network calls."""
import argparse
import hashlib
import json
import os
import pathlib
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('mode', choices=['parser', 'economy-cache'])
parser.add_argument('before', type=pathlib.Path)
parser.add_argument('after', type=pathlib.Path)
parser.add_argument('--runs', type=int, default=3)
parser.add_argument('--league', default='Covens')
parser.add_argument('--cache', type=pathlib.Path, help='Existing economy cache file; copied into a temporary test profile')
parser.add_argument('--output', type=pathlib.Path)
args = parser.parse_args()
if not 1 <= args.runs <= 30:
    parser.error('--runs must be 1–30')
root = pathlib.Path(__file__).resolve().parents[1]
binaries = {'before': args.before.resolve(), 'after': args.after.resolve()}
result = {
    'mode': args.mode,
    'binary_sha256': {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in binaries.items()},
    'samples': [],
    'network': 'blocked through local proxy; economy results must be cache hits',
    'scope': 'cold diagnostic processes, Linux wait4 peak RSS in KiB; not graphical-app RSS',
}
with tempfile.TemporaryDirectory(prefix='apt-benchmark-') as temporary:
    directory = pathlib.Path(temporary)
    environment = os.environ.copy()
    for key in ('HTTPS_PROXY', 'HTTP_PROXY', 'ALL_PROXY', 'https_proxy', 'http_proxy', 'all_proxy'):
        environment[key] = 'http://127.0.0.1:9'
    environment['NO_PROXY'] = environment['no_proxy'] = ''
    fixtures = ['driftwood-wand', 'scroll-of-wisdom']
    if args.mode == 'economy-cache':
        if args.cache is None:
            parser.error('economy-cache requires --cache; the original file is never changed')
        cached = json.loads(args.cache.read_text())
        if cached['league'] != args.league:
            parser.error('cache league differs from --league')
        cached['fetched'] = int(time.time())
        # Only this local timestamp is refreshed to keep the measured path offline.
        from urllib.parse import quote
        path = directory / 'cache/awakened-poe-trade-rs/economy' / (quote(args.league, safe='-._~') + '.json')
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps(cached, ensure_ascii=False, separators=(',', ':')))
        environment['XDG_CACHE_HOME'] = str(directory / 'cache')
        fixtures = ['scroll-of-wisdom']
        result['cache_bytes'] = path.stat().st_size
    for fixture in fixtures:
        for run in range(args.runs):
            outputs = {}
            for label, binary in binaries.items():
                command = [str(binary), 'inspect' if args.mode == 'parser' else 'economy', str(root / f'src/fixtures/{fixture}.txt')]
                if args.mode == 'economy-cache':
                    command.append(args.league)
                out_path = directory / f'{label}.json'
                log_path = directory / f'{label}.log'
                with out_path.open('wb') as out, log_path.open('wb') as log:
                    start = time.perf_counter()
                    child = subprocess.Popen(command, env=environment, stdout=out, stderr=log)
                    _, status, usage = os.wait4(child.pid, 0)
                    child.returncode = os.waitstatus_to_exitcode(status)
                    if child.returncode:
                        raise SystemExit(log_path.read_text())
                    elapsed = time.perf_counter() - start
                value = json.loads(out_path.read_text())
                if args.mode == 'economy-cache':
                    assert value['economy']['Ok']['from_cache'], 'Lookup reached a network path'
                    value['economy']['Ok'].pop('age_seconds')
                outputs[label] = value
                result['samples'].append({
                    'phase': label, 'fixture': fixture, 'run': run,
                    'elapsed_ms': round(elapsed * 1000, 3),
                    'cpu_ms': round((usage.ru_utime + usage.ru_stime) * 1000, 3),
                    'peak_rss_kib': usage.ru_maxrss,
                    'diagnostics': log_path.read_text().strip(),
                })
            assert outputs['before'] == outputs['after'], 'Functional result changed'
result['functional_results_equal'] = True
text = json.dumps(result, indent=2, ensure_ascii=False) + '\n'
if args.output:
    args.output.write_text(text)
else:
    print(text, end='')
