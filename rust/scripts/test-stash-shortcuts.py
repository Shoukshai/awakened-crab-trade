#!/usr/bin/env python3
"""Native stash shortcuts and XRecord wheel policy on a private River/XWayland desktop. Never touches the real game."""
import hashlib,json,os,pathlib,select,shutil,subprocess,sys,tempfile,time,urllib.parse
from PIL import Image
root=pathlib.Path(__file__).resolve().parent.parent
os.chdir(root)
for name in ['river','riverctl','wlr-randr','xdotool','wl-copy','wtype','xkbcli']:
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
env=os.environ.copy();env.update(XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(runtime/'config'),XDG_CACHE_HOME=str(runtime/'cache'),XDG_STATE_HOME=str(runtime/'state'),XDG_DATA_HOME=str(runtime/'data'),GSETTINGS_BACKEND='memory',WLR_BACKENDS='headless',WLR_HEADLESS_OUTPUTS='1',WLR_RENDERER='pixman',GDK_BACKEND='wayland',HTTPS_PROXY='http://127.0.0.1:9',HTTP_PROXY='http://127.0.0.1:9',ALL_PROXY='http://127.0.0.1:9')
if os.environ.get('APT_TEST_NO_RECORD')=='1':
    wrapper=runtime/'Xwayland-no-record';wrapper.write_text('#!/bin/sh\nexec /usr/bin/Xwayland -extension RECORD "$@"\n');wrapper.chmod(0o700);env['WLR_XWAYLAND']=str(wrapper)
OUTPUT=[2560,1600]
logs=[];processes=[];success=False;report={}
def start(args,name,**kwargs):
    if name=='overlay' and os.environ.get('APT_IMAGE_GDB')=='1':
        args=['gdb','--batch','-ex','set print thread-events off','-ex','set environment G_DEBUG fatal-criticals','-ex','run','-ex','bt 40','--args']+args
    log=open(runtime/(name+'.log'),'w');logs.append(log)
    p=subprocess.Popen(args,env=(dict(env,WAYLAND_DEBUG='client') if name=='overlay' and os.environ.get('APT_MENU_WAYLAND_DEBUG')=='1' else env),stderr=log,**({'stdout':log}|kwargs));processes.append(p);return p
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
    args=[str(target/'debug/examples/pointer'),str(int(x)),str(int(y)),str(OUTPUT[0]),str(OUTPUT[1])]
    if click:args+=['--click']
    subprocess.run(args,env=env,check=True)
def focus_game():
    subprocess.run(['riverctl','set-focused-tags','1'],env=env,check=True)
    pointer(OUTPUT[0]*.8,OUTPUT[1]*.75)
    wait_for(lambda:state(game_active=True),'game active')
def fixture_command(name):
    fixture.stdin.write(name+'\n');fixture.stdin.flush()
    assert select.select([fixture.stdout],[],[],3)[0],'fixture not responding'
    return json.loads(fixture.stdout.readline())
def show_price():
    focus_game();command('check');s=wait_for(lambda:state(state='Passive',rendered=True,visible=True),'price visible')
    assert s['game_active'] and s['keyboard_mode']=='None',s
    return s
def finish():
    global success
    report.update(no_network=True,scope='Private River/XWayland with GTK fake game only; no real wheel, similar-search or Enter injected',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-stash-shortcuts.py','scripts/compare-stash-scroll.cjs','../main/src/shortcuts/Shortcuts.ts','../main/src/windowing/GameWindow.ts','examples/native-key.rs','examples/stash-fixture.rs','src/stash_scroll.rs','src/desktop/wheel.rs','src/desktop/xwayland_shortcuts.rs','src/desktop/game_input.rs','src/desktop/clipboard.rs','src/hotkeys.rs','src/settings.rs','src/ui.rs','src/ui/settings_panel.rs']}
    assert '-CRITICAL' not in (runtime/'overlay.log').read_text();assert 'trade search generation=' not in (runtime/'overlay.log').read_text()
    if len(sys.argv)>2:pathlib.Path(sys.argv[2]).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2));success=True
