#!/usr/bin/env python3
"""Native image-strip widgets on a private River/XWayland desktop. Never touches the real game."""
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
    original={'league':'Standard','automatic_search':False,'image_strips':[{'id':1,'title':'Première bande','x':50,'y':10,'anchor':'tc','wanted':True,'images':[{'id':1,'url':'syndicate.jpg'}]},{'id':2,'title':'Seconde bande','x':75,'y':65,'anchor':'tc','wanted':True,'images':[{'id':1,'url':'syndicate.jpg'}]}]}
    gif_path=runtime/'animated.gif'
    Image.new('RGB',(64,48),(240,60,80)).save(gif_path,save_all=True,append_images=[Image.new('RGB',(64,48),(20,210,90))],duration=100,loop=0)
    upload_dir=pathlib.Path(env['XDG_DATA_HOME'])/'awakened-poe-trade-rs/images';upload_dir.mkdir(parents=True)
    gif_url=hashlib.sha256(gif_path.read_bytes()).hexdigest()+'.gif';(upload_dir/gif_url).write_bytes(gif_path.read_bytes())
    original['image_strips'][1]['images'].append({'id':2,'url':gif_url})
    configpath.write_text(json.dumps(original));original_bytes=configpath.read_bytes()
    daemon=start([str(app),'daemon'],'overlay')
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'initial shortcuts')
    focus_game()
    def gallery():return status()['images']
    def row(id):return next(r for r in gallery()['rows'] if r['id']==id)
    def click(r,canvas=False):
        assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        o={'x':0,'y':0} if canvas else status()['origin']
        pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    assert gallery()['available'] and not gallery()['visible']
    command('images');wait_for(lambda:state(state='Passive',rendered=True,view='Images'),'image manager passive')
    assert status()['game_active'] and not gallery()['visible'] and gallery()['keyboard_mode']=='None'
    command('interact');wait_for(lambda:state(state='Interactive') and gallery()['rendered'] and gallery()['keyboard_mode']=='OnDemand','image surface interactive after paint')
    wait_for(lambda:all(row(i)['cards'][0]['loaded'] for i in [1,2]),'built-in thumbnails decoded')
    card=row(1)['cards'][0]['rect'];click(card,True)
    wait_for(lambda:gallery()['preview'] and gallery()['preview']['loaded'] and gallery()['preview']['visible'],'image preview loaded')
    assert status()['state']=='Interactive' and gallery()['focused']
    subprocess.run(['grim','/tmp/apt-v18-private-image-preview.png'],env=env,check=True)
    target_button=status()['image_manager']['add'];origin=status()['origin']
    pointer(origin['x']+target_button['x']+15,origin['y']+target_button['y']+15)
    wait_for(lambda:gallery()['preview'] is None and status()['state']=='Interactive','preview leaves with thumbnail into owned manager')
    report['builtin_multiple_strips_click_preview_and_leave']=True
    # Animated files redraw only when their strip is actually visible.
    wait_for(lambda:row(2)['cards'][1]['loaded'] and row(2)['cards'][1]['animated'] and gallery()['scheduled_animation'],'animated GIF active')
    colors=set()
    for _ in range(6):
        r=row(2)['cards'][1]['rect'];capture=runtime/'animation.png';subprocess.run(['grim',str(capture)],env=env,check=True)
        colors.add(Image.open(capture).getpixel((r['x']+r['width']//2,r['y']+r['height']//2))[:3]);time.sleep(.07)
    assert (240,60,80) in colors and (20,210,90) in colors,colors
    click(status()['image_manager']['rows'][1]['wanted']);wait_for(lambda:not row(2)['visible'] and not status()['preferences_pending'],'animated strip hidden')
    assert not gallery()['scheduled_animation'];time.sleep(.15);before=gallery()['frames'];time.sleep(.35);assert gallery()['frames']==before
    click(status()['image_manager']['rows'][1]['wanted']);wait_for(lambda:row(2)['visible'] and not status()['preferences_pending'],'animated strip restored')
    report['gif_pixels_animate_and_hidden_animation_stops']=True
    original_bytes=configpath.read_bytes()
    # Embedded native selector imports a real local file into the detached draft.
    command('hide');focus_game();command('settings');wait_for(lambda:state(state='Passive',rendered=True,view='Settings'),'image settings passive')
    command('interact');wait_for(lambda:state(state='Interactive'),'image settings interaction');click(status()['preferences']['tabs'][4]);wait_for(lambda:status()['preferences']['page']==4,'image tab')
    def edit():return status()['preferences']['images']
    click(edit()['import']);wait_for(lambda:edit()['page']=='file','embedded file chooser')
    asset=runtime/'chosen.png';Image.new('RGB',(320,180),(220,50,30)).save(asset)
    # Only this private compositor receives text/navigation keys.
    wait_for(lambda:edit()['chooser'] and edit()['chooser']['mapped'],'file chooser mapped');time.sleep(.15)
    subprocess.run(['wtype','-M','ctrl','-k','l','-m','ctrl'],env=env,check=True);time.sleep(.12)
    subprocess.run(['wtype','-d','35','-M','ctrl','-k','a','-m','ctrl','--',str(asset)],env=env,check=True);time.sleep(.1)
    # GtkFileChooser resolves its location entry on Return, inside the private fixture.
    subprocess.run(['wtype','-k','Return'],env=env,check=True)
    wait_for(lambda:edit()['filename']==str(asset),'file selected')
    click(edit()['choose']);wait_for(lambda:edit()['page']=='form' and len(edit()['rows'])==2 and not edit()['pending'],'file imported into draft')
    assert configpath.read_bytes()==original_bytes
    files=list(upload_dir.iterdir());assert len(files)==2 and any(f.read_bytes()==asset.read_bytes() for f in files)
    # Reorder in the native draft, then save as one profile transaction.
    click(edit()['rows'][1]['up']);wait_for(lambda:edit()['rows'][0]['id']==2,'images reordered')
    click(status()['preferences']['apply']);wait_for(lambda:state(state='Hidden',game_active=True) and not status()['preferences_pending'],'image draft committed')
    saved=json.loads(configpath.read_text());assert [i['id'] for i in saved['image_strips'][0]['images']]==[2,1]
    report['native_file_chooser_import_reorder_and_persistence']=True
    command('images');wait_for(lambda:state(state='Passive',rendered=True),'manager reopened');command('interact');wait_for(lambda:gallery()['rendered'] and len(row(1)['cards'])==2 and row(1)['cards'][0]['loaded'],'imported image displayed')
    click(row(1)['cards'][0]['rect'],True);wait_for(lambda:gallery()['preview'] and gallery()['preview']['loaded'],'imported image preview')
    report['imported_image_native_thumbnail_and_preview']=True
    # Move via a bottom-right handle, preserving the selected anchor and actual geometry.
    target_button=status()['image_manager']['add'];origin=status()['origin'];pointer(origin['x']+target_button['x']+15,origin['y']+target_button['y']+15)
    wait_for(lambda:gallery()['preview'] is None,'preview closed before moving')
    def hover(id):
        r=row(id)['body'];pointer(r['x']+r['width']/2,r['y']+10);time.sleep(.06)
    hover(1);click(row(1)['controls']['move'],True);wait_for(lambda:row(1)['moving'],'image move mode')
    r=row(1)['body'];subprocess.run([str(target/'debug/examples/pointer'),str(r['x']+r['width']-3),str(r['y']+r['height']-3),'2560','1600','--drag-to','1500','1150'],env=env,check=True)
    wait_for(lambda:row(1)['anchor']=='br' and abs(row(1)['body']['x']+row(1)['body']['width']-1500)<2 and abs(row(1)['body']['y']+row(1)['body']['height']-1150)<2 and not status()['preferences_pending'],'image anchor dragged and committed')
    saved=json.loads(configpath.read_text());assert saved['image_strips'][0]['anchor']=='br' and abs(saved['image_strips'][0]['x']-1500/2560*100)<.001
    hover(1);click(row(1)['controls']['move'],True);wait_for(lambda:not row(1)['moving'],'image move mode left')
    report['native_drag_anchor_geometry_and_persistence']=True
    # Leaving the delete button while held cancels; a full hold removes only that strip.
    hover(2);r=row(2)['controls']['remove'];body=row(2)['body']
    subprocess.run([str(target/'debug/examples/pointer'),str(r['x']+r['width']//2),str(r['y']+r['height']//2),'2560','1600','--drag-to',str(body['x']+20),str(body['y']+20),'1150'],env=env,check=True)
    assert len(gallery()['rows'])==2
    hover(2);r=row(2)['controls']['remove'];subprocess.run([str(target/'debug/examples/pointer'),str(r['x']+r['width']//2),str(r['y']+r['height']//2),'2560','1600','--hold','1150'],env=env,check=True)
    wait_for(lambda:len(gallery()['rows'])==1 and not status()['preferences_pending'],'image strip removed after hold')
    assert json.loads(configpath.read_text())['image_strips'][0]['images'][0]['id']==2
    report['native_hold_cancel_and_targeted_strip_deletion']=True
    # Reopen passively then interact explicitly after each private output change.
    output_cases=[]
    for w,h,scale,lw,lh in [(1920,1080,1,1920,1080),(2560,1600,2,1280,800),(2560,1600,1,2560,1600)]:
        command('hide');focus_game();subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode',f'{w}x{h}@60Hz','--scale',str(scale)],env=env,check=True)
        command('images');wait_for(lambda:state(state='Passive',rendered=True),'manager after output change');command('interact')
        wait_for(lambda:gallery()['viewport']==[lw,lh] and gallery()['buffer_scale']==scale and gallery()['rendered'] and row(1)['cards'][1]['loaded'],'images sized for output')
        r=row(1);assert abs(r['body']['x']+r['body']['width']-r['x']/100*lw)<2 and abs(r['body']['y']+r['body']['height']-r['y']/100*lh)<2
        output_cases.append({'physical':[w,h],'scale':scale,'logical':[lw,lh]})
    report['private_output_scale_and_anchor_geometry']=output_cases
    # A voluntary switch wins while a preview is visible; returning never reopens it.
    click(row(1)['cards'][0]['rect'],True);wait_for(lambda:gallery()['preview'] and gallery()['preview']['loaded'],'preview before foreign app')
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True)
    other=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',str(first['other'])],env=env,text=True).splitlines());pointer(int(other['X'])+25,int(other['Y'])+25)
    wait_for(lambda:state(state='Hidden',game_active=False) and not gallery()['visible'] and gallery()['preview'] is None and gallery()['keyboard_mode']=='None','foreign application revokes image surface')
    focus_game();assert not gallery()['visible'] and gallery()['preview'] is None
    report['foreign_application_hides_preview_and_no_return_activation']=True
    subprocess.run(['xdotool','key','--clearmodifiers','shift+space'],env=env,check=True)
    wait_for(lambda:state(state='Interactive',view='Menu') and gallery()['rendered'] and gallery()['keyboard_mode']=='OnDemand','explicit shortcut restores images after a new first frame')
    report['explicit_interact_shortcut_reopens_wanted_images_after_paint']=True
    # Persisted sources and image order survive a real process restart.
    saved_bytes=configpath.read_bytes();previous_pid=daemon.pid;command('quit');assert daemon.wait(timeout=4)==0
    daemon=start([str(app),'daemon'],'overlay-restart');assert daemon.pid!=previous_pid
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'restarted socket')
    wait_for(lambda:state(state='Hidden',game_active=True) and 'Ok' in status()['shortcut_backend']['status'],'restart remains hidden')
    assert not gallery()['visible'] and gallery()['keyboard_mode']=='None' and configpath.read_bytes()==saved_bytes
    command('interact');wait_for(lambda:state(state='Interactive',view='Menu') and gallery()['rendered'] and row(1)['cards'][0]['loaded'] and row(1)['cards'][1]['loaded'],'saved images loaded after restart')
    assert [c['id'] for c in row(1)['cards']]==[2,1]
    report['restart_keeps_image_order_and_sources_but_starts_hidden']=True
    # Explicit hide revokes all input, and a foreign application never reopens it.
    command('hide');focus_game();assert not gallery()['visible'] and gallery()['keyboard_mode']=='None' and gallery()['preview'] is None and not gallery()['scheduled_animation']
    time.sleep(.2);before=gallery()['frames'];time.sleep(1);assert gallery()['frames']==before
    report['hidden_no_render_or_animation']=True
    assert 'trade search generation=' not in (runtime/'overlay.log').read_text()
    assert 'Gtk-CRITICAL' not in (runtime/'overlay.log').read_text(),(runtime/'overlay.log').read_text()[-3000:]
    assert 'Gtk-CRITICAL' not in (runtime/'overlay-restart.log').read_text(),(runtime/'overlay-restart.log').read_text()[-3000:]
    report.update(no_network=True,scope='Private headless River/XWayland; fake game, local fixture files, native chooser and images only',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-image-strips.py','src/image_strip.rs','src/desktop/images.rs','src/ui/image_panel.rs','src/ui/image_settings.rs','src/ui.rs']}
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
