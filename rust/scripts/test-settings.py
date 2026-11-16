#!/usr/bin/env python3
"""Native settings on a private River/XWayland desktop. Never touches the real game."""
import hashlib,json,os,pathlib,select,shutil,subprocess,sys,tempfile,time,urllib.parse
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
    original={'league':'Standard','filters':{'stats':[{'id':'explicit.stat_3299347043','min':12}]},'map_check':maps}
    configpath.write_text(json.dumps(original));original_bytes=configpath.read_bytes()
    daemon=start([str(app),'daemon'],'overlay')
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'initial shortcuts')
    focus_game()
    def pref():return status()['preferences']
    def click(r):
        assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        o=status()['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def tab(index):
        click(pref()['tabs'][index]);wait_for(lambda:pref()['page']==index and (index!=1 or all(c['rect'] and c['rect']['mapped'] and c['rect']['width']>5 for c in pref()['hotkeys'])),'settings tab')
    def reveal(name):
        for _ in range(32):
            p=pref();r=p[name]['rect'];v=p['general_scroll'];y=r['y']+r['height']/2
            if v['y']+8<y<v['y']+v['height']-8:return r
            o=status()['origin'];subprocess.run([str(target/'debug/examples/pointer'),str(int(o['x']+v['x']+6)),str(int(o['y']+v['y']+v['height']/2)),'2560','1600','--scroll','60' if y>v['y']+v['height']/2 else '-60'],env=env,check=True);time.sleep(.08)
        raise RuntimeError('settings control did not scroll into view: '+name)
    def write(r,value):
        click(r);time.sleep(.06);args=['wtype','-M','ctrl','-k','a','-m','ctrl']
        # A blank draft uses a space, exercising trim + validation without
        # relying on virtual-keyboard navigation/editing keys.
        args+=['--',str(value) if value else ' ']
        subprocess.run(args,env=env,check=True)
        time.sleep(.05)
    def entry(name,value):
        tab(0);write(reveal(name),value)
        wait_for(lambda:float(pref()[name].get('text',pref()[name]['value']))==float(value) if name in ['scale','range'] else str(pref()[name]['value'])==str(value),'draft '+name)
    def checkbox(name,desired):
        tab(0);r=reveal(name)
        if pref()[name]['value']!=desired:click(r)
        wait_for(lambda:pref()[name]['value']==desired,'draft '+name)
    def next_currency(name, expected):
        tab(0);r=reveal(name);click(r);time.sleep(.15)
        o=status()['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']*1.5,True)
        wait_for(lambda:pref()[name]['value']==expected,'draft '+name)
    def map_preferences(profile, show_new):
        tab(2);p=pref();r=p['profile']['rect'];o=status()['origin']
        assert r and r['mapped'],r
        pointer(o['x']+r['x']+r['width']-10,o['y']+r['y']+r['height']/4,True)
        wait_for(lambda:pref()['profile']['value']==profile,'map profile')
        if pref()['show_new']['value']!=show_new:click(pref()['show_new']['rect'])
        wait_for(lambda:pref()['show_new']['value']==show_new,'new map mods')
    def open_settings():
        command('hide');focus_game();command('settings')
        s=wait_for(lambda:state(state='Passive',rendered=True,view='Settings'),'settings passive')
        assert s['game_active'] and s['keyboard_mode']=='None'
        command('interact');wait_for(lambda:state(state='Interactive'),'settings interactive')
    def native_key(index,value):
        tab(1);write(pref()['hotkeys'][index]['rect'],value)
        wait_for(lambda:pref()['hotkeys'][index]['value']==value,'hotkey draft')
    def save_error(fragment):
        click(pref()['apply']);wait_for(lambda:fragment in pref()['message'] and not status()['preferences_pending'],'settings validation error')
        assert status()['state']=='Interactive' and configpath.read_bytes()==original_bytes
    # Opening, defaults and cancellation never commit a draft or issue HTTP.
    open_settings();entry('scale',1.25);entry('range',25.0);click(pref()['reset']);time.sleep(.08);click(pref()['cancel']);wait_for(lambda:status()['view']=='Price','draft cancelled')
    assert configpath.read_bytes()==original_bytes and status()['settings']['scale']==1 and status()['view']=='Price'
    assert 'trade search generation=' not in (runtime/'overlay.log').read_text()
    # Invalid league, duplicate and reserved shortcut all leave file and grabs intact.
    open_settings();tab(0);write(reveal('league'),'');save_error('League')
    entry('league','Standard');native_key(1,'Ctrl + D');save_error('plusieurs fois')
    native_key(1,'Ctrl + Alt + D');native_key(0,'Ctrl + Alt + C');save_error('réservé')
    native_key(0,'Ctrl + D')
    # Capture real Wayland keyboard events into the detached shortcut draft.
    tab(1);click(pref()['record'][4]);wait_for(lambda:pref()['capture']==4,'capture armed')
    subprocess.run(['wtype','-M','ctrl','-k','g','-m','ctrl'],env=env,check=True)
    wait_for(lambda:pref()['hotkeys'][4]['value']=='Ctrl + G' and pref()['capture'] is None,'shortcut captured')
    report['native_key_capture']=True
    entry('scale',1.25);entry('range',25.0)
    # Simulate a real file-system refusal after native grabs have been acquired.
    configdir.chmod(0o500)
    try:save_error('Modifications non enregistrées')
    finally:configdir.chmod(0o700)
    assert status()['settings']['scale']==1 and status()['settings']['hotkeys']['settings'] is None
    command('hide');focus_game();command('demo');wait_for(lambda:state(state='Passive',rendered=True),'old controls after IO failure')
    subprocess.run(['xdotool','key','--clearmodifiers','shift+space'],env=env,check=True)
    wait_for(lambda:state(state='Interactive'),'old shortcut retained after IO refusal')
    report['io_failure_preserves_preferences_and_shortcuts']=True
    # Persist the current implemented fields, retaining invisible advanced rules.
    open_settings();entry('scale',1.25);entry('range',25.0);checkbox('automatic',False);checkbox('stock',True);checkbox('merchant',False);checkbox('collapse',False)
    checkbox('online',False);next_currency('price_currency','chaos_divine');next_currency('bulk_currency','divine');map_preferences(2,True)
    native_key(4,'Ctrl + G');click(pref()['apply'])
    wait_for(lambda:state(state='Hidden'),'settings saved and hidden')
    saved=json.loads(configpath.read_text());assert saved['scale']==1.25 and saved['search_range']==25 and not saved['automatic_search'] and saved['hotkeys']['settings']=='Ctrl + G'
    assert saved['filters']['stats'][0]['id']==original['filters']['stats'][0]['id'] and saved['filters']['stats'][0]['min']==12
    assert saved['map_check']['selectedStats']==maps['selectedStats'] and saved['map_check']['profile']==2 and saved['map_check']['showNewStats']
    assert not saved['filters']['online'] and saved['filters']['default_currency']=='chaos_divine' and saved['filters']['bulk_currency']=='divine'
    report['online_currencies_map_preferences']=True
    focus_game();subprocess.run(['xdotool','key','--clearmodifiers','ctrl+g'],env=env,check=True)
    wait_for(lambda:state(state='Passive',rendered=True,view='Settings'),'new shortcut opens settings')
    assert status()['game_active'] and status()['keyboard_mode']=='None'
    report['saved_scale_range_and_new_shortcut']=True
    command('hide');focus_game()
    subprocess.run(['wl-copy','--type','text/plain;charset=utf-8'],input=(root/'src/fixtures/scroll-of-wisdom.txt').read_text(),env=env,text=True,check=True,stderr=subprocess.DEVNULL)
    before=(runtime/'overlay.log').read_text().count('trade search generation=');show_price();s=status()
    assert s['app_scale']==1.25 and s['width']>=775 and s['request'] is None and s['economy'] is None and s['minimum_stock']==9,s
    assert (runtime/'overlay.log').read_text().count('trade search generation=')==before
    command('prices');s=wait_for(lambda:status() if status()['request'] else None,'manual search')
    assert s['request']['query']['filters']['misc_filters']['filters']['stack_size']['min']==9 and any(f['id']=='explicit.stat_3299347043' for g in s['request']['query']['stats'] for f in g['filters']),s['request']
    assert s['request']['query']['status']['option']=='any' and s['payment_currency']=='divine'
    report['manual_search_and_copied_stock']=True
    open_settings();click(pref()['reset']);click(pref()['apply']);wait_for(lambda:state(state='Hidden'),'defaults saved')
    saved=json.loads(configpath.read_text());assert saved.get('automatic_search',True) and saved.get('hotkeys') is None and saved['scale']==1
    assert saved['filters']['stats'][0]['min']==12 and saved['map_check']['selectedStats']==maps['selectedStats']
    focus_game();subprocess.run(['xdotool','key','--clearmodifiers','ctrl+g'],env=env,check=True);time.sleep(.15);assert status()['state']=='Hidden','removed shortcut still acts'
    # Saving cannot reactivate the overlay after a deliberate application switch.
    open_settings();click(pref()['apply']);subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True)
    wait_for(lambda:state(state='Hidden',game_active=False),'save while switching applications')
    time.sleep(.1);assert status()['keyboard_mode']=='None'
    report.update(draft_cancel_and_defaults=True,validation_errors_keep_file=True,advanced_rules_preserved=True,default_bindings_restored=True,save_does_not_reopen_after_switch=True,scope='Private River/XWayland; virtual keyboard/pointer and fake game only; all external services blocked',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-settings.py','src/settings.rs','src/hotkeys.rs','src/desktop/preferences.rs','src/desktop/xwayland_shortcuts.rs','src/ui.rs','src/ui/settings_panel.rs','src/filters.rs']}
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