try:
    river=start(['river','-c',str(init)],'river');wait_for(lambda:(runtime/'display').exists() and (runtime/'wayland').exists(),'private compositor')
    display=(runtime/'display').read_text().strip();wayland=(runtime/'wayland').read_text().strip();assert display!=os.environ.get('DISPLAY');env.update(DISPLAY=display,WAYLAND_DISPLAY=wayland)
    subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode','2560x1600@60Hz'],env=env,check=True)
    foreign=start([str(target/'debug/examples/overlay-fixture'),'--isolated'],'foreign',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True);assert select.select([foreign.stdout],[],[],3)[0];first=json.loads(foreign.stdout.readline());foreign.stdin.write('unmap\n');foreign.stdin.flush();assert select.select([foreign.stdout],[],[],3)[0];json.loads(foreign.stdout.readline())
    keyboard=start(['wtype','-s','60000','-k','Escape'],'keyboard')
    fixture=start([str(target/'debug/examples/stash-fixture')],'fixture',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True);assert select.select([fixture.stdout],[],[],5)[0];assert json.loads(fixture.stdout.readline())['ready']
    configdir=pathlib.Path(env['XDG_CONFIG_HOME'])/'awakened-poe-trade-rs';configdir.mkdir(parents=True);configpath=configdir/'config.json';configpath.write_text(json.dumps({'league':'Standard','automatic_search':False,'restore_clipboard':True,'hotkeys':{'similar':'Ctrl + K'}}))
    daemon=start([str(app),'daemon'],'overlay');wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket');wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'XWayland keys');wait_for(lambda:('Err' if os.environ.get('APT_TEST_NO_RECORD')=='1' else 'Ok') in status()['stash_scroll']['status'] and 'Initialisation' not in str(status()['stash_scroll']['status']),'XRecord capability');focus_game()
    def arrows():return [e for e in fixture_command('input')['key_events'] if e['key'] in [0xff51,0xff53]]
    observations=[]
    def pulse(x,y,button=5,ctrl=True,shift=False,wayland=False):
        pointer(x,y);time.sleep(.05);before=arrows();wheels=len(fixture_command('input')['wheels'])
        if OUTPUT[0]==1280:
            geom=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowfocus','getwindowgeometry','--shell'],env=env,text=True).splitlines());mouse=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getmouselocation','--shell'],env=env,text=True).splitlines())
            observations.append({'ctrl':ctrl,'active':True,'enabled':True,'x':int(mouse['X']),'y':int(mouse['Y']),'rotation':-1 if button in [4,6] else 1,'bounds':{'x':int(geom['X']),'y':int(geom['Y']),'width':int(geom['WIDTH']),'height':int(geom['HEIGHT'])}})
        mods=(['ctrl'] if ctrl else [])+(['shift'] if shift else [])
        for mod in mods:subprocess.run(['xdotool','keydown',mod],env=env,check=True)
        try:
            if wayland:subprocess.run([str(target/'debug/examples/pointer'),str(x),str(y),str(OUTPUT[0]),str(OUTPUT[1]),'--scroll','10' if button==5 else '-10'],env=env,check=True)
            else:subprocess.run(['xdotool','click',str(button)],env=env,check=True)
            wait_for(lambda:len(fixture_command('input')['wheels'])>wheels,'original wheel still reaches fake game');time.sleep(.22)
            wait_for(lambda:status()['stash_pending'] is None,'wheel jobs settled')
            return arrows()[len(before):]
        finally:
            for mod in reversed(mods):subprocess.run(['xdotool','keyup',mod],env=env,check=True)
    if os.environ.get('APT_TEST_NO_RECORD')=='1':
        assert status()['stash_scroll']['wanted'] and 'Err' in status()['stash_scroll']['status']
        assert not pulse(1800,600,wayland=True)
        subprocess.run([str(target/'debug/examples/native-key'),'shift-space'],env=env,check=True);wait_for(lambda:state(view='Menu',state='Interactive') and status()['widget_menu']['rendered'],'physical Wayland shortcut survives missing test extensions')
        command('hide');focus_game()
        text=(root/'src/fixtures/driftwood-wand.txt').read_text();owner=start(['wl-copy','--foreground','--type','text/plain;charset=utf-8'],'fallback-clipboard',stdin=subprocess.PIPE,text=True);owner.stdin.write(text);owner.stdin.close();time.sleep(.06)
        command('inspect-check');wait_for(lambda:state(view='Inspection',state='Passive',rendered=True) and status()['inspection']['dps']['physical']==11,'manual clipboard read without XRecord/XTEST');command('hide');focus_game()
        command('settings');wait_for(lambda:state(view='Settings',state='Passive',rendered=True),'fallback settings');command('interact');wait_for(lambda:state(state='Interactive'),'fallback explicit interaction')
        assert not status()['preferences']['stash_scroll']['sensitive']
        report['missing_xrecord_keeps_original_wheel_physical_overlay_shortcut_and_manual_clipboard_read']=True
        report['capability_error']=status()['stash_scroll']['status']['Err']
        report['private_server_also_disables_xtest_no_synthetic_copy_or_search_claimed']=True
        command('hide');finish();sys.exit(0)
    out=pulse(1800,600,wayland=True);assert out and all(e['key']==0xff53 and e['state']&4 for e in out),out
    assert not pulse(500,600) and not pulse(1800,600,ctrl=False)
    out=pulse(1800,600,button=4);assert len(out)==1 and out[0]['key']==0xff51 and out[0]['state']&4,out
    out=pulse(1800,600,button=7,shift=True);assert len(out)==1 and out[0]['key']==0xff53 and out[0]['state']&5==5,out
    assert status()['state']=='Hidden' and not status()['visible'] and not status()['widget_canvas']['visible']
    report['wayland_wheel_seen_by_xrecord_and_original_event_preserved']=True
    report['ctrl_directions_sidebar_exclusion_and_modifiers']=True
    # Every core wheel step in a short burst must retain its direction and modifier.
    pointer(1800,600);before=len(arrows());original_wheels=len(fixture_command('input')['wheels']);subprocess.run(['xdotool','keydown','ctrl'],env=env,check=True)
    try:
        subprocess.run(['xdotool','click','--repeat','10','--delay','20','5'],env=env,check=True)
        time.sleep(.5);wait_for(lambda:status()['stash_pending'] is None,'wheel burst drained')
        out=arrows()[before:];assert len(out)==10 and all(e['key']==0xff53 and e['state']&4 for e in out),(out,status()['stash_scroll'])
        assert len(fixture_command('input')['wheels'])-original_wheels==10
    finally:subprocess.run(['xdotool','keyup','ctrl'],env=env,check=True)
    report['ten_step_burst_preserves_original_wheel_and_all_tab_steps']=True
    # The similar-item hotkey performs a guarded copy and quoted search with no panel or HTTP.
    before=len(fixture_command('input')['submitted']);frames=status()['widget_canvas']['frames'];main_frames=status()['rendered']
    owner=start(['wl-copy','--foreground','--type','text/plain;charset=utf-8','clipboard before similar copy'],'clipboard');time.sleep(.08)
    subprocess.run(['xdotool','key','--clearmodifiers','ctrl+k'],env=env,check=True)
    wait_for(lambda:len(fixture_command('input')['submitted'])==before+1,'similar copy and search');wait_for(lambda:status()['stash_pending'] is None and status()['capture_pending'] is None,'similar cleanup')
    assert fixture_command('input')['submitted'][-1]=='"Driftwood Wand"'
    assert subprocess.check_output(['wl-paste','--no-newline'],env=env,text=True)=='clipboard before similar copy'
    s=status();assert s['state']=='Hidden' and not s['visible'] and not s['widget_canvas']['visible'] and s['widget_canvas']['frames']==frames and s['request'] is None and s['query'] is None,s
    report['similar_hotkey_copies_before_action_restores_text_and_opens_no_overlay_or_network']=True
    # A slow clipboard snapshot must not send even the copy gesture after an away/return.
    prior=runtime/'prior.txt';prior.write_text('slow previous clipboard');provider=start([str(target/'debug/examples/slow-clipboard'),str(prior)],'slow-copy',stdout=subprocess.PIPE,bufsize=0)
    assert select.select([provider.stdout],[],[],3)[0] and provider.stdout.readline()==b'READY\n';time.sleep(.06)
    before=fixture_command('input');subprocess.run(['xdotool','key','--clearmodifiers','ctrl+k'],env=env,check=True);wait_for(lambda:status()['capture_pending'] is not None,'delayed similar capture pending')
    assert select.select([provider.stdout],[],[],3)[0] and provider.stdout.readline()==b'REQUEST\n'
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True);pointer(30,30);wait_for(lambda:not status()['game_active'],'capture loses game');focus_game();time.sleep(1.)
    after=fixture_command('input');assert after['submitted']==before['submitted'] and after['keys'].count(ord('c'))==before['keys'].count(ord('c')) and status()['state']=='Hidden' and status()['capture_pending'] is None
    report['late_snapshot_cancelled_before_copy_gesture_after_away_and_return']=True
    # Settings replace the eighth shortcut and publish the wheel flag only on commit.
    def edit():
        command('hide');focus_game();command('settings');wait_for(lambda:state(view='Settings',state='Passive',rendered=True),'settings passive');command('interact');wait_for(lambda:state(state='Interactive'),'settings interactive');time.sleep(.1)
        q=status();pointer(q['origin']['x']+q['width']/2,q['origin']['y']+12,True);wait_for(lambda:status()['overlay_focused'] and not status()['game_active'],'explicit header click owns settings focus')
    def click(r):
        q=status();assert q['state']=='Interactive' and q['keyboard_mode']=='OnDemand' and r['mapped'];o=q['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def reveal(fn,which):
        for _ in range(32):
            r=fn();v=status()['preferences'][which];y=r['y']+r['height']/2
            if v['y']+5<y<v['y']+v['height']-5:return r
            o=status()['origin'];pointer(o['x']+v['x']+12,o['y']+v['y']+v['height']/2);time.sleep(.06)
            assert not status()['game_active'],'settings owns input before scrolling'
            subprocess.run([str(target/'debug/examples/pointer'),str(int(o['x']+v['x']+12)),str(int(o['y']+v['y']+v['height']/2)),str(OUTPUT[0]),str(OUTPUT[1]),'--scroll','80' if y>v['y'] else '-80'],env=env,check=True);time.sleep(.07)
        raise RuntimeError('native settings control clipped')
    def change():
        edit();click(reveal(lambda:status()['preferences']['stash_scroll']['rect'],'general_scroll'));assert not status()['preferences']['stash_scroll']['value']
        click(status()['preferences']['tabs'][1]);wait_for(lambda:status()['preferences']['page']==1,'shortcut page');click(reveal(lambda:status()['preferences']['record'][7],'shortcuts_scroll'))
        wait_for(lambda:status()['preferences']['capture']==7,'similar capture armed');subprocess.run(['wtype','-M','ctrl','-k','l','-m','ctrl'],env=env,check=True);wait_for(lambda:status()['preferences']['hotkeys'][7]['value']=='Ctrl + L' and status()['preferences']['capture'] is None,'new similar key captured')
    change()
    saved=configpath.read_bytes();configdir.chmod(0o500)
    try:
        click(status()['preferences']['apply']);wait_for(lambda:not status()['preferences_pending'] and 'non enregistrées' in status()['preferences']['message'],'real settings IO failure');assert configpath.read_bytes()==saved and status()['stash_scroll']['wanted']
    finally:configdir.chmod(0o700)
    click(status()['preferences']['cancel']);command('hide');focus_game()
    assert len(pulse(1800,600))==1,'previous wheel setting survives failed commit'
    before=len(fixture_command('input')['submitted']);subprocess.run(['xdotool','key','--clearmodifiers','ctrl+k'],env=env,check=True);wait_for(lambda:len(fixture_command('input')['submitted'])==before+1,'previous similar binding survives failed commit');wait_for(lambda:status()['stash_pending'] is None,'rollback search settled')
    change()
    click(status()['preferences']['apply']);wait_for(lambda:state(state='Hidden') and not status()['preferences_pending'],'wheel flag and key committed');focus_game()
    assert json.loads(configpath.read_text())['stash_scroll'] is False and json.loads(configpath.read_text())['hotkeys']['similar']=='Ctrl + L'
    assert not pulse(1800,600,wayland=True)
    before=len(fixture_command('input')['submitted']);subprocess.run(['xdotool','key','--clearmodifiers','ctrl+k'],env=env,check=True);time.sleep(.2);assert len(fixture_command('input')['submitted'])==before
    subprocess.run(['xdotool','key','--clearmodifiers','ctrl+l'],env=env,check=True);wait_for(lambda:len(fixture_command('input')['submitted'])==before+1,'replacement similar key works');wait_for(lambda:status()['stash_pending'] is None,'replacement search finished')
    report['native_similar_key_capture_wheel_toggle_io_rollback_and_rebind']=True
    edit();click(reveal(lambda:status()['preferences']['stash_scroll']['rect'],'general_scroll'));click(status()['preferences']['apply']);wait_for(lambda:state(state='Hidden') and not status()['preferences_pending'],'wheel reenabled');assert 'stash_scroll' not in json.loads(configpath.read_text());focus_game()
    # Changing to another application wins over the wheel observer.
    before=len(arrows());subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True);pointer(30,30);wait_for(lambda:not status()['game_active'],'foreign active')
    subprocess.run(['xdotool','keydown','ctrl','click','5','keyup','ctrl'],env=env,check=True);time.sleep(.2);assert len(arrows())==before and status()['state']=='Hidden';focus_game()
    report['foreign_app_wheel_does_not_send_game_arrows']=True
    # Reconfiguration uses actual X client bounds, including a factor-2 output.
    subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode','2560x1600@60Hz','--scale','2'],env=env,check=True);OUTPUT[:]=[1280,800];time.sleep(.2);focus_game()
    actual=[]
    for x,y in [(300,300),(493,300),(494,300),(495,300),(987,300),(988,300),(300,154),(300,155),(300,77),(300,595),(300,596),(300,597),(1000,300)]:
        out=pulse(x,y);assert all(e['key']==0xff53 and e['state']&4 for e in out),out;actual.append('right' if out else None)
    expected=json.loads(subprocess.check_output(['node','scripts/compare-stash-scroll.cjs','--evaluate'],input=json.dumps(observations),env=env,text=True))
    assert actual==[c['action'] for c in expected],(observations,actual,expected)
    report['private_factor_2_uses_actual_x_coordinates_and_upstream_policy']={'observations':observations,'actions':actual}
    subprocess.run(['wlr-randr','--output','HEADLESS-1','--scale','1'],env=env,check=True);OUTPUT[:]=[2560,1600];time.sleep(.2);focus_game()
    # Restart retains the option and shortcut, begins hidden, and creates one observer.
    saved=configpath.read_bytes();command('quit');assert daemon.wait(timeout=5)==0
    daemon=start([str(app),'daemon'],'overlay-restart');wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'restart socket');wait_for(lambda:'Ok' in status()['shortcut_backend']['status'] and 'Ok' in status()['stash_scroll']['status'],'restart backends');focus_game()
    assert status()['state']=='Hidden' and configpath.read_bytes()==saved
    out=pulse(1800,600);assert len(out)==1 and out[0]['key']==0xff53,out
    report['restart_retains_shortcut_and_option_without_duplicate_observer']=True
    # The same observer survives the disappearance and recreation of the game window.
    fixture.terminate();assert fixture.wait(timeout=5) in [0,-15]
    wait_for(lambda:not any(w['app_id']=='steam_app_238960' for w in status()['desktop']['windows']),'game absent')
    fixture=start([str(target/'debug/examples/stash-fixture')],'fixture-recreated',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True);assert select.select([fixture.stdout],[],[],5)[0] and json.loads(fixture.stdout.readline())['ready'];focus_game()
    assert 'Ok' in status()['stash_scroll']['status'];out=pulse(1800,600);assert len(out)==1 and out[0]['key']==0xff53
    report['observer_survives_game_window_disappearance_and_recreation']=True
    assert '-CRITICAL' not in (runtime/'overlay-restart.log').read_text()
    finish()
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
