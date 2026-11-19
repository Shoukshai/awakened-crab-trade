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
for parent in [root/'cache/awakened-crab-trade-ocr',*list((root/'prefix/drive_c/users').glob('*/AppData/Local/awakened-crab-trade-ocr'))]:
 for name in ['economy','icons']:shutil.rmtree(parent/name,ignore_errors=True)
 (parent/'checker.log').unlink(missing_ok=True)
for name in ['cloud.state.json','foreign.state.json','clipboard-read.txt','capture-black-accelerated','capture-all-blank','hide-cloud']:(root/name).unlink(missing_ok=True)
wp=lambda p:'Z:'+str(pathlib.Path(p).resolve()).replace('/','\\')
env.update(APPDATA=wp(root/'config'),LOCALAPPDATA=wp(root/'cache'),APT_RS_WINDOWS_TEST_DIR=wp(root),APT_RS_WINDOWS_TEST_TOKEN=(root/'fixture.guard').read_text(),GDK_BACKEND='win32',HTTPS_PROXY='http://127.0.0.1:9',HTTP_PROXY='http://127.0.0.1:9',NO_PROXY='127.0.0.1,localhost')
for d in ['config/awakened-crab-trade-ocr','cache/awakened-crab-trade-ocr']:(root/d).mkdir(parents=True,exist_ok=True)
config=root/'config/awakened-crab-trade-ocr/config.json'
config_data=json.dumps({'league':'Covens','automatic_league':False,'automatic_search':True,'language':os.environ.get('APT_FIXTURE_LANGUAGE','en'),'theme':'tide','restore_clipboard':True,'capture':{'mode':'ocr','executable':'apt-cloud-fixture.exe'}})
config.write_text(config_data)
# Wine initializes APPDATA from its private registry when starting explorer.
for user in (root/'prefix/drive_c/users').iterdir():
 if (user/'AppData').is_dir():
  path=user/'AppData/Roaming/awakened-crab-trade-ocr/config.json';path.parent.mkdir(parents=True,exist_ok=True);path.write_text(config_data)
posts=[];ninja=[];family_env={'APT_FIXTURE_ITEM':'wand'}
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),make_handler(r,family_env,posts,ninja));threading.Thread(target=server.serve_forever,daemon=True).start()
env['APT_RS_TEST_TRADE_BASE']=env['APT_RS_TEST_ECONOMY_BASE']=f'http://127.0.0.1:{server.server_address[1]}'
processes=[];logs=[];report={'windowing':'Independent Win32 windows with Wine managing placement on private XWayland; windowed fake game','os_keyboard_layout':'Private Wine prefix with pinned ReactOS US layout DLL; absent from production ZIP','scope':'Windows binary on private Wine desktop, fake PoE process, synthetic offers and public-market fixtures. No real game input or real market prices.'};success=False
app=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else r/'dist/windows-dev/awakened-crab-trade.exe').resolve()
def launch(exe,args,name):
 log=(root/(name+'.log')).open('w');logs.append(log)
 command=[s['wine'],wp(exe),*args]
 p=subprocess.Popen(command,env=env,stdout=log,stderr=subprocess.STDOUT);processes.append(p);return p
def endpoint(app_id="awakened-crab-trade-ocr"):
 p=root/('cache/'+app_id+'/control.json')
 if not p.exists():
  matches=list((root/'prefix/drive_c/users').glob('*/AppData/Local/'+app_id+'/control.json'));p=matches[0] if matches else p
 return json.loads(p.read_text())
def command(name,app_id="awakened-crab-trade-ocr"):
 e=endpoint(app_id)
 with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as sock:
  sock.settimeout(10);sock.connect(('127.0.0.1',e['port']));sock.send(json.dumps({'token':e['token'],'command':name}).encode());body=sock.recv(65536)
 return json.loads(body) if name=='status' else body.decode()
def status():return command('status')
def normal_command(name):return command(name,"awakened-crab-trade")
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
def focus_target(label, app_id="awakened-crab-trade-ocr"):
 # Wine may finish unmapping GTK after the hide acknowledgement. Retry only
 # on this guarded fake desktop instead of losing the single focus click.
 def focused():
  if command('status',app_id)['game_active']:return True
  click(200,700)
  return command('status',app_id)['game_active']
 wait(focused,label)
