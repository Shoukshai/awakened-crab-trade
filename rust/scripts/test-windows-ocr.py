#!/usr/bin/env python3
"""Exercise real OCR pixels, Alt capture, cancellation and a topmost cloud fixture."""
from PIL import Image
import os,pathlib,json,subprocess,time,socket,http.server,threading,sys,hashlib,fcntl
from trade_fixture import make_handler
r=pathlib.Path(__file__).resolve().parent.parent
s=json.loads((r/'dist/windows-sandbox.json').read_text());root=pathlib.Path(s['root'])
assert root.name.startswith('apt-windows-desktop-') and (root/'fixture.guard').is_file()
test_lock=(root/'test.lock').open('w');fcntl.flock(test_lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
env=os.environ.copy();env.update(s['env']);assert env['DISPLAY']!=os.environ.get('DISPLAY')
assert pathlib.Path(env['WINEPREFIX']).parent==root
subprocess.run([env['WINESERVER'],'-k'],env=env,timeout=10,check=False)
# GTK reads the OS keyboard-layout DLL. Wine lacks it; use an open-source
# ReactOS US layout in this guarded, private prefix, never in the shipped app.
import shutil
layout=r/'dist/windows-tools/reactos-test-layout/apt-test-kbdus.dll'
assert layout.is_file(),'Run scripts/setup-windows-test-layout.py first'
shutil.copy2(layout,root/'prefix/drive_c/windows/system32/apt-test-kbdus.dll')
registry=root/'prefix/system.reg';blocks=registry.read_text().split('\n\n');changed=0
for index,block in enumerate(blocks):
 first=block.split('\n',1)[0]
 if 'Keyboard Layouts' in first and '00000409]' in first:
  lines=[line for line in block.splitlines() if not line.startswith('"Layout File"=')]
  blocks[index]='\n'.join(lines)+'\n"Layout File"="apt-test-kbdus.dll"';changed+=1
assert changed==1,'Unexpected private Wine layout registry'
registry.write_text('\n\n'.join(blocks))
# Let Wine manage its Win32 windows itself. River must not recenter GTK windows
# or place the fake game's fullscreen surface above the Win32 topmost overlay.
user=root/'prefix/user.reg';text=user.read_text();section=r'[Software\\Wine\\X11 Driver]'
blocks=text.split('\n\n');found=False
for i,block in enumerate(blocks):
 if block.split('\n',1)[0].startswith(section):
  blocks[i]='\n'.join(line for line in block.splitlines() if not line.startswith('"Managed"='))+'\n"Managed"="N"';found=True
if not found:blocks.append(section+'\n"Managed"="N"')
user.write_text('\n\n'.join(blocks)+'\n')
for p in root.rglob('control.json'):p.unlink()
for parent in [root/'cache/awakened-crab-trade',*list((root/'prefix/drive_c/users').glob('*/AppData/Local/awakened-crab-trade'))]:
 for name in ['economy','icons']:shutil.rmtree(parent/name,ignore_errors=True)
for name in ['fixture.state.json','foreign.state.json','clipboard-read.txt']:(root/name).unlink(missing_ok=True)
wp=lambda p:'Z:'+str(pathlib.Path(p).resolve()).replace('/','\\')
env.update(APPDATA=wp(root/'config'),LOCALAPPDATA=wp(root/'cache'),APT_RS_WINDOWS_TEST_DIR=wp(root),APT_RS_WINDOWS_TEST_TOKEN=(root/'fixture.guard').read_text(),GDK_BACKEND='win32',HTTPS_PROXY='http://127.0.0.1:9',HTTP_PROXY='http://127.0.0.1:9',NO_PROXY='127.0.0.1,localhost')
for d in ['config/awakened-crab-trade','cache/awakened-crab-trade']:(root/d).mkdir(parents=True,exist_ok=True)
config=root/'config/awakened-crab-trade/config.json'
config_data=json.dumps({'league':'Covens','automatic_league':False,'automatic_search':True,'language':os.environ.get('APT_FIXTURE_LANGUAGE','en'),'theme':'tide','restore_clipboard':True,'capture':{'mode':'ocr','executable':'apt-cloud-fixture.exe','region':[38,0,59,85]}})
config.write_text(config_data)
# Wine initializes APPDATA from its private registry when starting explorer.
for user in (root/'prefix/drive_c/users').iterdir():
 if (user/'AppData').is_dir():
  path=user/'AppData/Roaming/awakened-crab-trade/config.json';path.parent.mkdir(parents=True,exist_ok=True);path.write_text(config_data)
posts=[];ninja=[];family_env={'APT_FIXTURE_ITEM':'wand'}
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),make_handler(r,family_env,posts,ninja));threading.Thread(target=server.serve_forever,daemon=True).start()
env['APT_RS_TEST_TRADE_BASE']=env['APT_RS_TEST_ECONOMY_BASE']=f'http://127.0.0.1:{server.server_address[1]}'
processes=[];logs=[];report={'windowing':'Independent Win32 windows with Wine managing placement on private XWayland; windowed fake game','os_keyboard_layout':'Private Wine prefix with pinned ReactOS US layout DLL; absent from production ZIP','scope':'Windows binary on private Wine desktop, fake PoE process, synthetic offers and public-market fixtures. No real game input or real market prices.'};success=False
app=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else r/'dist/windows-dev/awakened-crab-trade.exe').resolve()
def launch(exe,args,name):
 log=(root/(name+'.log')).open('w');logs.append(log)
 command=[s['wine'],wp(exe),*args]
 p=subprocess.Popen(command,env=env,stdout=log,stderr=subprocess.STDOUT);processes.append(p);return p
