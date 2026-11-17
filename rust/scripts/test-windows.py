#!/usr/bin/env python3
"""Validate the Windows binary on an isolated Wine desktop, with a fake game."""
from PIL import Image
import os,pathlib,json,subprocess,time,socket,http.server,threading,sys,hashlib,fcntl
from trade_fixture import make_handler
r=pathlib.Path(__file__).resolve().parent.parent
s=json.loads((r/'dist/windows-sandbox.json').read_text());root=pathlib.Path(s['root'])
assert root.name.startswith('apt-windows-desktop-') and (root/'fixture.guard').is_file()
test_lock=(root/'test.lock').open('w');fcntl.flock(test_lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
env=os.environ.copy();env.update(s['env']);assert env['DISPLAY']!=os.environ.get('DISPLAY')
assert pathlib.Path(env['WINEPREFIX']).parent==root
subprocess.run([env['WINESERVER'],'-k'],env=env,timeout=10,check=True)
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
for parent in [root/'cache/awakened-poe-trade-rs',*list((root/'prefix/drive_c/users').glob('*/AppData/Local/awakened-poe-trade-rs'))]:
 for name in ['economy','icons']:shutil.rmtree(parent/name,ignore_errors=True)
for name in ['fixture.state.json','foreign.state.json','clipboard-read.txt']:(root/name).unlink(missing_ok=True)
wp=lambda p:'Z:'+str(pathlib.Path(p).resolve()).replace('/','\\')
env.update(APPDATA=wp(root/'config'),LOCALAPPDATA=wp(root/'cache'),APT_RS_WINDOWS_TEST_DIR=wp(root),APT_RS_WINDOWS_TEST_TOKEN=(root/'fixture.guard').read_text(),GDK_BACKEND='win32',HTTPS_PROXY='http://127.0.0.1:9',HTTP_PROXY='http://127.0.0.1:9',NO_PROXY='127.0.0.1,localhost')
for d in ['config/awakened-poe-trade-rs','cache/awakened-poe-trade-rs']:(root/d).mkdir(parents=True,exist_ok=True)
config=root/'config/awakened-poe-trade-rs/config.json'
config_data=json.dumps({'league':'Covens','automatic_league':False,'automatic_search':False,'restore_clipboard':True})
config.write_text(config_data)
# Wine initializes APPDATA from its private registry when starting explorer.
for user in (root/'prefix/drive_c/users').iterdir():
 if (user/'AppData').is_dir():
  path=user/'AppData/Roaming/awakened-poe-trade-rs/config.json';path.parent.mkdir(parents=True,exist_ok=True);path.write_text(config_data)
posts=[];ninja=[];family_env={'APT_FIXTURE_ITEM':'wand'}
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),make_handler(r,family_env,posts,ninja));threading.Thread(target=server.serve_forever,daemon=True).start()
env['APT_RS_TEST_TRADE_BASE']=env['APT_RS_TEST_ECONOMY_BASE']=f'http://127.0.0.1:{server.server_address[1]}'
processes=[];logs=[];report={'windowing':'Independent Win32 windows with Wine managing placement on private XWayland; windowed fake game','os_keyboard_layout':'Private Wine prefix with pinned ReactOS US layout DLL; absent from production ZIP','scope':'Windows binary on private Wine desktop, fake PoE process, synthetic offers and public-market fixtures. No real game input or real market prices.'};success=False
app=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else r/'dist/windows-dev/awakened-poe-trade-rs.exe').resolve()
def launch(exe,args,name):
 log=(root/(name+'.log')).open('w');logs.append(log)
 command=[s['wine'],wp(exe),*args]
 p=subprocess.Popen(command,env=env,stdout=log,stderr=subprocess.STDOUT);processes.append(p);return p
def endpoint():
 p=root/'cache/awakened-poe-trade-rs/control.json'
 if not p.exists():
  matches=list((root/'prefix/drive_c/users').glob('*/AppData/Local/awakened-poe-trade-rs/control.json'));p=matches[0] if matches else p
 return json.loads(p.read_text())
