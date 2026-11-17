#!/usr/bin/env python3
"""Boot a private Windows test desktop; never use a real game or Wine prefix."""
import os,pathlib,tempfile,subprocess,json,time,signal,shutil,sys,secrets
r=pathlib.Path(__file__).resolve().parent.parent
root=pathlib.Path(tempfile.mkdtemp(prefix='apt-windows-desktop-'));root.chmod(0o700)
(root/'fixture.guard').write_text(secrets.token_hex(32));init=root/'init'
init.write_text('''#!/bin/sh
printf '%s\\n' "$DISPLAY" > "$XDG_RUNTIME_DIR/display"
printf '%s\\n' "$WAYLAND_DISPLAY" > "$XDG_RUNTIME_DIR/wayland"
riverctl rule-add -title "Path of Exile" float
riverctl rule-add -title "APT foreign fixture" float
riverctl rule-add -title "Awakened Crab Trade*" float
riverctl focus-follows-cursor disabled
touch "$XDG_RUNTIME_DIR/ready"
''');init.chmod(0o700)
env=os.environ.copy();env.update(XDG_RUNTIME_DIR=str(root),WLR_BACKENDS='headless',WLR_HEADLESS_OUTPUTS='1',WLR_RENDERER='pixman',WINEPREFIX=str(root/'prefix'),WINEARCH='win64',WINEDEBUG='-all',WINEDLLOVERRIDES='mscoree,mshtml=',WINEFSYNC='0',WINEESYNC='0')
wine=pathlib.Path(os.environ.get('APT_WINDOWS_WINE') or shutil.which('wine') or (pathlib.Path.home()/'.local/share/Steam/compatibilitytools.d/dwproton-11.0-14-x86_64/files/bin/wine'));assert wine.is_file()
env['WINESERVER']=str(wine.with_name('wineserver'));env['WINELOADER']=str(wine)
processes=[];logs=[]
def start(args,name):
 log=(root/(name+'.log')).open('w');logs.append(log);p=subprocess.Popen(args,env=env,stdout=log,stderr=subprocess.STDOUT);processes.append(p);return p
try:
 start(['river','-c',str(init)],'river')
 until=time.monotonic()+10
 while not(root/'ready').exists():
  if time.monotonic()>until:raise RuntimeError('private River failed')
  time.sleep(.05)
 env.update(DISPLAY=(root/'display').read_text().strip(),WAYLAND_DISPLAY=(root/'wayland').read_text().strip())
 assert env['DISPLAY']!=os.environ.get('DISPLAY')
 subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode','1600x1000@60Hz'],env=env,check=True,stdout=subprocess.DEVNULL)
 boot=start([str(wine),'wineboot','-i'],'wineboot');boot.wait(timeout=45)
 until=time.monotonic()+20
 while not(root/'prefix/system.reg').exists():
  if time.monotonic()>until:raise RuntimeError('Private Wine prefix did not initialize')
  time.sleep(.1)
 fonts=root/'prefix/drive_c/windows/Fonts';fonts.mkdir(exist_ok=True)
 for pattern in ['sans-serif','sans-serif:bold','sans-serif:italic']:
  font=pathlib.Path(subprocess.check_output(['fc-match','-f','%{file}',pattern],text=True).strip());assert font.is_file();shutil.copy2(font,fonts/font.name)
 info={'root':str(root),'wine':str(wine),'env':{k:v for k,v in env.items() if k in ['DISPLAY','WAYLAND_DISPLAY','XDG_RUNTIME_DIR','WLR_BACKENDS','WLR_HEADLESS_OUTPUTS','WLR_RENDERER','WINEPREFIX','WINEARCH','WINEDEBUG','WINEDLLOVERRIDES','WINEFSYNC','WINEESYNC','WINESERVER','WINELOADER']}};(r/'dist/windows-sandbox.json').write_text(json.dumps(info,indent=2)+'\n');print(json.dumps({'root':str(root),'ready':True}),flush=True)
 while not(root/'stop').exists():time.sleep(.5)
finally:
 subprocess.run([str(wine.with_name('wineserver')),'-k'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 for p in reversed(processes):
  if p.poll() is None:
   p.terminate()
   try:p.wait(timeout=3)
   except subprocess.TimeoutExpired:p.kill();p.wait()
 for log in logs:log.close()