def endpoint():
 p=root/'cache/awakened-crab-trade/control.json'
 if not p.exists():
  matches=list((root/'prefix/drive_c/users').glob('*/AppData/Local/awakened-crab-trade/control.json'));p=matches[0] if matches else p
 return json.loads(p.read_text())
def command(name):
 e=endpoint()
 with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as sock:
  sock.settimeout(10);sock.connect(('127.0.0.1',e['port']));sock.send(json.dumps({'token':e['token'],'command':name}).encode());body=sock.recv(65536)
 return json.loads(body) if name=='status' else body.decode()
def status():return command('status')
def wait(fn,label,seconds=15):
 until=time.monotonic()+seconds;last=None
 while time.monotonic()<until:
  try:
   last=fn()
   if last:return last
  except (OSError,ValueError,KeyError):pass
  time.sleep(.05)
 raise RuntimeError(f'Timeout {label}: {last}')
def click(x,y):subprocess.run(['xdotool','mousemove','--sync',str(int(x)),str(int(y)),'click','1'],env=env,check=True)
def pointer(x,y):subprocess.run(['xdotool','mousemove','--sync',str(int(x)),str(int(y))],env=env,check=True)
def control(rect):
 st=status();o=st['origin'];assert st['state']=='Interactive' and rect and rect.get('mapped',True),rect
 click(o['x']+rect['x']+rect['width']/2,o['y']+rect['y']+rect['height']/2)
def shot(name):
 folder=r/'dist/previews/windows';folder.mkdir(parents=True,exist_ok=True);st=status();o=st['origin']
 subprocess.run(['grim','-g',f"{o['x']},{o['y']} {st['width']}x{st['height']}",str(folder/name)],env=env,check=True)
