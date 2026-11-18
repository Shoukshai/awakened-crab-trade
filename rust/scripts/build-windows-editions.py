#!/usr/bin/env python3
"""Build independent standard and OCR portable executables from one checkout."""
import os,pathlib,subprocess
r=pathlib.Path(__file__).resolve().parents[1]
for ocr in [False,True]:
 env=os.environ.copy();env['CARGO_TARGET_DIR']=str(r/('target/windows-ocr' if ocr else 'target'))
 subprocess.run(['sh',str(r/'scripts/cargo-windows.sh'),'build','--release','--locked','--offline','--bin','awakened-crab-trade','-j3',*(['--features','cloud-ocr'] if ocr else [])],env=env,check=True)
 for script in ['package-windows.py','package-windows-portable.py']:
  subprocess.run(['python3',str(r/'scripts'/script),*(['--ocr'] if ocr else [])],check=True)
