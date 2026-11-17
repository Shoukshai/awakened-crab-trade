#!/usr/bin/env python3
"""Native interleaved widget stacking on a private River/XWayland desktop. Never touches the real game."""
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
env=os.environ.copy();env.update(XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(runtime/'config'),XDG_CACHE_HOME=str(runtime/'cache'),XDG_STATE_HOME=str(runtime/'state'),XDG_DATA_HOME=str(runtime/'data'),GSETTINGS_BACKEND='memory',WLR_BACKENDS='headless',WLR_HEADLESS_OUTPUTS='1',WLR_RENDERER='pixman',GDK_BACKEND='wayland',HTTPS_PROXY='http://127.0.0.1:9',HTTP_PROXY='http://127.0.0.1:9',ALL_PROXY='http://127.0.0.1:9')
logs=[];processes=[];success=False;report={}
def start(args,name,**kwargs):
    if name=='overlay' and os.environ.get('APT_IMAGE_GDB')=='1':
        args=['gdb','--batch','-ex','set print thread-events off','-ex','set environment G_DEBUG fatal-criticals','-ex','run','-ex','bt 40','--args']+args
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
    source=runtime/'stack.png';Image.new('RGB',(64,48),(240,60,80)).save(source)
    uploads=pathlib.Path(env['XDG_DATA_HOME'])/'awakened-poe-trade-rs/images';uploads.mkdir(parents=True)
    url=hashlib.sha256(source.read_bytes()).hexdigest()+'.png';(uploads/url).write_bytes(source.read_bytes())
    original={'league':'Standard','automatic_search':False,'stopwatches':[{'id':1,'title':'Premier','x':50,'y':50,'wanted':True}],'image_strips':[{'id':1,'title':'Image','x':46,'y':47,'anchor':'tl','wanted':True,'images':[{'id':1,'url':url}]}]}
    configpath.write_text(json.dumps(original));original_bytes=configpath.read_bytes()
    if os.environ.get('APT_STACK_WAYLAND_DEBUG')=='1':env['WAYLAND_DEBUG']='1'
    daemon=start([str(app),'daemon'],'overlay')
    env.pop('WAYLAND_DEBUG',None)
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'shortcuts')
    focus_game()
    timer={'type':'stopwatch','id':1};image={'type':'image','id':1}
    def clock():return status()['stopwatches']['rows'][0]
    def band():return status()['images']['rows'][0]
    def canvas():return status()['widget_canvas']
    def interact():
        command('stopwatches');wait_for(lambda:state(view='Stopwatches',rendered=True,state='Passive'),'manager passive')
        assert status()['game_active'] and not status()['images']['visible']
        command('interact');wait_for(lambda:state(state='Interactive') and status()['images']['rendered'] and status()['stopwatches']['rendered'] and band()['cards'][0]['loaded'],'shared widgets painted')
    def click(r,owner=False):
        assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        o={'x':0,'y':0} if owner else status()['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def pixel(x,y):
        path=runtime/'pixels.png';subprocess.run(['grim',str(path)],env=env,check=True);return Image.open(path).convert('RGB').getpixel((int(x),int(y)))
    def settled():wait_for(lambda:not status()['preferences_pending'] and not canvas()['dirty'],'order persisted')
    interact();time.sleep(.12)
    assert canvas()['order']==[timer,image]
    a=clock()['rect'];b=band()['cards'][0]['rect']
    # Bottom-left of the face lies below the toolbar, intersects the image card,
    # and avoids timer text/buttons before hovering.
    px=max(a['x']+8,b['x']+8);py=min(a['y']+a['height']-8,b['y']+b['height']-8)
    assert b['x']<px<b['x']+b['width'] and b['y']<py<b['y']+b['height']
    assert pixel(px,py)==(240,60,80),(a,b,px,py,pixel(px,py))
    tx=a['x']+12;ty=a['y']+a['height']-12
    assert tx<band()['rect']['x'],'timer exposed edge required'
    pointer(tx,ty,True);settled();wait_for(lambda:canvas()['order']==[image,timer],'cross-type timer raised')
    assert pixel(px,py)!=(240,60,80),'native drawing order did not change'
    assert json.loads(configpath.read_text())['widget_order']==[image,timer]
    report['cross_type_native_pixels_and_persisted_raise']=True
    # Click an overlapping timer button. The image below must receive no click.
    control=clock()['controls']['toggle'];assert control and control['mapped']
    cx=control['x']+control['width']/2;cy=min(control['y']+control['height']-4,b['y']+8)
    assert b['x']<cx<b['x']+b['width'] and b['y']<cy<b['y']+b['height'] and control['y']<cy<control['y']+control['height'],(control,b,cx,cy)
    pointer(cx,cy,True);wait_for(lambda:clock()['running'],'top timer gets overlap click')
    assert status()['images']['preview'] is None
    click(clock()['controls']['toggle'],True);wait_for(lambda:not clock()['running'],'pause')
    report['overlap_click_routed_to_top_widget_only']=True
    # Return the image to the top using its exposed lower card, then leave preview.
    pointer(b['x']+b['width']/2,b['y']+b['height']-8,True)
    wait_for(lambda:canvas()['order']==[timer,image],'image raised');settled()
    pointer(tx,ty);wait_for(lambda:status()['images']['preview'] is None,'preview leave')
    assert pixel(px,py)==(240,60,80)
    assert 'widget_order' not in json.loads(configpath.read_text())
    pointer(tx,ty,True);settled();assert canvas()['order']==[image,timer]
    saved_bytes=configpath.read_bytes()
    # Failed IO must restore both persistence and visual stacking without retrying.
    configdir.chmod(0o500)
    try:
        pointer(b['x']+b['width']/2,b['y']+b['height']-8,True)
        wait_for(lambda:'Modifications non enregistrées' in status()['stopwatch_manager']['message'] and not status()['preferences_pending'],'order IO failure')
        pointer(tx,ty);wait_for(lambda:status()['images']['preview'] is None,'failed order preview left')
        assert canvas()['order']==[image,timer] and not canvas()['dirty'] and configpath.read_bytes()==saved_bytes
        assert pixel(px,py)!=(240,60,80)
    finally:configdir.chmod(0o700)
    report['real_io_failure_restores_native_stack_without_retry']=True
    command('quit');assert daemon.wait(timeout=5)==0
    daemon=start([str(app),'daemon'],'overlay-restart')
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'restart socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'restart shortcuts');focus_game()
    assert status()['state']=='Hidden' and not canvas()['visible']
    interact();time.sleep(.15)
    assert canvas()['order']==[image,timer] and pixel(px,py)!=(240,60,80)
    report['restart_restores_cross_type_pixels_and_starts_hidden']=True
    # A detached editor must prune stacking references before validating deletion.
    command('hide');focus_game();command('settings');wait_for(lambda:state(state='Passive',view='Settings',rendered=True),'delete settings')
    command('interact');wait_for(lambda:state(state='Interactive'),'delete interaction')
    click(status()['preferences']['tabs'][3]);wait_for(lambda:status()['preferences']['page']==3,'delete timer tab')
    for _ in range(20):
        p=status()['preferences']['stopwatches'];r=p['rows'][0]['remove'];v=p['scroll'];y=r['y']+r['height']/2
        if v['y']+8<y<v['y']+v['height']-8:break
        o=status()['origin'];subprocess.run([str(target/'debug/examples/pointer'),str(o['x']+v['x']+8),str(int(o['y']+v['y']+v['height']/2)),'2560','1600','--scroll','70'],env=env,check=True);time.sleep(.06)
    else:raise RuntimeError('draft remove clipped')
    click(r);wait_for(lambda:not status()['preferences']['stopwatches']['rows'],'removed draft')
    time.sleep(.1)
    click(status()['preferences']['apply']);wait_for(lambda:state(state='Hidden') and not status()['preferences_pending'],'deleted draft saved')
    assert not json.loads(configpath.read_text()).get('stopwatches') and 'widget_order' not in json.loads(configpath.read_text())
    report['native_draft_deletion_prunes_saved_order']=True
    # Recreate a timer through the native manager to test independent input owners.
    focus_game();command('stopwatches');wait_for(lambda:state(state='Passive',view='Stopwatches',rendered=True),'new manager')
    command('interact');wait_for(lambda:state(state='Interactive'),'new interaction')
    click(status()['stopwatch_manager']['add']);wait_for(lambda:len(status()['stopwatches']['rows'])==1 and not status()['preferences_pending'],'new timer')
    a=clock()['rect'];tx=a['x']+12;ty=a['y']+a['height']-12
    # With one domain hidden, the other retains its input union on the same surface.
    command('images');wait_for(lambda:state(view='Images'),'images manager')
    click(status()['image_manager']['rows'][0]['wanted']);settled()
    assert not status()['images']['visible'] and status()['images']['keyboard_mode']=='None' and canvas()['visible'] and canvas()['keyboard_mode']=='OnDemand'
    pointer(tx,ty);wait_for(lambda:clock()['controls']['toggle'] and clock()['controls']['toggle']['mapped'],'remaining clock controls')
    click(clock()['controls']['toggle'],True);wait_for(lambda:clock()['running'],'remaining domain click')
    report['hiding_images_preserves_clock_input_on_one_surface']=True
    command('hide');focus_game();command('demo');wait_for(lambda:state(state='Passive',view='Price',rendered=True),'exclusive price')
    assert not canvas()['visible'] and canvas()['keyboard_mode']=='None'
    command('hide');focus_game();time.sleep(.15);frames=canvas()['frames'];time.sleep(1.2)
    assert canvas()['frames']==frames and not canvas()['visible'] and canvas()['keyboard_mode']=='None' and not status()['stopwatches']['scheduled_tick'] and not status()['images']['scheduled_animation']
    report['exclusive_and_hidden_revoke_union_without_frames']=True
    assert 'trade search generation=' not in (runtime/'overlay.log').read_text()
    for name in ['overlay','overlay-restart']:
        assert '-CRITICAL' not in (runtime/(name+'.log')).read_text(),(runtime/(name+'.log')).read_text()[-3000:]
    report.update(no_network=True,scope='Private River/XWayland only; native overlapping GTK image/timer pixels, mouse routing, profile IO failure and restart',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-widget-order.py','src/widget_order.rs','src/ui/widget_canvas.rs','src/ui/stopwatch_panel.rs','src/ui/image_panel.rs','src/settings.rs','src/ui.rs','src/ui/settings_panel.rs']}
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
