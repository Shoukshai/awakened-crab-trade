#!/usr/bin/env python3
"""Native Delve grid on a private River/XWayland desktop. Never touches the real game."""
import hashlib,json,os,pathlib,select,shutil,subprocess,sys,tempfile,time,urllib.parse
from PIL import Image
root=pathlib.Path(__file__).resolve().parent.parent
os.chdir(root)
for name in ['river','riverctl','wlr-randr','xdotool','wl-copy','wtype']:
    assert shutil.which(name),name+' required'
subprocess.run(['cargo','build','--locked','--examples','-j','3'],check=True)
app=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else root/'target/release/awakened-poe-trade-rs').resolve()
target=pathlib.Path(os.environ.get('CARGO_TARGET_DIR',root/'target')).resolve()
runtime=pathlib.Path(tempfile.mkdtemp(prefix='apt-overlay-desktop-'));runtime.chmod(0o700)
init=runtime/'init.sh'
init.write_text('''#!/bin/sh
printf '%s\\n' "$DISPLAY" > "$XDG_RUNTIME_DIR/display"
printf '%s\\n' "$WAYLAND_DISPLAY" > "$XDG_RUNTIME_DIR/wayland"
riverctl focus-follows-cursor always
riverctl rule-add -app-id steam_app_238960 tags 1
riverctl rule-add -app-id steam_app_238960 fullscreen
riverctl rule-add -app-id apt_foreign_fixture tags 2
''');init.chmod(0o700)
env=os.environ.copy();env.update(XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(runtime/'config'),XDG_CACHE_HOME=str(runtime/'cache'),XDG_STATE_HOME=str(runtime/'state'),WLR_BACKENDS='headless',WLR_HEADLESS_OUTPUTS='1',WLR_RENDERER='pixman',GDK_BACKEND='wayland',HTTPS_PROXY='http://127.0.0.1:9',HTTP_PROXY='http://127.0.0.1:9',ALL_PROXY='http://127.0.0.1:9')
logs=[];processes=[];success=False;report={}
def start(args,name,**kwargs):
    log=open(runtime/(name+'.log'),'w');logs.append(log)
    p=subprocess.Popen(args,env=env,stderr=log,**({'stdout':log}|kwargs));processes.append(p);return p
def wait_for(fn,label,seconds=6):
    until=time.monotonic()+seconds;last=None
    while time.monotonic()<until:
        last=fn()
        if last:return last
        time.sleep(.025)
    raise RuntimeError(f'Timeout {label}: {last}')
def command(name):return subprocess.check_output([str(app),name],env=env,text=True)
def status():return json.loads(command('status'))
def state(**expected):
    s=status();return s if all(s[k]==v for k,v in expected.items()) else None
def pointer(x,y,click=False):
    args=[str(target/'debug/examples/pointer'),str(int(x)),str(int(y)),'2560','1600']
    if click:args+=['--click']
    subprocess.run(args,env=env,check=True)
def focus_game():
    subprocess.run(['riverctl','set-focused-tags','1'],env=env,check=True)
    pointer(2000,1200)
    wait_for(lambda:state(game_active=True),'game active')
def fixture_command(name):
    fixture.stdin.write(name+'\n');fixture.stdin.flush()
    assert select.select([fixture.stdout],[],[],3)[0],'fixture not responding'
    return json.loads(fixture.stdout.readline())
def show_price():
    focus_game();command('check');s=wait_for(lambda:state(state='Passive',rendered=True,visible=True),'price visible')
    assert s['game_active'] and s['keyboard_mode']=='None',s
    return s