try:
 # Build both fixture executables from the guarded test helper, never the game.
 subprocess.run(['sh',str(r/'scripts/cargo-windows.sh'),'build','--locked','--offline','--features','windows-test-fixture','--bin','apt-win32-fixture','-j3'],check=True)
 shutil.copy2(r/'target/x86_64-pc-windows-gnullvm/debug/apt-win32-fixture.exe',root/'apt-cloud-fixture.exe')
 shutil.copy2(r/'target/x86_64-pc-windows-gnullvm/debug/apt-win32-fixture.exe',root/'apt-win32-fixture.exe')
 from PIL import ImageDraw,ImageFont
 font_file=subprocess.check_output(['fc-match','-f','%{file}','sans-serif'],text=True).strip()
 font=ImageFont.truetype(font_file,22)
 small=ImageFont.truetype(font_file,20)
 for advanced in [False,True]:
  image=Image.new('RGB',(1392,866),(30,34,40));d=ImageDraw.Draw(image);d.rectangle((580,0,1300,700),fill=(5,5,5))
  rows=[('VICTORY TOUCH','#ffff77'),("ASSASSIN'S MITTS",'#ffff77'),('Quality: +14%','#eeeeee'),('Evasion Rating: 129','#eeeeee'),('Energy Shield: 41','#eeeeee')]
  rows += [('Item Level: 71','#eeeeee')] if advanced else [('Requires Level 61','#eeeeee')]
  rows += [('Adds 1 to 2 Physical Damage to Attacks','#9999ff'),('+445 to Accuracy Rating','#9999ff'),('+13 to maximum Energy Shield','#9999ff'),('+119 to maximum Life','#9999ff'),('Regenerate 56.5 Life per second','#9999ff'),('+19 to maximum Mana','#9999ff'),('+35% to Fire Resistance','#9999ff')]
  for i,(text,color) in enumerate(rows):d.text((940,20+i*37),text,fill=color,font=font if i<2 else small,anchor='mt')
  image.save(root/('fixture-ocr-advanced.bmp' if advanced else 'fixture-ocr.bmp'))
 launch(app,['daemon'],'ocr-checker');wait(lambda:endpoint(),'OCR control socket',45)
 launch(root/'apt-cloud-fixture.exe',['--windowed','--cloud'],'cloud')
 wait(lambda:(root/'fixture.state.json').exists(),'cloud fixture');assert json.loads((root/'fixture.state.json').read_text())['topmost'];click(200,700);wait(lambda:status()['game_active'],'cloud process selected')
 # Alt must already be held when the pixels are acquired. Keep it long enough
 # to test the gate, then release before review becomes interactive.
 subprocess.run(['xdotool','keydown','Alt_L'],env=env,check=True);time.sleep(.3)
 subprocess.run(['grim',str(root/'ocr-before.png')],env=env,check=True);subprocess.run(['xdotool','keydown','Control_L','key','d','keyup','Control_L'],env=env,check=True);wait(lambda:status()['capture_pending'] is not None,'OCR queued')
 checks=[]
 for _ in range(8):
  start=time.monotonic();st=status();checks.append(time.monotonic()-start);time.sleep(.1)
 subprocess.run(['xdotool','keyup','Alt_L'],env=env,check=True)
 wait(lambda:status().get('ocr_review',{}).get('mapped') and status()['state']=='Interactive','OCR review',30)
 st=status();assert st['native_above_target'],'overlay behind topmost cloud client';review=st['ocr_review'];assert review['apply']['enabled'],review
 assert 'Item Level: 71' in review['text'],review
 assert 'Regenerate 56.5 Life per second' in review['text'],review
 assert not posts and st['request'] is None
 assert json.loads((root/'fixture.state.json').read_text())['copies']==0,'OCR must never inject Ctrl+Alt+C'
 assert max(checks)<1,'GTK status stopped responding during OCR'
 shot('ocr-review.png');control(review['apply'])
 wait(lambda:status()['item'] and not status()['ocr_review']['mapped'],'OCR accepted')
 st=status();assert st['item']['base']=="Assassin's Mitts",st['item'];assert len(st['stat_controls'])>=7,st['stat_controls'];assert st['item_filters']['itemLevel']['value']==71,st['item_filters']
 shot('ocr-filters.png');assert not posts
 # A second request is cancelled by Escape/hide before it may reopen the panel.
 command('hide');click(200,700);wait(lambda:status()['game_active'],'cloud focused again')
 subprocess.run(['xdotool','keydown','Alt_L'],env=env,check=True);time.sleep(.2);command('locked-copy');time.sleep(.15);command('hide');subprocess.run(['xdotool','keyup','Alt_L'],env=env,check=True)
 time.sleep(2);assert status()['state']=='Hidden','cancelled OCR reopened the panel'
 # A foreign executable must not be accepted just because its title resembles a client.
 launch(root/'apt-win32-fixture.exe',['--foreign'],'ocr-foreign');wait(lambda:(root/'foreign.state.json').exists(),'foreign fixture')
 subprocess.run(['xdotool','key','--clearmodifiers','ctrl+alt+d'],env=env,check=True);time.sleep(.3);assert status()['state']=='Hidden' and status()['capture_pending'] is None
 for mode in ['clipboard','ocr']:
  command('settings');wait(lambda:status()['view']=='Settings' and status()['state']=='Interactive','capture settings')
  control(status()['preferences']['tabs'][2]);wait(lambda:(status()['preferences']['capture_source']['source']['rect'] or {}).get('mapped'),'capture tab')
  control(status()['preferences']['capture_source']['source']['rect']);wait(lambda:status()['owned_popup'],'source menu')
  subprocess.run(['xdotool','key','--clearmodifiers','Home',*(['Down'] if mode=='ocr' else []),'Return'],env=env,check=True)
  wait(lambda:status()['preferences']['capture_source']['source']['value']==mode and not status()['owned_popup'],'capture mode draft')
  control(status()['preferences']['apply']);wait(lambda:status()['state']=='Hidden' and status()['settings']['capture']['mode']==mode,'capture mode saved')
 report.update(capture_settings_persist=True,physical_shortcut=True,ocr_text=review['text'],ocr_status=review['status'],no_clipboard_injection=True,no_automatic_trade=True,cancellation=True,foreign_scope=True,max_status_ms=round(max(checks)*1000),topmost_fixture=True)
 (r/'dist/windows-ocr-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2),flush=True);success=True
finally:
 try:command('quit')
 except Exception:pass
 for process in reversed(processes):
  if process.poll() is None:
   process.terminate()
   try:process.wait(timeout=3)
   except subprocess.TimeoutExpired:process.kill();process.wait()
 for log in logs:log.close()
 for name in ['fixture-ocr.bmp','fixture-ocr-advanced.bmp']:(root/name).unlink(missing_ok=True)
 server.shutdown()
 if not success:print('Diagnostics:',root)
