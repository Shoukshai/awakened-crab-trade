#!/usr/bin/env python3
"""Build-time download of pinned neural OCR assets; no Python is shipped."""
import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import urllib.request
import zipfile

root = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--platform', choices=['windows', 'linux'], default='windows')
options = parser.parse_args()
lock = json.loads((root / 'packaging/windows/neural-ocr.json').read_text())
cache = root / 'dist/neural-downloads'
cache.mkdir(parents=True, exist_ok=True)
output = root / f'dist/neural-runtime-{options.platform}'
output.mkdir(exist_ok=True)
licenses = output / 'licenses/neural-ocr'
licenses.mkdir(parents=True, exist_ok=True)


def verify(data, sha, name):
    if hashlib.sha256(data).hexdigest() != sha:
        raise SystemExit(f'Checksum mismatch: {name}')
    return data


def fetch(record):
    path = cache / record['sha256']
    if not path.exists():
        temporary = path.with_suffix('.part')
        with urllib.request.urlopen(record['url'], timeout=90) as response:
            temporary.write_bytes(response.read())
        verify(temporary.read_bytes(), record['sha256'], record['url'])
        temporary.replace(path)
    verify(path.read_bytes(), record['sha256'], path)
    return path


model = output / 'share/ocr' / lock['model']['file']
model.parent.mkdir(parents=True, exist_ok=True)
shutil.copyfile(fetch(lock['model']), model)
shutil.copyfile(fetch(lock['model_license']), licenses / 'RapidOCR-LICENSE')
runtime = lock['runtime'][options.platform]
with zipfile.ZipFile(fetch(runtime)) as archive:
    # Wheel is only an upstream binary container. Never extract Python modules,
    # the Python extension, or install the wheel into an interpreter.
    for source, target in [
        ('onnxruntime/capi/' + runtime['library'], output / ('onnxruntime.dll' if options.platform == 'windows' else 'libonnxruntime.so')),
        ('onnxruntime/LICENSE', licenses / 'ONNX-Runtime-LICENSE'),
        ('onnxruntime/ThirdPartyNotices.txt', licenses / 'ONNX-Runtime-ThirdPartyNotices.txt'),
    ]:
        target.write_bytes(archive.read(source))

if options.platform == 'windows':
    vc = lock['visual_cpp']
    data = fetch(vc).read_bytes()
    work = cache / 'vc-extracted'
    work.mkdir(exist_ok=True)
    for name, members in [('bootstrap_cab', ['u4']), ('payload_cab', ['a12'])]:
        cabinet = vc[name]
        path = work / (name + '.cab')
        path.write_bytes(data[cabinet['offset']:cabinet['offset'] + cabinet['size']])
        subprocess.run(['7z', 'x', '-y', '-o' + str(work), str(path), *members], check=True, stdout=subprocess.DEVNULL)
    terms = verify((work / 'u4').read_bytes(), vc['license_sha256'], 'Visual C++ license')
    (licenses / 'Visual-Cpp-Runtime-LICENSE.rtf').write_bytes(terms)
    subprocess.run(['7z', 'x', '-y', '-o' + str(work), str(work / 'a12'),
                    *(name + '_amd64' for name in vc['dlls'])], check=True, stdout=subprocess.DEVNULL)
    for name, sha in vc['dlls'].items():
        (output / name).write_bytes(verify((work / (name + '_amd64')).read_bytes(), sha, name))

shutil.copyfile(root / 'packaging/windows/neural-ocr.json', licenses / 'sources.json')
print(json.dumps({'platform': options.platform, 'runtime': str(output), 'model_bytes': model.stat().st_size, 'python_runtime': False}))
