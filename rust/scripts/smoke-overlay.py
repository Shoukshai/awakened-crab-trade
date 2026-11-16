#!/usr/bin/env python3
"""Live desktop smoke check. PoE must be active; never sends gameplay keys."""
import json
import pathlib
import subprocess
import sys
import time

app = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else 'target/debug/awakened-poe-trade-rs').resolve()
def command(name):
    return subprocess.check_output([str(app), name], text=True)
def status():
    return json.loads(command('status'))
def wait_for(predicate):
    until = time.monotonic() + 4
    while time.monotonic() < until:
        s = status()
        if predicate(s):
            return s
        time.sleep(.03)
    raise RuntimeError(f'Timed out: {s}')
assert status()['game_active'], 'PoE must be active before testing'
for cycle in range(5):
    command('demo')
    s = wait_for(lambda s: s['state'] == 'Passive' and s['rendered'] and s['visible'])
    assert s['game_active'], s
    title = subprocess.check_output(['xdotool', 'getwindowfocus', 'getwindowname'], text=True).strip()
    assert title == 'Path of Exile', title
    command('hide')
    wait_for(lambda s: s['state'] == 'Hidden' and not s['visible'])
    assert status()['game_active']
    print(f'Cycle {cycle + 1}: drawn passive overlay, game focused, hidden cleanly')
print('PASS: 5 live open/close cycles. Pointer clicks and switching apps require separate checks.')
