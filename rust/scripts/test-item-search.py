#!/usr/bin/env python3
"""Native Heist search on a private River/XWayland desktop. Never touches the real game."""
import hashlib,json,os,pathlib,select,shutil,subprocess,sys,tempfile,time,urllib.parse,socket,threading
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
    original['hotkeys']={'catalogue':'Ctrl + H'}
    configpath.write_text(json.dumps(original));original_bytes=configpath.read_bytes()
    # A deterministic feed exercises success without making an external request.
    names=[('Replica Headhunter','Leather Belt','UniqueAccessory'),("Replica Kaom's Heart",'Glorious Plate','UniqueArmour'),("Replica Lioneye's Paws",'Bronzescale Boots','UniqueArmour'),("Replica Hyrri's Ire",'Zodiac Leather','UniqueArmour'),('Replica Shroud of the Lightless','Carnal Armour','UniqueArmour'),('Replica Dreamfeather','Eternal Sword','UniqueWeapon')]
    feed={'version':1,'league':'Standard','fetched':int(time.time()),'data':{'currencyOverviews':[{'type':'Currency','lines':[{'name':'Divine Orb','chaos':100,'variant':None}]}],'itemOverviews':[{'type':kind,'lines':[{'name':name,'variant':base,'chaos':i+1,'graph':[]}]} for i,(name,base,kind) in enumerate(names)]}}
    cachedir=pathlib.Path(env['XDG_CACHE_HOME'])/'awakened-poe-trade-rs/economy';cachedir.mkdir(parents=True);(cachedir/'Standard.json').write_text(json.dumps(feed))
    # A refusing proxy, with one controllable delayed CONNECT for the late reply.
    proxy=socket.socket();proxy.bind(('127.0.0.1',0));proxy.listen();proxy.settimeout(.2)
    stall_next=threading.Event();proxy_seen=threading.Event();proxy_release=threading.Event();proxy_stop=threading.Event()
    def proxy_loop():
        while not proxy_stop.is_set():
            try:client,_=proxy.accept()
            except socket.timeout:continue
            except OSError:break
            with client:
                client.settimeout(1)
                try:client.recv(16384)
                except OSError:pass
                if stall_next.is_set():
                    stall_next.clear();proxy_seen.set();proxy_release.wait(8)
    proxy_thread=threading.Thread(target=proxy_loop,daemon=True);proxy_thread.start()
    proxy_url='http://127.0.0.1:'+str(proxy.getsockname()[1]);env.update(HTTPS_PROXY=proxy_url,HTTP_PROXY=proxy_url,ALL_PROXY=proxy_url)
    daemon=start([str(app),'daemon'],'overlay')
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'initial shortcuts')
    focus_game()
    def catalogue():return status()['catalogue']
    def click(r):
        assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        o=status()['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def type_search(text):
        click(catalogue()['entry']['rect']);time.sleep(.06)
        subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--',text],env=env,check=True)
        wait_for(lambda:catalogue()['entry']['value']==text,'search typed')
    def open_catalogue(use_hotkey=False):
        command('hide');focus_game()
        if use_hotkey:subprocess.run(['xdotool','key','--clearmodifiers','ctrl+h'],env=env,check=True)
        else:command('catalogue')
        s=wait_for(lambda:state(state='Passive',rendered=True,view='Catalogue'),'catalogue passive')
        assert s['game_active'] and s['keyboard_mode']=='None' and not s['overlay_focused'],s
        command('interact');wait_for(lambda:state(state='Interactive'),'catalogue interactive')
    def select(name):
        type_search(name)
        wait_for(lambda:any(r['choice']['name']==name for r in catalogue()['results']),'matching result')
        c=next(r for r in catalogue()['results'] if r['choice']['name']==name)
        click(c['rect']);wait_for(lambda:catalogue()['entry']['value']=='','selection clears search')
        wait_for(lambda:any(r['choice']['name']==name and r['quote'] and r['quote'].get('quote') for r in catalogue()['selected']),'selected quote')
    def searches():return (runtime/'overlay.log').read_text().count('trade search generation=')
    def quote_jobs():return (runtime/'overlay.log').read_text().count('catalogue economy generation=')
    subprocess.run(['wl-copy','--type','text/plain;charset=utf-8'],input=(root/'src/fixtures/driftwood-wand.txt').read_text(),env=env,text=True,check=True,stderr=subprocess.DEVNULL)
    clipboard=subprocess.check_output(['wl-paste','--no-newline'],env=env)
    open_catalogue(True);report['configured_shortcut_opens_passively']=True;assert 'aucune ancienne gemme' in catalogue()['search_message']
    type_search('headhunter');assert not catalogue()['results'] and searches()==0 and quote_jobs()==0
    r=catalogue()['kind_control'];click(r);time.sleep(.15);o=status()['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']*1.5,True)
    wait_for(lambda:catalogue()['kind']=='replica' and len(catalogue()['results'])==1,'replica target filter')
    type_search('replica');assert not catalogue()['results'] and 'Précisez' in catalogue()['search_message']
    type_search('replica zzz');assert not catalogue()['results'] and 'Aucun' in catalogue()['search_message']
    assert searches()==0 and quote_jobs()==0
    report['typing_and_kind_changes_are_offline']=True
    command('hide');focus_game();subprocess.run([str(app),'catalogue','replica','Replica Headhunter'],env=env,check=True)
    s=wait_for(lambda:state(state='Passive',rendered=True,view='Catalogue'),'prefilled passive catalogue')
    assert s['game_active'] and s['keyboard_mode']=='None' and catalogue()['entry']['value']=='Replica Headhunter' and len(catalogue()['results'])==1
    assert searches()==0 and quote_jobs()==0
    command('interact');wait_for(lambda:state(state='Interactive'),'prefilled catalogue interaction')
    report['prefilled_query_is_passive_and_offline']=True
    names=['Replica Headhunter',"Replica Kaom's Heart","Replica Lioneye's Paws","Replica Hyrri's Ire",'Replica Shroud of the Lightless','Replica Dreamfeather']
    for i,name in enumerate(names):
        select(name)
        count=quote_jobs();select(name);assert quote_jobs()==count,'duplicate requested prices'
        p=catalogue();assert len(p['selected'])==(i+1 if i<5 else 1),p
        if i<5:assert p['selected'][0]['choice']['name']==name,p
    assert searches()==0 and catalogue()['selected'][0]['chaos']==6
    report['five_sorted_choices_duplicate_guard_and_sixth_replacement']=True
    report['quotes_from_seeded_dense_cache']=True
    # Settings cancellation restores this view without issuing a request.
    before=quote_jobs();click(status()['settings_button']);wait_for(lambda:state(view='Settings') and status()['preferences']['cancel']['width']>5,'settings from catalogue');click(status()['preferences']['cancel']);wait_for(lambda:state(view='Catalogue'),'return to catalogue');assert quote_jobs()==before
    # Explicit consultation adds the catalogue base missing from upstream's virtual item.
    click(catalogue()['selected'][0]['rect']);s=wait_for(lambda:status() if status()['view']=='Price' and status()['request'] else None,'virtual Trade query')
    assert s['state']=='Interactive' and s['item']['virtual'] and s['item']['reference']=='Replica Dreamfeather',s
    assert s['request']['query']['name']=='Replica Dreamfeather' and s['request']['query']['type']=='Eternal Sword',s['request']
    assert any(f['id']=='explicit.stat_3299347043' for g in s['request']['query']['stats'] for f in g['filters'])
    assert subprocess.check_output(['wl-paste','--no-newline'],env=env)==clipboard
    report['virtual_consultation_exact_unique_identity_clipboard_unchanged']=True
    wait_for(lambda:status()['query'] is not None,'blocked Trade terminates')
    open_catalogue();assert len(catalogue()['selected'])==1 and catalogue()['selected'][0]['quote'] is None
    before=quote_jobs();click(catalogue()['reset']);wait_for(lambda:not catalogue()['selected'],'clear basket');assert quote_jobs()==before
    select(names[0]);before=quote_jobs()
    subprocess.run([str(app),'league','Hardcore Standard'],env=env,check=True)
    wait_for(lambda:catalogue()['league']=='Hardcore Standard','new league')
    assert catalogue()['selected'][0]['quote'] is None and quote_jobs()==before
    # Delay the next proxy CONNECT, hide via a deliberate app switch, then fail it.
    stall_next.set();click(catalogue()['refresh']);assert proxy_seen.wait(3),'expected delayed economy CONNECT'
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True)
    geometry=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',str(first['other'])],env=env,text=True).splitlines())
    pointer(int(geometry['X'])+int(geometry['WIDTH'])/2,int(geometry['Y'])+int(geometry['HEIGHT'])/2)
    wait_for(lambda:state(state='Hidden',game_active=False),'switch hides catalogue');proxy_release.set()
    wait_for(lambda:'discarded catalogue economy' in (runtime/'overlay.log').read_text(),'late response discarded')
    time.sleep(.1);assert status()['state']=='Hidden' and status()['keyboard_mode']=='None'
    focus_game();assert status()['state']=='Hidden'
    report['league_revokes_quotes_and_late_response_cannot_reopen']=True
    open_catalogue();click(catalogue()['refresh']);wait_for(lambda:'indisponibles' in catalogue()['message'],'quote error is visible')
    assert all(i['quote'] is None for i in catalogue()['selected'])
    report['quote_error_keeps_unknown_price']=True
    report.update(scope='Private River/XWayland; seeded economy cache and refusing/delayed local proxy; no external services or real game input',binary_version=subprocess.check_output([str(app),'--version'],env=env,text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-item-search.py','src/item/search.rs','src/item.rs','src/ui/item_search_panel.rs','src/ui.rs','src/hotkeys.rs','src/ui/settings_panel.rs']}
    if len(sys.argv)>2:pathlib.Path(sys.argv[2]).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2));success=True

finally:
    if 'proxy_release' in globals():proxy_release.set();proxy_stop.set();proxy.close()
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