try:
    river=start(['river','-c',str(init)],'river')
    wait_for(lambda:(runtime/'display').exists() and (runtime/'wayland').exists(),'private compositor')
    display=(runtime/'display').read_text().strip();wayland=(runtime/'wayland').read_text().strip()
    assert display!=os.environ.get('DISPLAY') and (str(runtime),wayland)!=(os.environ.get('XDG_RUNTIME_DIR'),os.environ.get('WAYLAND_DISPLAY')),'refusing current desktop'
    env.update(DISPLAY=display,WAYLAND_DISPLAY=wayland)
    subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode','2560x1600@60Hz'],env=env,check=True)
    fixture=start([str(target/'debug/examples/overlay-fixture'),'--isolated'],'fixture',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    assert select.select([fixture.stdout],[],[],3)[0],'fixture not ready'
    first=json.loads(fixture.stdout.readline())
    # Keep a keyboard device present, as on the real desktop. No key is sent:
    # cleanup terminates wtype before its delayed Escape (private server only).
    keyboard=start(['wtype','-s','60000','-k','Escape'],'keyboard')
    time.sleep(.1)
    configdir=pathlib.Path(env['XDG_CONFIG_HOME'])/'awakened-poe-trade-rs';configdir.mkdir(parents=True)
    configpath=configdir/'config.json'
    maps=json.loads((root/'src/data/map-check-defaults.json').read_text())['config'];maps['selectedStats'][0]['decision']='gws'
    original={'league':'Standard','automatic_search':False,'hotkeys':{'delve':'Ctrl + G'}}
    configpath.write_text(json.dumps(original));original_bytes=configpath.read_bytes()
    daemon=start([str(app),'daemon'],'overlay')
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'initial shortcuts')
    focus_game()
    def grid():return status()['delve']
    def visible(w=2560,h=1600):
        s=status();g=s['delve'];geo=g['geometry']
        return s if g['visible'] and g['mapped'] and g['policy']['phase']=='Passive' and geo and geo['viewport_width']==w and geo['viewport_height']==h else None
    def show_grid(key=False):
        focus_game()
        if key:subprocess.run(['xdotool','key','--clearmodifiers','ctrl+g'],env=env,check=True)
        else:command('delve')
        s=wait_for(visible,'grid painted')
        assert s['game_active'] and s['delve']['keyboard_mode']=='None' and s['delve']['input_empty'] and not s['delve']['focused'],s
        assert subprocess.check_output(['xdotool','getwindowfocus','getwindowname'],env=env,text=True).strip()=='Path of Exile'
        return s
    def click(r):
        assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        o=status()['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    assert grid()['available'] and not grid()['visible']
    show_grid(True);report['configured_shortcut_shows_only_passive_grid']=True
    # Verify the actual rendering on an opaque fake-game background.
    image_path=runtime/'grid.png';subprocess.run(['grim',str(image_path)],env=env,check=True)
    image=Image.open(image_path);g=grid()['geometry'];frame_x=int(g['left']);y=g['top']+g['cell']//2
    samples={'outside':image.getpixel((20,20))[:3],'background':image.getpixel((frame_x+g['cell']//2,y))[:3],'clipped':image.getpixel((frame_x-2,y))[:3],'border':image.getpixel((round(g['grid_left']+g['cell']),y))[:3]}
    expected={'outside':(34,51,68),'background':(56,71,87),'clipped':(34,51,68),'border':(113,128,150)}
    for key,value in expected.items():assert all(abs(a-b)<=1 for a,b in zip(samples[key],value)),(key,samples,expected)
    report['rendered_pixels']=samples
    # Real pointer events must reach the fake game through both painted and transparent areas.
    clicks=fixture_command('input')['clicks']
    for x,y in [(20,20),(1280,700),(frame_x+1,g['top']+1),(frame_x+g['width']-2,g['top']+g['height']-2)]:
        pointer(x,y,True);clicks+=1
        wait_for(lambda:fixture_command('input')['clicks']>=clicks,'click passes through')
        assert fixture_command('input')['last_click']['window']==first['game']
        assert grid()['visible'] and not grid()['focused'] and status()['game_active']
    report['pass_through_clicks']=clicks
    time.sleep(.2);draws=grid()['draws'];frames=grid()['frames'];time.sleep(1)
    assert (grid()['draws'],grid()['frames'])==(draws,frames),'static grid redrew while idle'
    report['static_grid_no_redraws_1s']=True
    # Price/inspection/settings are exclusive: suppress, retain wanted state, then restore for a nonexclusive view.
    command('demo');wait_for(lambda:state(state='Passive',rendered=True),'passive main panel');assert not grid()['visible'] and grid()['exclusive_suppressed'] and grid()['policy']['phase']=='Passive'
    command('catalogue');wait_for(lambda:state(view='Catalogue') and grid()['visible'],'nonexclusive catalogue restores grid');assert not grid()['exclusive_suppressed']
    command('interact');wait_for(lambda:state(state='Interactive'),'main interaction');assert not grid()['visible'] and grid()['policy']['last_hide']=='Interactive'
    command('hide');focus_game();assert not grid()['visible']
    report['exclusive_suppression_nonexclusive_restore_and_hide_on_interaction']=True
    command('demo');wait_for(lambda:state(state='Passive',rendered=True),'exclusive price before new grid request')
    command('delve');wait_for(lambda:state(state='Hidden',game_active=True) and visible(),'new grid request dismisses exclusive panel')
    command('delve-hide');report['new_grid_request_dismisses_exclusive_panel']=True
    # Exercise explicit native-button handoff repeatedly, not only IPC or keyboard.
    for _ in range(20):
        command('demo');wait_for(lambda:state(state='Passive',rendered=True) and status()['delve_button']['width']>5,'main panel for grid button')
        command('interact');wait_for(lambda:state(state='Interactive'),'button interaction');click(status()['delve_button'])
        wait_for(lambda:state(state='Hidden',game_active=True) and visible(),'native button handoff')
        command('delve-hide');wait_for(lambda:not grid()['visible'],'grid only hidden');assert status()['state']=='Hidden'
    report['native_button_handoffs']=20
    show_grid();command('delve');wait_for(lambda:not grid()['visible'],'toggle off')
    assert not grid()['policy']['phase']=='Preparing';show_grid();command('hide');assert not grid()['visible']
    report['explicit_toggle_and_hide']=True
    # Other-application switch cancels requested state; returning to the game does not restore it.
    show_grid();subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True)
    other=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',str(first['other'])],env=env,text=True).splitlines())
    pointer(int(other['X'])+int(other['WIDTH'])/2,int(other['Y'])+int(other['HEIGHT'])/2)
    wait_for(lambda:not status()['game_active'] and not grid()['visible'] and grid()['policy']['phase']=='Hidden','foreign app cancels grid')
    command('delve');time.sleep(.05);assert not grid()['visible'] and grid()['policy']['phase']=='Hidden'
    focus_game();assert not grid()['visible'];report['application_switch_and_inactive_refusal']=True
    # Removal/recreation of the game also cancels requested visibility.
    show_grid();fixture_command('destroy');wait_for(lambda:not grid()['visible'] and grid()['policy']['last_hide']=='GameGone','game gone hides grid')
    fixture_command('create');focus_game();assert not grid()['visible'];report['game_recreation_does_not_restore_grid']=True
    # Reconfigure only the private output, first resolution, then output scale.
    show_grid();reconfigured=[]
    for w,h,scale,logical_w,logical_h in [(1920,1080,1,1920,1080),(2560,1600,2,1280,800),(2560,1600,1,2560,1600)]:
        subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode',str(w)+'x'+str(h)+'@60Hz','--scale',str(scale)],env=env,check=True)
        s=wait_for(lambda:visible(logical_w,logical_h),'resized grid')
        assert s['game_active'] and s['delve']['keyboard_mode']=='None' and not s['delve']['focused']
        reconfigured.append({'physical':[w,h],'output_scale':scale,'geometry':s['delve']['geometry'],'buffer_scale':s['delve']['buffer_scale']})
    report['private_output_reconfiguration']=reconfigured
    command('hide');assert not grid()['visible'] and grid()['keyboard_mode']=='None'
    assert configpath.read_bytes()==original_bytes
    report['widget_actions_keep_preferences_bytes']=True
    # Capture and apply the seventh native shortcut, then verify the actual grabs.
    command('settings');wait_for(lambda:state(state='Passive',rendered=True,view='Settings'),'shortcut settings passive')
    command('interact');wait_for(lambda:state(state='Interactive'),'shortcut settings interaction')
    click(status()['preferences']['tabs'][1]);wait_for(lambda:status()['preferences']['page']==1 and len(status()['preferences']['hotkeys'])==7 and status()['preferences']['record'][6] and status()['preferences']['record'][6]['width']>5,'seventh shortcut mapped')
    for _ in range(24):
        p=status()['preferences'];r=p['record'][6];v=p['shortcuts_scroll'];y=r['y']+r['height']/2
        if v['y']+10<y<v['y']+v['height']-10:break
        o=status()['origin'];subprocess.run([str(target/'debug/examples/pointer'),str(int(o['x']+v['x']+6)),str(int(o['y']+v['y']+v['height']/2)),'2560','1600','--scroll','60' if y>v['y']+v['height']/2 else '-60'],env=env,check=True);time.sleep(.08)
    else:raise RuntimeError('Delve shortcut did not scroll into view')
    click(status()['preferences']['record'][6]);wait_for(lambda:status()['preferences']['capture']==6,'Delve capture armed')
    subprocess.run(['wtype','-M','ctrl','-k','h','-m','ctrl'],env=env,check=True)
    wait_for(lambda:status()['preferences']['hotkeys'][6]['value']=='Ctrl + H' and status()['preferences']['capture'] is None,'Delve shortcut captured')
    click(status()['preferences']['apply']);wait_for(lambda:state(state='Hidden',game_active=True) and not status()['preferences_pending'],'Delve shortcut saved')
    assert json.loads(configpath.read_text())['hotkeys']['delve']=='Ctrl + H'
    subprocess.run(['xdotool','key','--clearmodifiers','ctrl+g'],env=env,check=True);time.sleep(.1);assert not grid()['visible']
    subprocess.run(['xdotool','key','--clearmodifiers','ctrl+h'],env=env,check=True);wait_for(visible,'new Delve binding acts')
    command('hide');report['native_shortcut_capture_save_and_rebind']=True
    assert 'trade search generation=' not in (runtime/'overlay.log').read_text() and 'economy network_' not in (runtime/'overlay.log').read_text()
    report.update(no_network=True,scope='Private headless River/XWayland, fake game and local pointer events only; pixel sampling and output changes confined to private compositor',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-delve.py','src/delve.rs','src/ui/delve_grid.rs','src/ui.rs','examples/overlay-fixture.rs','src/hotkeys.rs','src/ui/settings_panel.rs']}
    if len(sys.argv)>2:pathlib.Path(sys.argv[2]).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2));success=True

finally:
    if not success:
        try:(runtime/'status.json').write_text(command('status'))
        except Exception:pass
    if not success and shutil.which('grim') and 'WAYLAND_DISPLAY' in env:
        subprocess.run(['grim',str(runtime/'failure.png')],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=3)
    for p in reversed(processes):
        if p.poll() is None:
            p.terminate()
            try:p.wait(timeout=3)
            except subprocess.TimeoutExpired:p.kill();p.wait()
    for log in logs:log.close()
    if success:shutil.rmtree(runtime)
    else:print('Diagnostic logs preserved at '+str(runtime),file=sys.stderr)
