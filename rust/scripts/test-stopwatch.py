#!/usr/bin/env python3
"""Native multi-instance stopwatch on a private River/XWayland desktop. Never touches the real game."""
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
    original={'league':'Standard','automatic_search':False,'stopwatches':[{'id':1,'title':'Premier','x':50,'y':50,'wanted':True},{'id':2,'title':'Second','x':70,'y':65,'wanted':True}],'hotkeys':{'stopwatch_keys':[{'id':1,'toggle':'Ctrl + J','reset':'Ctrl + K'},{'id':2,'toggle':'Ctrl + L','reset':'Ctrl + U'}]}}
    configpath.write_text(json.dumps(original));original_bytes=configpath.read_bytes()
    daemon=start([str(app),'daemon'],'overlay')
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'initial shortcuts')
    focus_game()
    def clocks():return status()['stopwatches']
    def row(id):return next(r for r in clocks()['rows'] if r['id']==id)
    def key(value):subprocess.run(['xdotool','key','--clearmodifiers',value],env=env,check=True)
    def click(r,canvas=False):
        assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        o={'x':0,'y':0} if canvas else status()['origin']
        pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    assert not clocks()['visible'] and not clocks()['scheduled_tick']
    key('ctrl+j');wait_for(lambda:row(1)['running'] and clocks()['visible'] and clocks()['rendered'],'first timer passive')
    assert status()['game_active'] and clocks()['keyboard_mode']=='None' and not clocks()['focused'] and not row(2)['visible']
    clicks=fixture_command('input')['clicks'];pointer(1280,800,True)
    wait_for(lambda:fixture_command('input')['clicks']==clicks+1,'clock passes click to fake game')
    key('ctrl+l');wait_for(lambda:row(2)['running'] and row(2)['visible'],'second independent timer')
    key('ctrl+k');wait_for(lambda:1000<=row(1)['millis']<1800,'reset running keeps one second')
    assert row(2)['running'];time.sleep(1.1)
    key('ctrl+j');wait_for(lambda:not row(1)['running'] and row(1)['millis']>=1000,'pause retains passive clock')
    assert row(1)['visible'] and row(2)['running']
    report['per_instance_shortcuts_reset_pause_and_pass_through']=True
    # Explicit interaction works with the main panel hidden, after canvas paint.
    key('shift+space');wait_for(lambda:state(state='Interactive') and clocks()['keyboard_mode']=='OnDemand','HUD interaction')
    r=row(1)['rect'];pointer(r['x']+r['width']/2,r['y']+r['height']-15)
    wait_for(lambda:row(1)['controls']['toggle'] and row(1)['controls']['toggle']['mapped'],'hover controls')
    click(row(1)['controls']['toggle'],True);wait_for(lambda:row(1)['running'] and clocks()['focused'],'native clock starts')
    click(row(1)['controls']['reset'],True);wait_for(lambda:1000<=row(1)['millis']<1800,'native clock reset')
    report['native_controls_and_hidden_main_interaction']=True
    # Hold-to-delete must cancel on release; dragging updates the saved center anchor.
    def hover(id):
        r=row(id)['rect'];pointer(r['x']+r['width']/2,r['y']+r['height']-15)
        wait_for(lambda:row(id)['controls']['move'] and row(id)['controls']['move']['mapped'],'toolbar visible')
        time.sleep(.06)
    hover(1);r=row(1)['controls']['remove'];
    subprocess.run([str(target/'debug/examples/pointer'),str(r['x']+r['width']//2),str(r['y']+r['height']//2),'2560','1600','--hold','150'],env=env,check=True)
    time.sleep(1.1);assert len(clocks()['rows'])==2 and not status()['preferences_pending']
    hover(1);click(row(1)['controls']['move'],True)
    r=row(1)['rect'];x=round(row(1)['x']/100*2560);y=round(row(1)['y']/100*1600)
    subprocess.run([str(target/'debug/examples/pointer'),str(x),str(y),'2560','1600','--drag-to','1450','900'],env=env,check=True)
    wait_for(lambda:abs(row(1)['x']-1450/2560*100)<.01 and abs(row(1)['rect']['x']+row(1)['rect']['width']/2-1450)<2 and not status()['preferences_pending'] and abs(json.loads(configpath.read_text())['stopwatches'][0]['x']-1450/2560*100)<.01,'drag center saved')
    hover(1);click(row(1)['controls']['move'],True)
    wait_for(lambda:not row(1)['moving'],'movement mode left')
    subprocess.run(['grim','/tmp/apt-v17-private-stopwatch.png'],env=env,check=True)
    report['native_hold_cancellation_and_drag_persistence']=True

    # Losing focus to a foreign app hides every input region. Sticky HUD returns passively.
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True)
    other=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',str(first['other'])],env=env,text=True).splitlines())
    pointer(int(other['X'])+30,int(other['Y'])+30)
    wait_for(lambda:not status()['game_active'] and not clocks()['visible'] and clocks()['keyboard_mode']=='None','foreign app hides clocks')
    before=clocks();time.sleep(1.2);after=clocks()
    assert before['frames']==after['frames'] and not after['scheduled_tick'] and row(2)['running']
    focus_game();wait_for(lambda:clocks()['visible'] and clocks()['rendered'],'sticky clocks return passively')
    assert status()['state']=='Hidden' and not status()['visible'] and clocks()['keyboard_mode']=='None' and not clocks()['focused']
    report['foreign_app_hides_and_return_is_passive_no_hidden_ticks']=True
    # Exclusive price/settings suppress clocks, manager restores them, explicit hide persists.
    command('demo');wait_for(lambda:state(state='Passive',rendered=True),'exclusive price')
    assert not clocks()['visible'] and not clocks()['scheduled_tick']
    command('stopwatches');wait_for(lambda:state(view='Stopwatches') and clocks()['visible'],'manager restores clocks')
    command('interact');wait_for(lambda:state(state='Interactive'),'manager interaction')
    click(status()['stopwatch_manager']['add']);wait_for(lambda:len(clocks()['rows'])==3 and not status()['preferences_pending'],'native add persisted')
    assert len(json.loads(configpath.read_text())['stopwatches'])==3
    report['native_manager_add_and_exclusive_views']=True
    # A one-second hold actually deletes only its target, including its keys.
    command('hide');focus_game();key('ctrl+j');wait_for(lambda:clocks()['visible'] and clocks()['rendered'],'clock resumed')
    key('shift+space');wait_for(lambda:state(state='Interactive'),'delete interaction')
    hover(2);r=row(2)['controls']['remove'];subprocess.run([str(target/'debug/examples/pointer'),str(r['x']+r['width']//2),str(r['y']+r['height']//2),'2560','1600','--hold','1150'],env=env,check=True)
    wait_for(lambda:len(clocks()['rows'])==2 and not status()['preferences_pending'],'hold deleted target')
    saved=json.loads(configpath.read_text());assert [c['id'] for c in saved['stopwatches']]==[1,3] and [k['id'] for k in saved['hotkeys']['stopwatch_keys']]==[1]
    report['native_hold_deletion_targets_instance_and_removes_keys']=True


    # Detached editor: per-instance key capture, duplicate refusal and real IO rollback.
    command('hide');focus_game();command('settings');wait_for(lambda:state(state='Passive',rendered=True,view='Settings'),'timer settings passive')
    command('interact');wait_for(lambda:state(state='Interactive'),'timer settings interactive')
    click(status()['preferences']['tabs'][3]);wait_for(lambda:status()['preferences']['page']==3,'timer tab')
    def draft():return status()['preferences']['stopwatches']
    def drow():return next(r for r in draft()['rows'] if r['id']==1)
    def reveal(r):
        for _ in range(24):
            control=r();v=draft()['scroll'];y=control['y']+control['height']/2
            if v['y']+8<y<v['y']+v['height']-8:return control
            o=status()['origin'];subprocess.run([str(target/'debug/examples/pointer'),str(int(o['x']+v['x']+6)),str(int(o['y']+v['y']+v['height']/2)),'2560','1600','--scroll','70' if y>v['y']+v['height']/2 else '-70'],env=env,check=True);time.sleep(.06)
        raise RuntimeError('editor row not revealed')
    def write(r,value):
        click(reveal(r));subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--',value],env=env,check=True)
    saved_bytes=configpath.read_bytes()
    write(lambda:drow()['toggle']['rect'],'Ctrl + D')
    click(status()['preferences']['apply']);wait_for(lambda:'plusieurs fois' in status()['preferences']['message'],'timer key duplicates global')
    assert configpath.read_bytes()==saved_bytes
    click(reveal(lambda:drow()['record_toggle']));wait_for(lambda:draft()['capture']==[1,False],'timer record armed')
    subprocess.run(['wtype','-M','ctrl','-k','m','-m','ctrl'],env=env,check=True)
    wait_for(lambda:drow()['toggle']['value']=='Ctrl + M' and draft()['capture'] is None,'per-instance key recorded')
    write(lambda:drow()['title']['rect'],'Expedition')
    configdir.chmod(0o500)
    try:
        click(status()['preferences']['apply']);wait_for(lambda:'Modifications non enregistrées' in status()['preferences']['message'] and not status()['preferences_pending'],'timer file rollback')
    finally:configdir.chmod(0o700)
    assert configpath.read_bytes()==saved_bytes
    click(status()['preferences']['apply']);wait_for(lambda:state(state='Hidden') and not status()['preferences_pending'],'timer edit saved')
    saved=json.loads(configpath.read_text());assert saved['stopwatches'][0]['title']=='Expedition' and saved['hotkeys']['stopwatch_keys'][0]['toggle']=='Ctrl + M'
    focus_game();was=row(1)['running'];key('ctrl+j');time.sleep(.1);assert row(1)['running']==was
    key('ctrl+m');wait_for(lambda:row(1)['running']!=was,'new instance shortcut acts')
    report['native_instance_editor_capture_conflict_rollback_rebind']=True
    command('hide');focus_game();time.sleep(.2)
    assert not clocks()['visible'] and clocks()['keyboard_mode']=='None' and not clocks()['scheduled_tick']
    # Hidden zero/paused widgets open the manager only on explicit interaction, after its first frame.
    command('interact');wait_for(lambda:state(state='Interactive',view='Stopwatches',rendered=True),'manager deferred interaction')
    report['explicit_hidden_manager_interaction_after_paint']=True
    # Transfer between the manager and widget canvas without dismissing either.
    hover(1);click(row(1)['controls']['toggle'],True);time.sleep(.1)
    assert status()['state']=='Interactive' and status()['visible'] and clocks()['focused']
    click(status()['stopwatch_manager']['rows'][0]['reset']);time.sleep(.1)
    assert status()['state']=='Interactive' and status()['visible']
    report['focus_transfers_between_owned_surfaces']=True
    # The widget toolbar releases its surface before opening the exclusive editor.
    for _ in range(5):
        hover(1);click(row(1)['controls']['edit'],True)
        wait_for(lambda:state(state='Passive',view='Settings',rendered=True) and status()['preferences']['page']==3,'widget edit handoff')
        assert status()['game_active'] and not clocks()['visible'] and clocks()['keyboard_mode']=='None'
        command('hide');focus_game();command('stopwatches');wait_for(lambda:state(state='Passive',view='Stopwatches',rendered=True),'manager for next handoff')
        command('interact');wait_for(lambda:state(state='Interactive'),'owned handoff interaction')
    report['native_settings_handoffs']=5
    # Paused widgets need no tick; output dimensions and anchors track private scale changes.
    command('hide');focus_game();key('ctrl+m');time.sleep(.1)
    dimensions=[]
    for w,h,scale,lw,lh in [(1920,1080,1,1920,1080),(2560,1600,2,1280,800),(2560,1600,1,2560,1600)]:
        subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode',f'{w}x{h}@60Hz','--scale',str(scale)],env=env,check=True)
        wait_for(lambda:clocks()['viewport']==[lw,lh] and clocks()['rendered'] and abs(row(1)['rect']['x']+row(1)['rect']['width']/2-row(1)['x']/100*lw)<2,'clock output resized and child placed')
        rr=row(1);assert abs(rr['rect']['x']+rr['rect']['width']/2-rr['x']/100*lw)<2
        assert clocks()['keyboard_mode']=='None' and not clocks()['focused']
        dimensions.append({'physical':[w,h],'scale':scale,'logical':[lw,lh]})
    report['private_output_reconfiguration']=dimensions

    command('hide');focus_game()
    assert 'trade search generation=' not in (runtime/'overlay.log').read_text()
    report.update(no_network=True,scope='Private River/XWayland with fake game only',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-stopwatch.py','src/stopwatch.rs','src/ui/stopwatch_panel.rs','src/ui.rs']}
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
