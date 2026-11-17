#!/usr/bin/env python3
"""Native floating widget menu on a private River/XWayland desktop. Never touches the real game."""
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
    original={'league':'Standard','automatic_search':False,'stash_searches':[],'stopwatches':[{'id':1,'title':'Premier','x':50,'y':50,'wanted':True}]}
    configpath.write_text(json.dumps(original));original_bytes=configpath.read_bytes()
    daemon=start([str(app),'daemon'],'overlay')
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'shortcuts');focus_game()
    def menu():return status()['widget_menu']
    def clock(id=1):return next(r for r in status()['stopwatches']['rows'] if r['id']==id)
    def row(target):return next(r for r in menu()['rows'] if r['target']==target)['rect']
    def click(r,canvas=True):
        assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        s=status();o={'x':0,'y':0} if canvas else s['origin']
        if canvas:assert s['widget_canvas']['keyboard_mode']=='OnDemand' and menu()['rendered']
        pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def settled():wait_for(lambda:not status()['preferences_pending'] and not status()['widget_canvas']['dirty'],'preferences settled')
    def opened():return state(state='Interactive',view='Menu',visible=False) and menu()['rendered'] and menu()['interactive']
    def open_menu():
        command('hide');focus_game();command('interact');wait_for(opened,'menu after first paint')
        wait_for(lambda:all(r['visible']==r['wanted'] for r in status()['stopwatches']['rows']+status()['images']['rows']),'wanted widgets restored with menu')
    command('menu');wait_for(lambda:state(state='Passive',view='Menu',visible=False) and menu()['rendered'],'passive native menu')
    assert status()['game_active'] and status()['widget_canvas']['keyboard_mode']=='None'
    before=fixture_command('input')['clicks'];r=menu()['body'];pointer(r['x']+r['width']/2,r['y']+r['height']/2,True)
    wait_for(lambda:fixture_command('input')['clicks']==before+1,'passive menu passes through to fake game')
    command('interact');wait_for(opened,'explicit native menu interaction')
    click(row({'type':'settings'}));wait_for(lambda:state(view='Settings',state='Interactive',rendered=True),'first menu click opens settings')
    click(status()['preferences']['cancel'],False);wait_for(opened,'first settings cancel returns to menu')
    report['fresh_menu_settings_handoff_no_lost_click']=True
    assert [r['target'] for r in menu()['rows']]==[{'type':'settings'},{'type':'catalogue'},{'type':'image','id':1}]
    r=menu()['body'];pointer(r['x']+10,r['y']+10);time.sleep(.08)
    click(menu()['edit']);wait_for(lambda:menu()['always']['mapped'] and not menu()['paste']['mapped'],'inline menu settings')
    click(menu()['always']);settled();assert json.loads(configpath.read_text())['widget_menu']['always_show']
    assert any(r['target']=={'type':'stopwatch','id':1} for r in menu()['rows'])
    click(row({'type':'stopwatch','id':1}));settled();assert not clock()['wanted'] and not clock()['visible']
    click(row({'type':'stopwatch','id':1}));settled();assert clock()['wanted'] and status()['widget_canvas']['order'][-1]=={'type':'stopwatch','id':1}
    report['native_filter_always_show_and_toggle_show_raises']=True
    click(menu()['plus']);wait_for(lambda:menu()['add_timer']['mapped'],'native add choices');time.sleep(.08)
    click(menu()['add_timer']);wait_for(lambda:len(status()['stopwatches']['rows'])==2 and not status()['preferences_pending'],'timer created from menu')
    assert 40<=clock(2)['x']<60 and 40<=clock(2)['y']<60
    click(menu()['plus']);wait_for(lambda:menu()['add_images']['mapped'],'native image choice');time.sleep(.08)
    click(menu()['add_images']);wait_for(lambda:len(status()['images']['rows'])==2 and not status()['preferences_pending'],'image created from menu')
    report['native_menu_add_timer_and_image']=True
    # Four-corner movement uses the visible body and survives profile IO.
    click(menu()['move']);wait_for(lambda:menu()['moving'],'menu move mode')
    b=menu()['body'];subprocess.run([str(target/'debug/examples/pointer'),str(b['x']+b['width']-4),str(b['y']+b['height']-4),'2560','1600','--drag-to','1900','1300'],env=env,check=True)
    wait_for(lambda:not status()['preferences_pending'] and menu()['config']['anchor']=='br' and abs(menu()['body']['x']+menu()['body']['width']-1900)<2 and abs(menu()['body']['y']+menu()['body']['height']-1300)<2,'menu corner persisted and drawn')
    assert json.loads(configpath.read_text())['widget_menu']['anchor']=='br'
    click(menu()['move']);wait_for(lambda:not menu()['moving'],'move mode left')
    click(menu()['edit']);wait_for(lambda:menu()['always']['mapped'],'menu editor for IO failure')
    saved=configpath.read_bytes();configdir.chmod(0o500)
    try:
        click(menu()['always']);wait_for(lambda:not status()['preferences_pending'] and 'non enregistrées' in menu()['message'],'real menu IO failure')
        assert configpath.read_bytes()==saved and menu()['always_value'] and menu()['config']['always_show']
    finally:configdir.chmod(0o700)
    report['native_corner_drag_and_config_io_rollback']=True
    # Actual native clipboard paste: the menu clears its field and opens prices
    # only after the new panel has painted. All external services are blocked.
    click(menu()['edit']);wait_for(lambda:menu()['paste']['mapped'],'paste field shown');time.sleep(.08)
    text=(root/'src/fixtures/driftwood-wand.txt').read_text()
    owner=start(['wl-copy','--foreground','--type','text/plain;charset=utf-8'],'clipboard',stdin=subprocess.PIPE,text=True);owner.stdin.write(text);owner.stdin.close();time.sleep(.05)
    click(menu()['paste']);wait_for(lambda:menu()['paste_focused'],'native paste has focus')
    subprocess.run([str(target/'debug/examples/native-key'),'ctrl-v'],env=env,check=True)
    wait_for(lambda:state(view='Price',state='Interactive',rendered=True) and status()['item'] and status()['item']['base']=='Driftwood Wand','native paste to rendered price panel')
    assert status()['query'] is None and status()['request'] is None and not menu()['visible']
    assert subprocess.check_output(['wl-paste','--no-newline'],env=env,text=True)==text
    report['native_paste_parses_without_game_copy_or_automatic_http']=True
    # Settings handoff releases the shared surface; Cancel returns to the menu.
    for _ in range(3):
        open_menu();click(row({'type':'settings'}));wait_for(lambda:state(view='Settings',state='Interactive',rendered=True),'menu settings handoff')
        click(status()['preferences']['cancel'],False);wait_for(opened,'cancel returns to floating menu')
        click(menu()['paste']);time.sleep(.1);assert opened() and status()['widget_canvas']['focused']
    report['settings_cancel_and_owned_focus_handoffs']=3
    open_menu();click(menu()['edit']);wait_for(lambda:menu()['always']['mapped'],'size stability editor');time.sleep(.06)
    width=menu()['body']['width'];click(menu()['always']);settled();time.sleep(.12);assert not menu()['always_value'] and menu()['body']['width']>=width and opened() and status()['widget_canvas']['focused']
    click(menu()['always']);settled();click(menu()['edit']);wait_for(lambda:menu()['paste']['mapped'],'size stability restored')
    report['removing_buttons_keeps_painted_menu_under_pointer']=True
    open_menu();click(row({'type':'catalogue'}));wait_for(lambda:state(view='Catalogue',state='Interactive',rendered=True) and menu()['visible'],'catalogue from menu')
    click(row({'type':'catalogue'}));wait_for(opened,'catalogue toggle returns to menu')
    report['native_catalogue_toggle_preserves_menu']=True
    settled();saved=configpath.read_bytes();command('quit');assert daemon.wait(timeout=5)==0
    daemon=start([str(app),'daemon'],'overlay-restart');wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'restart socket');wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'restart shortcuts');focus_game()
    assert status()['state']=='Hidden' and not menu()['visible'] and configpath.read_bytes()==saved
    open_menu();assert menu()['config']['anchor']=='br' and menu()['config']['always_show'] and len(status()['stopwatches']['rows'])==2
    report['restart_retains_menu_config_and_starts_hidden']=True
    dimensions=[]
    for w,h,scale,lw,lh in [(1920,1080,1,1920,1080),(2560,1600,2,1280,800),(2560,1600,1,2560,1600)]:
        command('hide');focus_game();subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode',f'{w}x{h}@60Hz','--scale',str(scale)],env=env,check=True)
        open_menu();wait_for(lambda:menu()['viewport']==[lw,lh] and menu()['buffer_scale']==scale and abs(menu()['body']['x']+menu()['body']['width']-menu()['config']['x']/100*lw)<2 and abs(menu()['body']['y']+menu()['body']['height']-menu()['config']['y']/100*lh)<2,'menu output geometry')
        dimensions.append({'physical':[w,h],'scale':scale,'logical':[lw,lh]})
    report['private_menu_output_reconfiguration']=dimensions
    subprocess.run(['grim','/tmp/apt-v20-private-menu.png'],env=env,check=True)
    # Respect a voluntary switch and never reopen on return.
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True)
    other=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',str(first['other'])],env=env,text=True).splitlines())
    pointer(int(other['X'])+30,int(other['Y'])+30)
    wait_for(lambda:state(state='Hidden') and not menu()['visible'] and status()['widget_canvas']['keyboard_mode']=='None','other app hides menu')
    focus_game();time.sleep(.15);assert not menu()['visible']
    report['foreign_app_hides_menu_without_return_activation']=True
    command('hide');time.sleep(.15);frames=status()['widget_canvas']['frames'];time.sleep(1.2)
    assert status()['widget_canvas']['frames']==frames and not status()['widget_canvas']['visible']
    # A late GTK clipboard response belongs to the old buffer/session.
    late=start([str(target/'debug/examples/slow-clipboard'),str(root/'src/fixtures/driftwood-wand.txt')],'late-gtk-clipboard',stdout=subprocess.PIPE,bufsize=0)
    assert select.select([late.stdout],[],[],3)[0] and late.stdout.readline()==b'READY\n'
    time.sleep(.05);open_menu();click(menu()['paste']);wait_for(lambda:menu()['paste_focused'],'late paste focus')
    subprocess.run([str(target/'debug/examples/native-key'),'ctrl-v'],env=env,check=True)
    assert select.select([late.stdout],[],[],3)[0] and late.stdout.readline()==b'REQUEST\n'
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True);pointer(int(other['X'])+30,int(other['Y'])+30)
    wait_for(lambda:not status()['game_active'] and not menu()['visible'],'GTK paste loses menu context');focus_game();open_menu()
    time.sleep(1.4);assert opened() and menu()['paste_length']==0
    report['late_gtk_paste_dropped_after_menu_close_and_reopen']=True
    command('hide');focus_game()
    # Confirm a real, delayed clipboard read is cancelled even if the game
    # returns before the data arrives. The provider and both windows are private.
    provider=start([str(target/'debug/examples/slow-clipboard'),str(root/'src/fixtures/driftwood-wand.txt')],'delayed-clipboard',stdout=subprocess.PIPE,bufsize=0)
    assert select.select([provider.stdout],[],[],3)[0] and provider.stdout.readline()==b'READY\n'
    time.sleep(.05);command('check');wait_for(lambda:status()['capture_pending'] is not None,'delayed capture pending')
    observed=status()['generation'];assert select.select([provider.stdout],[],[],3)[0] and provider.stdout.readline()==b'REQUEST\n'
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True);pointer(int(other['X'])+30,int(other['Y'])+30)
    wait_for(lambda:not status()['game_active'],'capture loses game context');focus_game()
    wait_for(lambda:f'discarded capture generation={observed}' in (runtime/'overlay-restart.log').read_text(),'late capture discarded after return',4)
    assert state(state='Hidden',visible=False,game_active=True) and not menu()['visible'] and status()['generation']>observed
    report['delayed_capture_does_not_reopen_after_foreign_app_and_return']=True
    # Exercise all configured instances at the largest UI scale and factor 2.
    command('quit');assert daemon.wait(timeout=5)==0
    stress=json.loads(configpath.read_text());stress.pop('widget_order',None);stress['scale']=2.5;stress.pop('widget_menu',None)
    stress['stopwatches']=[{'id':i,'title':'Long timer title '+str(i)+' '+('x'*60),'x':50,'y':50,'wanted':False} for i in range(1,33)]
    stress['image_strips']=[{'id':i,'title':'Long image title '+str(i)+' '+('x'*60),'x':50,'y':10,'anchor':'tc','wanted':False,'images':[]} for i in range(1,17)]
    configpath.write_text(json.dumps(stress));subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode','2560x1600@60Hz','--scale','2'],env=env,check=True)
    daemon=start([str(app),'daemon'],'overlay-stress');wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'stress socket');wait_for(lambda:state(game_active=True) and 'Ok' in status()['shortcut_backend']['status'],'stress ready')
    open_menu();wait_for(lambda:menu()['viewport']==[1280,800] and len(menu()['rows'])==50,'maximum menu')
    rect=menu()['rect'];assert rect['x']>=0 and rect['y']>=0 and rect['x']+rect['width']<=1280 and rect['y']+rect['height']<=800,rect
    for _ in range(64):
        r=row({'type':'image','id':16});v=menu()['scroll'];y=r['y']+r['height']/2
        if v['y']+8<y<v['y']+v['height']-8:break
        subprocess.run([str(target/'debug/examples/pointer'),str(v['x']+12),str(int(v['y']+v['height']/2)),'1280','800','--scroll','200'],env=env,check=True);time.sleep(.06)
    else:raise RuntimeError('last maximum-profile widget unreachable')
    assert v['x']<r['x']+r['width']/2<v['x']+v['width']
    subprocess.run([str(target/'debug/examples/pointer'),str(int(r['x']+r['width']/2)),str(int(r['y']+r['height']/2)),'1280','800','--click'],env=env,check=True)
    wait_for(lambda:not status()['preferences_pending'] and status()['images']['rows'][-1]['wanted'],'last scrolled menu button acts')
    report['maximum_profile_50_buttons_scale_2_5_on_factor_2_output_reachable']=True
    command('hide');focus_game()
    assert 'trade search generation=' not in (runtime/'overlay.log').read_text()
    for name in ['overlay','overlay-restart','overlay-stress']:
        assert '-CRITICAL' not in (runtime/(name+'.log')).read_text(),(runtime/(name+'.log')).read_text()[-4000:]
    report.update(no_network=True,scope='Private River/XWayland only; native menu controls, clipboard paste and fake game; no real game inputs',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-widget-menu.py','src/widget_menu.rs','src/ui/widget_menu.rs','src/ui/widget_canvas.rs','src/widget_order.rs','src/ui.rs','examples/native-key.rs','examples/slow-clipboard.rs','examples/protocols/virtual-keyboard-unstable-v1.xml']}
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