def pointer(x,y):subprocess.run(['xdotool','mousemove','--sync',str(int(x)),str(int(y))],env=env,check=True)
def control(rect):
 st=status();o=st['origin'];assert st['state']=='Interactive' and rect and rect.get('mapped',True),rect
 click(o['x']+rect['x']+rect['width']/2,o['y']+rect['y']+rect['height']/2)
def capture_settings():
 command('settings');wait(lambda:status()['view']=='Settings' and status()['state']=='Interactive','settings opened')
 control(status()['preferences']['tabs'][2])
 wait(lambda:status()['preferences']['capture_source']['local_poe']['rect'].get('mapped'),'capture tab opened')
def shot(name):
 folder=r/'dist/previews/windows';folder.mkdir(parents=True,exist_ok=True);st=status();o=st['origin']
 subprocess.run(['grim','-g',f"{o['x']},{o['y']} {st['width']}x{st['height']}",str(folder/name)],env=env,check=True)
try:
 # Build both fixture executables from the guarded test helper, never the game.
 subprocess.run(['sh',str(r/'scripts/cargo-windows.sh'),'build','--locked','--offline','--features','windows-test-fixture','--bin','apt-win32-fixture','-j3'],check=True)
 shutil.copy2(r/'target/x86_64-pc-windows-gnullvm/debug/apt-win32-fixture.exe',root/'apt-cloud-fixture.exe')
 shutil.copy2(r/'target/x86_64-pc-windows-gnullvm/debug/apt-win32-fixture.exe',root/'apt-win32-fixture.exe')
 shutil.copy2(r/'dist/windows-tools/llvm-mingw-20261104-ucrt-ubuntu-22.04-x86_64/x86_64-w64-mingw32/bin/libunwind.dll',root/'libunwind.dll')
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
  if advanced:image.save(root/'fixture-local-ready.bmp')
 # Old runs stay in the disk log but must not leak into a copied session report.
 log_paths=[root/'cache/awakened-crab-trade-ocr/checker.log']
 log_paths += [u/'AppData/Local/awakened-crab-trade-ocr/checker.log' for u in (root/'prefix/drive_c/users').iterdir() if (u/'AppData').is_dir()]
 for path in log_paths:
  path.parent.mkdir(parents=True,exist_ok=True);path.write_text('APT OLD SESSION MUST NOT BE COPIED\n')
 launch(app,['daemon'],'ocr-checker');wait(lambda:endpoint(),'OCR control socket',45)
 if len(sys.argv)>2:
  normal=pathlib.Path(sys.argv[2]).resolve()
  data=json.dumps({'league':'Covens','automatic_league':False,'language':'en','theme':'terminal','capture':{'mode':'ocr','executable':'apt-cloud-fixture.exe'}})
  paths=[root/'config/awakened-crab-trade/config.json',*list((root/'prefix/drive_c/users').glob('*/AppData/Roaming/awakened-crab-trade/config.json'))]
  for user in (root/'prefix/drive_c/users').iterdir():
   if (user/'AppData').is_dir():paths.append(user/'AppData/Roaming/awakened-crab-trade/config.json')
  for path in paths:path.parent.mkdir(parents=True,exist_ok=True);path.write_text(data)
  launch(normal,['daemon'],'normal-checker');wait(lambda:endpoint('awakened-crab-trade'),'normal control socket',45)
  assert endpoint()['port']!=endpoint('awakened-crab-trade')['port']
  assert normal_command('status')['edition']=='awakened-crab-trade'
  assert normal_command('status')['settings']['capture']['mode']=='clipboard'
  assert normal_command('status')['settings']['theme']=='terminal' and status()['settings']['theme']=='tide'
  (root/'fixture-item.txt').write_text((r/'src/fixtures/driftwood-wand.txt').read_text())
  launch(root/'apt-win32-fixture.exe',['--windowed','--ocr-local'],'local-game');wait(lambda:(root/'fixture.state.json').exists(),'local game')
  focus_target('normal scope','awakened-crab-trade')
  subprocess.run(['xdotool','key','--clearmodifiers','ctrl+alt+d'],env=env,check=True)
  wait(lambda:normal_command('status')['item'] and normal_command('status')['state']=='Interactive','normal shortcut')
  assert status()['state']=='Hidden' and status()['capture_pending'] is None
  normal_command('hide');normal_generation=normal_command('status')['generation']
 launch(root/'apt-cloud-fixture.exe',['--windowed','--cloud'],'cloud')
 wait(lambda:(root/'cloud.state.json').exists(),'cloud fixture');assert json.loads((root/'cloud.state.json').read_text())['topmost']
 def focus_cloud():
  click(200,700)
  return status()['game_active']
 wait(focus_cloud,'cloud process selected')
 # Alt must already be held when the pixels are acquired. Keep it long enough
 # to test the gate, then release before review becomes interactive.
 (root/'capture-black-accelerated').touch()
 (root/'ocr-snapshot.ppm').unlink(missing_ok=True)
 subprocess.run(['xdotool','keydown','Alt_L'],env=env,check=True);time.sleep(.3)
 subprocess.run(['grim',str(root/'ocr-before.png')],env=env,check=True);subprocess.run(['xdotool','keydown','Control_L','key','d','keyup','Control_L'],env=env,check=True);wait(lambda:status()['capture_pending'] is not None,'OCR queued')
 checks=[]
 for _ in range(8):
  start=time.monotonic();st=status();checks.append(time.monotonic()-start);time.sleep(.1)
 wait(lambda:(root/'ocr-snapshot.ppm').exists(),'pixels captured while Alt held',15)
 subprocess.run(['xdotool','keyup','Alt_L'],env=env,check=True)
 def first_review_ready():
  current=status()
  assert current['state']!='Hidden' or current['game_active'],('OCR lost target focus before review',current['desktop'])
  return current.get('ocr_review',{}).get('mapped') and current['state']=='Interactive'
 wait(first_review_ready,'OCR review',30)
 report['target_focus_retained_during_recognition']=True
 st=status();assert st['native_above_target'],'overlay behind topmost cloud client';review=st['ocr_review'];assert review['apply']['enabled'],review
 if os.environ.get('APT_FIXTURE_EXPECT_FALLBACK')=='1':
  assert review['status'].startswith('Fallback OCR'),review
  assert review['diagnostic_visible'] and 'Version 0.36.7' in review['diagnostic'] and 'Neural OCR model:' in review['diagnostic'],review
  assert review['copy_diagnostic']['width']>0,review
  shot('ocr-fallback-diagnostic-en.png')
  control(review['copy_diagnostic'])
  focus_target('return to cloud after copying diagnostic');wait(lambda:status()['state']=='Hidden','review hidden after copying diagnostic')
  def copied_diagnostic():
   subprocess.run(['xdotool','key','--clearmodifiers','F12'],env=env,check=True)
   p=root/'clipboard-read.txt'
   text=p.read_text() if p.exists() else ''
   return text if 'Awakened Crab Trade diagnostics' in text and 'Neural OCR model:' in text else None
  full=wait(copied_diagnostic,'read full copied diagnostic')
  assert 'Version: 0.36.7' in full and 'CPU:' in full and 'Complete current session log' in full,full
  assert 'ocr neural fallback:' in full and 'APT OLD SESSION MUST NOT BE COPIED' not in full,full
  assert json.loads((root/'cloud.state.json').read_text())['copies']==0
  report.update(neural_failure_visible=True,diagnostic_copied=True,diagnostic=review['diagnostic'],no_clipboard_injection=True)
  (r/'dist/windows-ocr-fallback-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2),flush=True);success=True
  raise SystemExit(0)
 assert not review['diagnostic_visible'] and not review['diagnostic'],review
 if os.environ.get('APT_FIXTURE_EXPECT_RUST')=='1':assert review['status'].startswith('PP-OCRv5 English (Rust CPU)'),review
 assert 'Item Level: 71' in review['text'],review
 assert 'Regenerate 56.5 Life per second' in review['text'],review
 assert not posts and st['request'] is None
 assert json.loads((root/'cloud.state.json').read_text())['copies']==0,'OCR must never inject Ctrl+Alt+C'
 assert max(checks)<1,'GTK status stopped responding during OCR'
 (root/'capture-black-accelerated').unlink()
 for log in logs:log.flush()
 capture_log='\n'.join(p.read_text() for p in [root/'cache/awakened-crab-trade-ocr/checker.log',*list((root/'prefix/drive_c/users').glob('*/AppData/Local/awakened-crab-trade-ocr/checker.log'))] if p.exists())
 assert 'Windows Graphics Capture: blank frame (private fixture)' in capture_log,capture_log
 assert 'Desktop Duplication: blank frame (private fixture)' in capture_log,capture_log
 assert 'ocr capture backend=Client GDI' in capture_log or 'ocr capture backend=Desktop GDI' in capture_log,capture_log
 report['blank_accelerated_frames_fall_back']=True
 if len(sys.argv)>2:
  assert normal_command('status')['state']=='Hidden' and normal_command('status')['generation']==normal_generation
  assert json.loads((root/'fixture.state.json').read_text())['copies']>=1
  assert json.loads((root/'cloud.state.json').read_text())['copies']==0
  report['two_editions_running_without_shared_shortcuts_or_profiles']=True

 shot('ocr-review.png');control(review['apply'])
 wait(lambda:status()['item'] and not status()['ocr_review']['mapped'],'OCR accepted')
 st=status();assert st['item']['base']=="Assassin's Mitts",st['item'];assert len(st['stat_controls'])>=7,st['stat_controls'];assert st['item_filters']['itemLevel']['value']==71,st['item_filters']
 shot('ocr-filters.png');assert not posts
 auto_cases=[
  ('gem','Gem',350,330,[('IMPALE SUPPORT','#55d0bc'),('Level: 20','#eeeeee'),('Quality: +20%','#eeeeee')]),
  ('currency','Currency',400,150,[('DIVINE ORB','#eeeeee'),('Stack Size: 1/20','#eeeeee')]),
  ('unique','Unique',940,260,[('GOLDRIM','#df8d43'),('LEATHER CAP','#df8d43'),('Item Level: 71','#eeeeee'),('+35% to all Elemental Resistances','#9999ff')]),
  ('map','Map',370,350,[('STRAND MAP','#eeeeee'),('Map Tier: 16','#eeeeee'),('Item Level: 83','#eeeeee')]),
  ('wand','Equipment',940,220,[('GLYPHIC DRIFTWOOD WAND','#9999ff'),('Item Level: 71','#eeeeee'),('+25 to maximum Mana','#9999ff')]),
 ]
 for case,expected,cx,top,rows in auto_cases:
  image=Image.new('RGB',(1392,866),(30,34,40));draw=ImageDraw.Draw(image);draw.rectangle((max(0,cx-340),top-10,min(1390,cx+340),min(850,top+len(rows)*40+40)),fill=(5,5,5))
  for i,(text,color) in enumerate(rows):draw.text((cx,top+i*38),text,fill=color,font=font if i==0 else small,anchor='mt')
  for name in ['fixture-ocr.bmp','fixture-ocr-advanced.bmp']:image.save(root/name)
  family_env['APT_FIXTURE_ITEM']=case;command('hide');focus_target('cloud focused '+case)
  (root/'ocr-snapshot.ppm').unlink(missing_ok=True)
  subprocess.run(['xdotool','keydown','Alt_L'],env=env,check=True);time.sleep(.2);command('locked-copy');wait(lambda:(root/'ocr-snapshot.ppm').exists(),'auto snapshot '+case,15);subprocess.run(['xdotool','keyup','Alt_L'],env=env,check=True)
  wait(lambda:status().get('ocr_review',{}).get('mapped') and status()['state']=='Interactive','automatic '+case,35)
  current=status()['ocr_review'];assert current['apply']['enabled'],(case,current)
  control(current['apply']);wait(lambda:status()['item'] and not status()['ocr_review']['mapped'],'accept '+case)
  assert status()['family']==expected,(case,status()['item'],status()['family']);assert not posts
  print('automatic',case,'passed',flush=True)
 report['automatic_families']=[expected for _,expected,_,_,_ in auto_cases]
 # Exhausting every method must be bounded and keep the review action disabled.
 command('hide');focus_target('cloud focused for blank capture')
 (root/'capture-all-blank').touch();(root/'ocr-snapshot.ppm').unlink(missing_ok=True)
 subprocess.run(['xdotool','keydown','Alt_L'],env=env,check=True);time.sleep(.2);command('locked-copy');time.sleep(.2)
 subprocess.run(['xdotool','keyup','Alt_L'],env=env,check=True)
 wait(lambda:status().get('ocr_review',{}).get('mapped') and status()['state']=='Interactive','all blank diagnostic',10)
 failed=status()['ocr_review'];assert not failed['apply']['enabled'],failed
 assert 'No usable image from the cloud window' in failed['status'],failed
 for name in ['Windows Graphics Capture','Desktop Duplication','Desktop GDI','Client GDI']:assert name+': blank frame' in failed['status'],failed
 assert not (root/'ocr-snapshot.ppm').exists() and not posts
 (root/'capture-all-blank').unlink();report['all_blank_reports_each_backend']=True
 # A second request is cancelled by Escape/hide before it may reopen the panel.
 command('hide');focus_target('cloud focused again')
 subprocess.run(['xdotool','keydown','Alt_L'],env=env,check=True);time.sleep(.2);command('locked-copy');time.sleep(.15);command('hide');subprocess.run(['xdotool','keyup','Alt_L'],env=env,check=True)
 time.sleep(2);assert status()['state']=='Hidden','cancelled OCR reopened the panel'
 # A foreign executable must not be accepted just because its title resembles a client.
 launch(root/'apt-win32-fixture.exe',['--foreign'],'ocr-foreign');wait(lambda:(root/'foreign.state.json').exists(),'foreign fixture')
 subprocess.run(['xdotool','key','--clearmodifiers','ctrl+alt+d'],env=env,check=True);time.sleep(.3);assert status()['state']=='Hidden' and status()['capture_pending'] is None
 command('settings');wait(lambda:status()['view']=='Settings' and status()['state']=='Interactive','OCR settings')
 control(status()['preferences']['tabs'][2]);wait(lambda:(status()['preferences']['capture_source']['source']['rect'] or {}).get('mapped'),'OCR capture tab')
 assert status()['preferences']['capture_source']['source']['value']=='ocr'
 assert 'region' not in status()['settings']['capture']
 control(status()['preferences']['apply']);wait(lambda:status()['state']=='Hidden' and status()['settings']['capture']['mode']=='ocr','OCR settings saved')
 assert status()['edition']=='awakened-crab-trade-ocr'
 if len(sys.argv)>2:
  # A draft must not change the active target or its bindings until Save.
  capture_settings()
  local=status()['preferences']['capture_source']['local_poe'];assert not local['value']
  control(local['rect']);wait(lambda:status()['preferences']['capture_source']['local_poe']['value'],'local OCR draft selected')
  assert not status()['preferences']['capture_source']['client']['enabled']
  assert not status()['settings']['capture'].get('local_poe',False)
  control(status()['preferences']['cancel']);wait(lambda:status()['state']=='Hidden','cancel local OCR draft')
  capture_settings();assert not status()['preferences']['capture_source']['local_poe']['value']
  cloud_keys=status()['preferences']['hotkeys']
  control(status()['preferences']['capture_source']['local_poe']['rect']);wait(lambda:status()['preferences']['capture_source']['local_poe']['value'],'local OCR selected for Save')
  control(status()['preferences']['apply'])
  wait(lambda:status()['state']=='Hidden' and status()['settings']['capture'].get('local_poe'),'local OCR target saved')
  capture_settings();assert status()['preferences']['capture_source']['local_poe']['value']
  assert [k['value'] for k in status()['preferences']['hotkeys']]==[k['value'] for k in cloud_keys]
  control(status()['preferences']['cancel'])
  shutil.copy2(root/'fixture-local-ready.bmp',root/'fixture-ocr.bmp');shutil.copy2(root/'fixture-local-ready.bmp',root/'fixture-ocr-advanced.bmp')
  (root/'hide-cloud').touch();time.sleep(.2);command('hide');focus_target('local game selected')
  wait(lambda:normal_command('status')['game_active'],'local game selected by standard edition')
  normal_before=normal_command('status')['generation'];copies_before=json.loads((root/'fixture.state.json').read_text())['copies']
  (root/'ocr-snapshot.ppm').unlink(missing_ok=True)
  subprocess.run(['xdotool','keydown','Alt_L','keydown','Control_L','key','o','keyup','Control_L'],env=env,check=True)
  wait(lambda:(root/'ocr-snapshot.ppm').exists(),'local OCR physical shortcut capture',15)
  subprocess.run(['xdotool','keyup','Alt_L'],env=env,check=True)
  wait(lambda:status().get('ocr_review',{}).get('mapped') and status()['state']=='Interactive','local OCR review',35)
  local_review=status()['ocr_review'];assert local_review['apply']['enabled'] and 'PP-OCRv5' in local_review['status'],local_review
  assert "Assassin's Mitts" in local_review['text'] and 'Item Level: 71' in local_review['text'],local_review
  assert json.loads((root/'fixture.state.json').read_text())['copies']==copies_before,'Local OCR injected copy'
  assert normal_command('status')['state']=='Hidden' and normal_command('status')['generation']==normal_before,'Local OCR triggered standard hotkeys'
  assert not posts
  control(local_review['apply']);wait(lambda:status()['item'] and not status()['ocr_review']['mapped'],'accept local OCR')
  command('hide')
  def focus_local():
   click(200,700)
   return normal_command('status')['game_active']
  wait(focus_local,'standard local scope restored')
  subprocess.run(['xdotool','key','--clearmodifiers','ctrl+alt+d'],env=env,check=True)
  wait(lambda:normal_command('status')['state']=='Interactive' and normal_command('status')['item']['base']=='Driftwood Wand','standard clipboard remains available')
  assert status()['state']=='Hidden' and status()['capture_pending'] is None
  report['local_poe_ocr_opt_in_no_clipboard_or_shortcut_conflict']=True
 # The diagnostic tab must copy a fresh, complete report asynchronously.
 command('settings');wait(lambda:status()['view']=='Settings' and status()['state']=='Interactive','log settings')
 control(status()['preferences']['tabs'][-1])
 wait(lambda:status()['preferences']['logs']['mapped'] and not status()['preferences']['logs']['busy'] and status()['preferences']['logs']['bytes']>0,'session log view')
 log_info=status()['preferences']['logs']
 assert 'Version: 0.36.7' in log_info['prefix'] and 'CPU:' in log_info['prefix'],log_info
 shot('session-log-en.png')
 control(log_info['copy'])
 wait(lambda:not status()['preferences']['logs']['busy'] and status()['preferences']['logs']['status']=='Entire log copied.','copy entire current log')
 command('hide')
 def focus_log_target():
  click(200,700)
  return status()['game_active']
 wait(focus_log_target,'return to target after copying log')
 (root/'clipboard-read.txt').unlink(missing_ok=True)
 def copied_session():
  subprocess.run(['xdotool','key','--clearmodifiers','F12'],env=env,check=True)
  p=root/'clipboard-read.txt';text=p.read_text() if p.exists() else ''
  return text if 'Awakened Crab Trade diagnostics' in text else None
 full=wait(copied_session,'read full session clipboard')
 assert 'APT OLD SESSION MUST NOT BE COPIED' not in full,full
 expected_backend='ocr neural self_test=rust' if os.environ.get('APT_FIXTURE_EXPECT_RUST')=='1' else 'ocr neural self_test=optimized'
 for expected in ['Version: 0.36.7','CPU:','OS: Windows','ocr probe input shape=','ocr probe output shape=',expected_backend]:
  assert expected in full,(expected,full)
 report['entire_session_log_tab_and_clipboard']=True
 report['copied_log_bytes']=len(full.encode())
 report.update(capture_settings_persist=True,physical_shortcut=True,ocr_text=review['text'],ocr_status=review['status'],no_clipboard_injection=True,no_automatic_trade=True,cancellation=True,foreign_scope=True,max_status_ms=round(max(checks)*1000),topmost_fixture=True)
 (r/'dist/windows-ocr-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2),flush=True);success=True
finally:
 try:normal_command('quit')
 except Exception:pass
 try:command('quit')
 except Exception:pass
 for process in reversed(processes):
  if process.poll() is None:
   process.terminate()
   try:process.wait(timeout=3)
   except subprocess.TimeoutExpired:process.kill();process.wait()
 for log in logs:log.close()
 for name in ['fixture-ocr.bmp','fixture-ocr-advanced.bmp','fixture-local-ready.bmp','capture-black-accelerated','capture-all-blank','hide-cloud']:(root/name).unlink(missing_ok=True)
 server.shutdown()
 if not success:print('Diagnostics:',root)