def command(name):
 e=endpoint()
 with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as sock:
  sock.settimeout(3);sock.connect(('127.0.0.1',e['port']));sock.send(json.dumps({'token':e['token'],'command':name}).encode());body=sock.recv(65536)
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
 launch(app,['daemon'],'checker')
 wait(lambda:endpoint(),'Windows control socket',45)
 print('daemon control ready',flush=True)
 command('settings');wait(lambda:status()['view']=='Settings' and status()['state']=='Interactive' and status()['rendered'],'settings usable without game')
 shot('settings.png');command('hide');wait(lambda:status()['state']=='Hidden','hide settings')
 (root/'fixture-item.txt').write_text((r/'src/fixtures/driftwood-wand.txt').read_text())
 launch(root/'apt-win32-fixture.exe',['--windowed'],'game')
 wait(lambda:(root/'fixture.state.json').exists(),'fake game started',30)
 click(200,700);wait(lambda:status()['game_active'],'game detected by executable and class')
 subprocess.run(['grim',str(root/'before-veil.png')],env=env,check=True)
 baseline=Image.open(root/'before-veil.png').convert('RGB').getpixel((200,500));report['before_veil_pixel']=baseline
 print('fake game active',flush=True)
 for family,fixture in [('wand','driftwood-wand.txt'),('gem','price-gem.txt'),('currency','price-currency.txt'),('unique','price-unique.txt'),('map','price-map.txt'),('viscera','price-viscera.txt'),('influence','price-influence.txt')]:
  family_env['APT_FIXTURE_ITEM']=family;command('hide');(root/'fixture-item.txt').write_text((r/'src/fixtures'/fixture).read_text());click(200,700);wait(lambda:status()['game_active'],'game active before copy')
  before=json.loads((root/'fixture.state.json').read_text())['copies']
  subprocess.run(['xdotool','key','--clearmodifiers','ctrl+alt+d'],env=env,check=True)
  wait(lambda:status()['item'] and status()['visible'] and status()['state']=='Interactive' and status()['rendered'],'scoped shortcut copies into checker')
  assert json.loads((root/'fixture.state.json').read_text())['copies']>before
  wait(lambda:status()['summary']['image_ready'],'Windows image loader')
  st=status();assert st['native_rect']==st['panel_bounds'],(st['native_rect'],st['panel_bounds'])
  print(family,'captured',flush=True)
  if family=='wand':
   subprocess.run(['grim',str(root/'veil.png')],env=env,check=True)
   pixel=Image.open(root/'veil.png').convert('RGB').getpixel((200,500));report['veil_pixel']=pixel
   assert all(0<c<before for c,before in zip(pixel,(32,40,48))),pixel
  if family=='influence':assert any(row['influence_icons']>0 for row in status()['stat_controls'])
  if family=='currency':assert not status()['search_actions_visible']
  else:
   time.sleep(5.1)
   control(status()['search_button']);wait(lambda:status()['query'] and status()['query'].get('listings')==2,'Windows Trade offers')
   wait(lambda:len(status()['listings']['offers'])==2,'offer rows')
   if family=='wand':
    assert all(text in status()['listings']['offers'][0]['tooltip'] for text in ['30% increased Spell Damage','+21 to Intelligence','+100 to maximum Mana'])
    # A first click inside and an owned dropdown keep the checker open.
    control(status()['item_filter_controls']['corruption']['rect'])
    popup=wait(lambda:status()['popup_controls'] if status()['owned_popup'] else None,'owned dropdown')
    subprocess.run(['xdotool','key','--clearmodifiers','Escape'],env=env,check=True)
    wait(lambda:not status()['owned_popup'] and status()['state']=='Interactive','Escape closes only menu')
    control(status()['item_filter_controls']['corruption']['rect'])
    popup=wait(lambda:status()['popup_controls'] if status()['owned_popup'] else None,'owned dropdown reopened')
    item=popup['items'][1];o=status()['origin'];p=popup['origin']
    click(o['x']+p['x']+item['x']+item['width']/2,o['y']+p['y']+item['y']+item['height']/2)
    wait(lambda:not status()['owned_popup'] and status()['state']=='Interactive','menu selection keeps panel open')
    row=status()['stat_controls'][0];control(row['slider']);wait(lambda:status()['request'] is None,'slider invalidates results')
    assert status()['stat_controls'][0]['min_value']==10
    control(status()['stat_controls'][0]['min'])
    subprocess.run(['xdotool','key','--clearmodifiers','ctrl+a'],env=env,check=True)
    subprocess.run(['xdotool','type','11'],env=env,check=True)
    wait(lambda:status()['stat_controls'][0]['min_value']==11,'Windows numeric keyboard input')
    time.sleep(5.1)
    control(status()['search_button']);wait(lambda:status()['query'] and status()['query'].get('listings')==2,'updated slider query')
  pointer(status()['origin']['x']+8,900);time.sleep(.15);shot(family+'.png')
  if family!='currency':
   row=status()['listings']['offers'][0]['rect'];o=status()['origin'];pointer(o['x']+row['x']+row['width']-5,o['y']+row['y']+row['height']/2)
   started=time.monotonic()
   wait(lambda:status()['listings']['tooltip_visible'],'whole offer row hover');shot(family+'-hover.png')
   report[family+'_hover_ms']=round((time.monotonic()-started)*1000)
   assert report[family+'_hover_ms']<450
  click(1450,800);wait(lambda:status()['state']=='Hidden','outside click closes Windows panel')
  report[family]={'copied':True,'outside_close':True,'offers':family!='currency'}
 # Restoring a captured clipboard preserves text that existed before the copy.
 click(200,700);subprocess.run(['xdotool','key','F12'],env=env,check=True)
 wait(lambda:(root/'clipboard-read.txt').exists(),'clipboard inspection')
 assert (root/'clipboard-read.txt').read_text()=='APT private clipboard sentinel'
 report['clipboard_text_restored']=True
 # Passive mode keeps clicks and focus with the game; Escape closes it.
 subprocess.run(['xdotool','key','ctrl+d'],env=env,check=True)
 wait(lambda:status()['state']=='Passive' and status()['rendered'],'passive checker')
 control_rect=status()['search_button'];o=status()['origin'];click(o['x']+control_rect['x']+20,o['y']+control_rect['y']+10)
 assert status()['state']=='Passive' and status()['game_active']
 subprocess.run(['xdotool','key','Escape'],env=env,check=True);wait(lambda:status()['state']=='Hidden','Escape while game retains focus')
 report['passive_click_through_and_escape']=True
 # A different Windows executable receives its own keys and cannot copy from PoE.
 command('locked-copy');wait(lambda:status()['state']=='Interactive','open before foreign app')
 launch(root/'apt-win32-fixture.exe',['--foreign'],'foreign')
 wait(lambda:(root/'foreign.state.json').exists(),'foreign fixture')
 wait(lambda:status()['state']=='Hidden' and not status()['game_active'],'foreign focus hides checker')
 click(120,120);wait(lambda:not status()['game_active'],'foreign app receives real pointer focus')
 before=json.loads((root/'foreign.state.json').read_text())['keys']
 subprocess.run(['xdotool','key','ctrl+alt+d','z'],env=env,check=True);time.sleep(.2)
 assert status()['state']=='Hidden' and json.loads((root/'foreign.state.json').read_text())['keys']>before
 report['foreign_focus_hides_and_shortcuts_are_forwarded']=True
 # Launching the executable again reuses the existing service.
 endpoint_before=endpoint();launch(app,[],'second-instance')
 wait(lambda:status()['view']=='Settings' and status()['state']=='Interactive','second launch opens settings')
 assert endpoint()['pid']==endpoint_before['pid'];report['single_instance']=True
 report['binary_sha256']=hashlib.sha256(app.read_bytes()).hexdigest();report['settings_without_game']=True
 (r/'dist/windows-private-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2),flush=True);success=True
finally:
 if not success:
  try:(root/'failure-status.json').write_text(json.dumps(status(),indent=2))
  except Exception:pass
  subprocess.run(['grim',str(root/'failure.png')],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
  print('Diagnostics:',root,flush=True)
 try:command('quit')
 except Exception:pass
 server.shutdown();server.server_close()
 for log in logs:log.close()
