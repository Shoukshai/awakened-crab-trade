#!/usr/bin/env python3
"""Native stash-search widgets on a private River/XWayland desktop. Never touches the real game."""
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
    assert display!=os.environ.get('DISPLAY') and (str(runtime),wayland)!=(os.environ.get('XDG_RUNTIME_DIR'),os.environ.get('WAYLAND_DISPLAY')),'refusing host desktop'
    env.update(DISPLAY=display,WAYLAND_DISPLAY=wayland)
    subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode','2560x1600@60Hz'],env=env,check=True)
    foreign=start([str(target/'debug/examples/overlay-fixture'),'--isolated'],'foreign',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    assert select.select([foreign.stdout],[],[],3)[0];json.loads(foreign.stdout.readline());foreign.stdin.write('unmap\n');foreign.stdin.flush();assert select.select([foreign.stdout],[],[],3)[0];json.loads(foreign.stdout.readline())
    keyboard=start(['wtype','-s','60000','-k','Escape'],'keyboard')
    fixture=start([str(target/'debug/examples/stash-fixture')],'fixture',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    assert select.select([fixture.stdout],[],[],5)[0],'GTK fake game not ready';assert json.loads(fixture.stdout.readline())['ready']
    configdir=pathlib.Path(env['XDG_CONFIG_HOME'])/'awakened-poe-trade-rs';configdir.mkdir(parents=True);configpath=configdir/'config.json'
    original={'league':'Standard','automatic_search':False,'restore_clipboard':True,'stash_searches':[{'id':1,'title':'Premier','x':35,'y':46,'wanted':False,'entries':[{'id':1,'name':'Devise','text':'Currency','hotkey':'Ctrl + J'},{'id':2,'name':'','text':'Fossil','hotkey':None}]},{'id':2,'title':'Second','x':65,'y':55,'wanted':False,'entries':[{'id':1,'name':'','text':'Reflect','hotkey':None}]}]}
    configpath.write_text(json.dumps(original))
    daemon=start([str(app),'daemon'],'overlay');wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket');wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'shortcuts');focus_game()
    def menu():return status()['widget_menu']
    def row(id=1):return next(r for r in status()['stash']['rows'] if r['config']['id']==id)
    def settled():wait_for(lambda:not status()['preferences_pending'] and not status()['widget_canvas']['dirty'],'preferences settled');time.sleep(.08)
    def click(r,main=False):
        assert r and r['mapped'] and r['width']>4 and r['height']>4,r
        s=status();assert s['state']=='Interactive';o=s['origin'] if main else {'x':0,'y':0}
        assert (s['keyboard_mode'] if main else s['widget_canvas']['keyboard_mode'])=='OnDemand'
        pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def open_menu():
        command('hide');focus_game();command('interact');wait_for(lambda:state(state='Interactive',view='Menu') and menu()['rendered'],'menu ready');time.sleep(.08)
    target_path=target
    def mrow(target):
        for _ in range(32):
            r=next(r['rect'] for r in menu()['rows'] if r['target']==target);v=menu()['scroll'];y=r['y']+r['height']/2
            if v['y']+4<y<v['y']+v['height']-4:return r
            subprocess.run([str(target_path/'debug/examples/pointer'),str(v['x']+10),str(int(v['y']+v['height']/2)),'2560','1600','--scroll','70' if y>v['y'] else '-70'],env=env,check=True);time.sleep(.08)
        raise RuntimeError('menu target clipped')
    def shown(id=1):return row(id)['visible'] and row(id)['painted'] and status()['stash']['keyboard_mode']=='OnDemand'
    open_menu();click(mrow({'type':'stash','id':1}));settled();wait_for(shown,'stash painted')
    assert row()['config']['wanted'] and not row(2)['visible']
    subprocess.run(['grim','/tmp/apt-v21-private-stash.png'],env=env,check=True)
    b=row()['body'];pointer(b['x']+15,b['y']+12);time.sleep(.35);frames=status()['widget_canvas']['frames'];time.sleep(1.2);assert shown() and status()['widget_canvas']['frames']==frames
    report['static_visible_stash_has_no_periodic_frames']=True
    # Real GTK clipboard paste in a fake game field, followed by Enter, proves delivery.
    keeper=start(['wl-copy','--foreground','--type','text/plain;charset=utf-8','clipboard before search'],'clipboard')
    time.sleep(.08);before=fixture_command('input')['submitted'];click(row()['buttons'][0]['rect'])
    wait_for(lambda:status()['stash_pending'] is None,'stash job done')
    actual=wait_for(lambda:fixture_command('input')['submitted'],'native fake game received filter')
    assert actual==before+['Currency'],(actual,status())
    assert status()['state']=='Hidden' and not status()['widget_canvas']['visible'] and status()['stash_result']=={'Ok':None}
    assert subprocess.check_output(['wl-paste','--no-newline'],env=env,text=True)=='clipboard before search'
    report['native_search_hides_overlay_pastes_and_submits_only_in_private_game']=True
    report['text_clipboard_restored_after_search']=True
    # The per-entry hotkey follows the same guarded path while the overlay is hidden.
    focus_game();subprocess.run(['xdotool','key','--clearmodifiers','ctrl+j'],env=env,check=True)
    wait_for(lambda:len(fixture_command('input')['submitted'])==2,'entry shortcut submits');assert fixture_command('input')['submitted']==['Currency','Currency']
    report['hidden_per_entry_hotkey_works']=True
    open_menu();wait_for(shown,'persistent stash reappears only explicitly')
    # Disable all entry keys in this widget through its native header checkbox.
    click(row()['enable']);settled();assert not row()['config']['enable_hotkeys']
    command('hide');focus_game();subprocess.run(['xdotool','key','--clearmodifiers','ctrl+j'],env=env,check=True);time.sleep(.25)
    assert fixture_command('input')['submitted']==['Currency','Currency']
    report['native_enable_toggle_releases_entry_keys']=True
    open_menu();wait_for(shown,'reopen stash');b=row()['body'];pointer(b['x']+10,b['y']+12);time.sleep(.08)
    click(row()['edit']);wait_for(lambda:state(view='Settings',state='Interactive',rendered=True) and status()['preferences']['page']==5,'native stash editor')
    assert status()['preferences']['stash']['count']==2
    click(status()['preferences']['cancel'],True);wait_for(lambda:state(view='Menu',state='Interactive'),'cancel returns to menu')
    # Add a complete new instance through the menu.
    click(menu()['plus']);wait_for(lambda:menu()['add_stash']['mapped'],'stash add choice');time.sleep(.08);click(menu()['add_stash']);settled();wait_for(lambda:shown(3),'new stash painted')
    c=row(3)['config'];assert 20<=c['x']<40 and 20<=c['y']<40 and c['entries']==[{'id':1,'name':'','text':'Currency','hotkey':None}]
    report['native_menu_add_and_settings_cancel']=True
    # Drag the new widget using a corner, then exercise write rollback on that preview.
    b=row(3)['body'];pointer(b['x']+10,b['y']+10);time.sleep(.08);click(row(3)['move']);wait_for(lambda:row(3)['moving'],'stash move mode')
    b=row(3)['body'];subprocess.run([str(target/'debug/examples/pointer'),str(b['x']+b['width']-4),str(b['y']+b['height']-4),'2560','1600','--drag-to','1900','1300'],env=env,check=True);settled()
    assert row(3)['config']['anchor']=='br';b=row(3)['body'];assert abs(b['x']+b['width']-1900)<4 and abs(b['y']+b['height']-1300)<4
    saved=configpath.read_bytes();old=row(3)['config'].copy();configdir.chmod(0o500)
    try:
        b=row(3)['body'];subprocess.run([str(target/'debug/examples/pointer'),str(b['x']+b['width']-4),str(b['y']+b['height']-4),'2560','1600','--drag-to','1700','1100'],env=env,check=True)
        wait_for(lambda:not status()['preferences_pending'] and 'non enregistrées' in menu()['message'],'stash drag IO rollback')
        assert configpath.read_bytes()==saved and row(3)['config']==old
    finally:configdir.chmod(0o700)
    report['native_corner_drag_and_preview_io_rollback']=True
    # The editor keeps changes in its draft, rejects duplicate keys, then rebinds on Apply.
    open_menu();b=row()['body'];pointer(b['x']+10,b['y']+10);time.sleep(.08);click(row()['edit']);wait_for(lambda:state(view='Settings',state='Interactive') and status()['preferences']['page']==5,'stash editor again')
    def draft():return status()['preferences']['stash']
    def drow():return next(r for r in draft()['rows'] if r['id']==1)
    def reveal(fn):
        for _ in range(32):
            r=fn();v=draft()['scroll'];y=r['y']+r['height']/2
            if v['y']+5<y<v['y']+v['height']-5:return r
            o=status()['origin'];subprocess.run([str(target/'debug/examples/pointer'),str(o['x']+v['x']+8),str(int(o['y']+v['y']+v['height']/2)),'2560','1600','--scroll','70' if y>v['y'] else '-70'],env=env,check=True);time.sleep(.06)
        raise RuntimeError('stash draft control clipped')
    def write(fn,value):
        click(reveal(fn),True);subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--',value],env=env,check=True)
    write(lambda:drow()['entries'][0]['hotkey']['rect'],'Ctrl + D');click(reveal(lambda:drow()['enable']),True)
    saved=configpath.read_bytes();click(status()['preferences']['apply'],True);wait_for(lambda:'plusieurs fois' in status()['preferences']['message'],'duplicate stash key rejected');assert configpath.read_bytes()==saved
    click(reveal(lambda:drow()['entries'][0]['record']),True);wait_for(lambda:draft()['capture']==[1,1],'stash capture armed');subprocess.run(['wtype','-M','ctrl','-k','m','-m','ctrl'],env=env,check=True)
    wait_for(lambda:draft()['capture'] is None and drow()['entries'][0]['hotkey']['value']=='Ctrl + M','stash key captured')
    write(lambda:drow()['entries'][0]['text']['rect'],'\"Map Tier\"');write(lambda:drow()['entries'][0]['name']['rect'],'Cartes')
    click(reveal(lambda:drow()['entries'][0]['down']),True);wait_for(lambda:drow()['entries'][1]['id']==1,'filter reordered')
    configdir.chmod(0o500)
    try:
        click(status()['preferences']['apply'],True);wait_for(lambda:'non enregistrées' in status()['preferences']['message'] and not status()['preferences_pending'],'editor real IO rollback');assert configpath.read_bytes()==saved
    finally:configdir.chmod(0o700)
    click(status()['preferences']['apply'],True);wait_for(lambda:state(state='Hidden') and not status()['preferences_pending'],'stash editor saved')
    saved=json.loads(configpath.read_text());c=saved['stash_searches'][0];assert [e['id'] for e in c['entries']]==[2,1] and c['entries'][1]['text']=='\"Map Tier\"' and c['entries'][1]['name']=='Cartes' and c['entries'][1]['hotkey']=='Ctrl + M' and c['enable_hotkeys']
    focus_game();subprocess.run(['xdotool','key','--clearmodifiers','ctrl+m'],env=env,check=True);wait_for(lambda:len(fixture_command('input')['submitted'])==3,'new key performs edited search');assert fixture_command('input')['submitted'][-1]=='\"Map Tier\"'
    report['native_editor_key_capture_reorder_duplicate_refusal_io_rollback_and_rebind']=True
    # The local item inspector uses the catalog name, quoted exactly like JSON.stringify.
    command('hide');focus_game();text=(root/'src/fixtures/driftwood-wand.txt').read_text()
    owner=start(['wl-copy','--foreground','--type','text/plain;charset=utf-8'],'item-clipboard',stdin=subprocess.PIPE,text=True);owner.stdin.write(text);owner.stdin.close();time.sleep(.06)
    command('inspect-check');wait_for(lambda:state(view='Inspection',state='Passive',rendered=True),'local inspector')
    command('interact');wait_for(lambda:state(state='Interactive') and status()['inspection_stash']['mapped'],'inspector button ready');time.sleep(.12)
    click(status()['inspection_stash'],True);wait_for(lambda:len(fixture_command('input')['submitted'])==4,'inspector searches fake stash');assert fixture_command('input')['submitted'][-1]=='"Driftwood Wand"'
    wait_for(lambda:status()['stash_pending'] is None,'inspector search cleanup finished')
    assert subprocess.check_output(['wl-paste','--no-newline'],env=env,text=True)==text
    report['native_inspector_similar_search_quotes_catalog_name']=True
    # Advanced item-copy restoration is completed before the passive overlay appears.
    command('hide');focus_game();keeper=start(['wl-copy','--foreground','--type','text/plain;charset=utf-8','before advanced copy'],'copy-previous');time.sleep(.08)
    command('inspect-copy');wait_for(lambda:state(view='Inspection',state='Passive',rendered=True) and status()['inspection']['dps']['physical']==11,'private advanced copy')
    assert subprocess.check_output(['wl-paste','--no-newline'],env=env,text=True)=='before advanced copy'
    report['advanced_copy_restores_text_before_passive_overlay']=True
    # A real delayed clipboard read must not type after focus is lost and returns.
    open_menu();wait_for(shown,'stash visible before delayed action')
    delayed_text=runtime/'previous.txt';delayed_text.write_text('prior delayed clipboard')
    provider=start([str(target/'debug/examples/slow-clipboard'),str(delayed_text)],'slow-stash',stdout=subprocess.PIPE,bufsize=0)
    assert select.select([provider.stdout],[],[],3)[0] and provider.stdout.readline()==b'READY\n';time.sleep(.05)
    before=fixture_command('input')['submitted'];click(row()['buttons'][0]['rect']);wait_for(lambda:status()['stash_pending'] is not None,'delayed stash pending')
    assert select.select([provider.stdout],[],[],3)[0] and provider.stdout.readline()==b'REQUEST\n'
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True);pointer(30,30);wait_for(lambda:not status()['game_active'],'delayed action loses context');focus_game();time.sleep(1.1)
    assert fixture_command('input')['submitted']==before and status()['state']=='Hidden' and status()['stash_pending'] is None
    report['delayed_clipboard_search_cancelled_after_foreign_app_and_return']=True
    # A short hold is revoked, a full one deletes only the selected instance.
    open_menu();b=row(3)['body'];pointer(b['x']+10,b['y']+10);time.sleep(.08);r=row(3)['remove']
    subprocess.run([str(target/'debug/examples/pointer'),str(int(r['x']+r['width']/2)),str(int(r['y']+r['height']/2)),'2560','1600','--hold','250'],env=env,check=True);time.sleep(1.05)
    assert len(status()['stash']['rows'])==3 and status()['stash']['holds']==0
    r=row(3)['remove'];subprocess.run([str(target/'debug/examples/pointer'),str(int(r['x']+r['width']/2)),str(int(r['y']+r['height']/2)),'2560','1600','--hold','1100'],env=env,check=True)
    wait_for(lambda:len(status()['stash']['rows'])==2 and not status()['preferences_pending'],'full hold deletes only target')
    assert [c['id'] for c in json.loads(configpath.read_text())['stash_searches']]==[1,2]
    report['native_hold_deletion_is_revocable_and_targeted']=True
    # A maximum-length list remains reachable at scale 2.5 on a private factor-2 output.
    command('quit');assert daemon.wait(timeout=5)==0
    stress=json.loads(configpath.read_text());stress['scale']=2.5;stress['stash_searches']=[{'id':i,'title':'Recherche '+str(i),'wanted':False,'x':35,'y':40,'entries':[{'id':j,'name':'Filtre '+str(j),'text':'Currency '+str(j),'hotkey':None} for j in range(1,65 if i==16 else 2)]} for i in range(1,17)]
    stress['stopwatches']=[{'id':i,'wanted':False} for i in range(1,33)];stress['image_strips']=[{'id':i,'wanted':False,'images':[]} for i in range(1,17)];stress.pop('widget_order',None);stress.pop('hotkeys',None);configpath.write_text(json.dumps(stress))
    subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode','2560x1600@60Hz','--scale','2'],env=env,check=True)
    daemon=start([str(app),'daemon'],'overlay-stress');wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'stress socket');wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'stress shortcuts')
    def scaled_pointer(x,y,extra=[]):subprocess.run([str(target/'debug/examples/pointer'),str(int(x)),str(int(y)),'1280','800']+extra,env=env,check=True)
    subprocess.run(['riverctl','set-focused-tags','1'],env=env,check=True);scaled_pointer(1200,700);wait_for(lambda:state(game_active=True),'stress game active');command('interact');wait_for(lambda:state(view='Menu',state='Interactive') and menu()['rendered'],'stress menu')
    assert len(menu()['rows'])==66
    for _ in range(64):
        r=next(r['rect'] for r in menu()['rows'] if r['target']=={'type':'stash','id':16});v=menu()['scroll'];y=r['y']+r['height']/2
        if v['y']+5<y<v['y']+v['height']-5:break
        scaled_pointer(v['x']+10,v['y']+v['height']/2,['--scroll','200']);time.sleep(.06)
    else:raise RuntimeError('last of 66 menu buttons clipped')
    scaled_pointer(r['x']+r['width']/2,r['y']+r['height']/2,['--click']);wait_for(lambda:row(16)['visible'] and row(16)['painted'] and not status()['preferences_pending'],'stress stash visible')
    def within():
        r=row(16)['rect'];return r['x']>=0 and r['y']>=0 and r['x']+r['width']<=1280 and r['y']+r['height']<=800
    time.sleep(.15);wait_for(within,'allocated stash clamped to viewport')
    for _ in range(96):
        r=row(16)['buttons'][-1]['rect'];v=row(16)['scroll'];y=r['y']+r['height']/2
        if v['y']+5<y<v['y']+v['height']-5:break
        scaled_pointer(v['x']+10,v['y']+v['height']/2,['--scroll','200']);time.sleep(.05)
    else:raise RuntimeError('last of 64 filter buttons clipped')
    before=len(fixture_command('input')['submitted']);scaled_pointer(r['x']+r['width']/2,r['y']+r['height']/2,['--click']);wait_for(lambda:len(fixture_command('input')['submitted'])==before+1,'last filter submits')
    assert fixture_command('input')['submitted'][-1]=='Currency 64'
    report['maximum_66_menu_buttons_and_64_filters_ui_scale_2_5_output_factor_2_reachable']=True
    command('hide');before=status()['widget_canvas']['frames'];time.sleep(.5);assert status()['widget_canvas']['frames']==before
    report.update(no_network=True,scope='Private River/XWayland and GTK fake game only; no real game input',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-stash-search.py','examples/stash-fixture.rs','src/stash_search.rs','src/ui/stash_panel.rs','src/ui/stash_settings.rs','src/desktop/game_input.rs','src/desktop/clipboard.rs','src/ui.rs']}
    assert all('-CRITICAL' not in (runtime/(name+'.log')).read_text() and 'trade search generation=' not in (runtime/(name+'.log')).read_text() for name in ['overlay','overlay-stress'])
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
