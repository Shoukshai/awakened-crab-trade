#!/usr/bin/env python3
"""Real overlay on a private River/XWayland desktop. Never touches the real game."""
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
    assert s['game_active'] and s['keyboard_mode']=='None' and (s['stat_controls'] or s.get('mercenary_controls')),s
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
    daemon=start([str(app),'daemon'],'overlay')
    wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    focus_game()
    subprocess.run(['wl-copy','--type','text/plain;charset=utf-8'],input=(root/'src/fixtures/driftwood-wand.txt').read_text(),env=env,text=True,check=True,stderr=subprocess.DEVNULL)
    command('demo');s=wait_for(lambda:state(state='Passive',rendered=True),'demo')
    assert s['game_active'] and s['keyboard_mode']=='None' and not s['overlay_focused'],s
    command('hide');wait_for(lambda:state(state='Hidden',keyboard_mode='None'),'demo hidden')
    show_price();command('interact');wait_for(lambda:state(state='Interactive',keyboard_mode='OnDemand'),'interactive')
    # Open the real GTK expander with the same virtual pointer as normal clicks.
    s=status();rect=s['mod_expander'];origin=s['origin']
    pointer(origin['x']+rect['x']+10,origin['y']+rect['y']+10,True)
    def mapped_filter():
        s=status();rect=s['stat_controls'][0]['select']
        return s if rect and rect['mapped'] and rect['width']>20 and rect['height']>10 else None
    s=wait_for(mapped_filter,'stat list mapped')
    clicks=[]
    for index in range(30):
        before=status();rect=before['stat_controls'][0]['select'];origin=before['origin']
        x=origin['x']+rect['x']+10;y=origin['y']+rect['y']+rect['height']/2
        assert 50<x<670 and 100<y<1500,(x,y)
        pointer(x,y,True)
        after=wait_for(lambda:status() if status()['generation']>before['generation'] else None,'edit generation')
        assert after['state']=='Interactive' and after['visible'] and after['keyboard_mode']=='OnDemand',after
        assert after['stat_controls'][0]['selected']!=before['stat_controls'][0]['selected'],after
        assert after['request'] is None and after['query'] is None,after
        clicks.append({'generation':after['generation'],'x':x,'y':y,'focused':after['overlay_focused']})
    report['interactive_clicks']=len(clicks)
    command('hide');focus_game()
    subprocess.run(['wl-copy','--type','text/plain;charset=utf-8'],input=(root/'src/fixtures/synthetic-crafting-ring.txt').read_text(),env=env,text=True,check=True,stderr=subprocess.DEVNULL)
    show_price();command('interact');s=wait_for(lambda:state(state='Interactive'),'preset interactive')
    r=s['preset_control'];o=s['origin']
    pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def popup_open():
        s=status();return s if s['state']=='Interactive' and s['preset_control']['popup'] else None
    wait_for(popup_open,'native preset popup')
    time.sleep(.25)
    assert popup_open(),'popup lost focus before selection'
    pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']*1.5,True)
    s=wait_for(lambda:state(state='Interactive',preset='base'),'preset selected')
    assert s['request'] is None and s['query'] is None and s['keyboard_mode']=='OnDemand',s
    time.sleep(.1)
    assert state(state='Interactive',preset='base',visible=True),'focus check hid selected preset'
    report['native_preset_popup']=True
    report['popup_input']='pointer; virtual keyboard commit remains unvalidated'
    s=status();r=s['preset_control'];o=s['origin']
    pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    wait_for(popup_open,'popup before explicit hide')
    command('hide')
    s=wait_for(lambda:state(state='Hidden',visible=False,keyboard_mode='None',owned_popup=False),'hide dismisses popup')
    assert not s['preset_control']['popup'],s
    time.sleep(.1)
    assert state(state='Hidden',visible=False,keyboard_mode='None'),'pending focus check reopened panel'
    report['hide_dismisses_popup']=True
    show_price();command('interact');wait_for(lambda:state(state='Interactive'),'interactive before app switch')
    # Another application is selected by tags on this private server only.
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True)
    geometry=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',str(first['other'])],env=env,text=True).splitlines())
    pointer(int(geometry['X'])+int(geometry['WIDTH'])/2,int(geometry['Y'])+int(geometry['HEIGHT'])/2)
    hidden=wait_for(lambda:state(state='Hidden',visible=False,keyboard_mode='None'),'app switch hides')
    report['application_switch_reason']=hidden['last_hide_reason']
    focus_game();s=status();assert s['state']=='Hidden' and not s['visible'],s
    for interactive in [False,True]:
        show_price()
        if interactive:command('interact');wait_for(lambda:state(state='Interactive'),'interactive before unmap')
        fixture_command('unmap');s=wait_for(lambda:state(state='Hidden',game_active=False),'game disappears')
        assert s['last_hide_reason']=='GameGone',s
        fixture_command('map');focus_game();assert status()['state']=='Hidden'
    report['unmap_remap_modes']=['Passive','Interactive']
    # Special item controls use the same native panel and generation policy.
    command('hide');focus_game()
    subprocess.run(['wl-copy','--type','text/plain;charset=utf-8'],input=(root/'src/fixtures/synthetic-logbook.txt').read_text(),env=env,text=True,check=True,stderr=subprocess.DEVNULL)
    s=show_price();assert s['preset']=='I',s
    assert [f['id'] for f in s['stat_controls']]==['pseudo.pseudo_logbook_faction_mercenaries','implicit.stat_3159649981|2'],s
    command('interact');s=wait_for(lambda:state(state='Interactive'),'logbook interactive')
    r=s['preset_control'];o=s['origin']
    pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    wait_for(popup_open,'logbook zone menu')
    pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']*1.5,True)
    s=wait_for(lambda:state(state='Interactive',preset='II'),'logbook second zone')
    time.sleep(.1);s=status()
    assert s['state']=='Interactive' and s['query'] is None and s['request'] is None,s
    assert [f['id'] for f in s['stat_controls']]==['pseudo.pseudo_logbook_faction_druids'],s
    report['logbook_zone_change']={'from':'I','to':'II','clears_prices':True,'faction_and_boss_isolated':True}
    command('hide');focus_game()
    subprocess.run(['wl-copy','--type','text/plain;charset=utf-8'],input=(root/'src/fixtures/synthetic-mercenary.txt').read_text(),env=env,text=True,check=True,stderr=subprocess.DEVNULL)
    s=show_price();assert s['mercenary_controls'],s
    command('interact');wait_for(lambda:state(state='Interactive'),'mercenary interactive')
    s=status();r=s['mod_expander'];o=s['origin']
    if not r['expanded']:
        pointer(o['x']+r['x']+10,o['y']+r['y']+10,True)
    wait_for(lambda:status()['mod_expander']['expanded'],'mercenary filters expanded')
    def control(child):return next(c for c in status()['mercenary_controls'] if c['node']==0 and c['child']==child)
    def mapped_support_expander():
        r=control(None)['expander']['rect']
        return r if r and r['mapped'] and r['width']>20 and r['height']>10 else None
    r=wait_for(mapped_support_expander,'support expander mapped');o=status()['origin']
    pointer(o['x']+r['x']+10,o['y']+r['y']+10,True)
    wait_for(lambda:control(None)['expander']['expanded'],'support expander opened')
    def merc_click(child,key='select',value_key=None):
        for _ in range(24):
            s=status();c=next(c for c in s['mercenary_controls'] if c['node']==0 and c['child']==child)
            r=c[key] if key=='select' else c[key]['rect'];assert r and r['mapped'] and r['width']>10 and r['height']>10,c
            v=s['mod_scroll'];o=s['origin'];y=r['y']+r['height']/2
            if v['y']+12<y<v['y']+v['height']-12:
                pointer(o['x']+r['x']+(10 if key=='select' else r['width']/2),o['y']+y,True);time.sleep(.05);return
            delta=10 if y>v['y']+v['height']/2 else -10
            subprocess.run([str(target/'debug/examples/pointer'),str(o['x']+v['x']+v['width']//2),str(o['y']+v['y']+v['height']//2),'2560','1600','--scroll',str(delta)],env=env,check=True)
            time.sleep(.05)
            assert status()['generation']==s['generation'],'scroll edited a support instead of the list'
            report['mercenary_scroll_events']=report.get('mercenary_scroll_events',0)+1
        raise RuntimeError('Mercenary control cannot be scrolled into view: '+json.dumps({'control':c,'viewport':v}))
    merc_click(None);assert control(None)['selected']
    merc_click(0);assert control(0)['selected']
    merc_click(0,'minimum')
    s=status();r=control(0)['minimum']['rect'];o=s['origin'];before_min=control(0)['minimum']['value']
    assert before_min<5
    pointer(o['x']+r['x']+r['width']-10,o['y']+r['y']+r['height']/2,True)
    wait_for(lambda:control(0)['minimum']['value']==before_min+1,'minimum tier count incremented')
    for child in [1,2]:
        merc_click(child);assert control(child)['selected']
        merc_click(child,'option');wait_for(lambda:status()['owned_popup'],'support mode popup')
        s=status();r=control(child)['option']['rect'];o=s['origin']
        pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']*1.5,True)
        wait_for(lambda:control(child)['option']['value']=='0','optional support selected')
        time.sleep(.1);assert status()['state']=='Interactive'
    assert status()['request'] is None and status()['query'] is None
    command('prices');s=wait_for(lambda:status() if status()['request'] is not None else None,'mercenary request')
    assert any(g['type']=='mercenary' for g in s['request']['query']['stats']),s['request']
    link=urllib.parse.urlparse(s['trade_link']);assert link.hostname=='www.pathofexile.com'
    assert json.loads(urllib.parse.parse_qs(link.query)['q'][0])==s['request']
    merc_click(None);assert not control(None)['selected']
    assert status()['request'] is None and status()['query'] is None and status()['trade_link'] is None
    command('prices');s=wait_for(lambda:status() if status()['request'] is not None else None,'disabled group request')
    assert all(g['type']!='mercenary' for g in s['request']['query']['stats']),s['request']
    assert s['state']=='Interactive' and s['keyboard_mode']=='OnDemand'
    assert report.get('mercenary_scroll_events',0)>0,'scroll coverage is required'
    report['mercenary_editor']={'six_links':True,'minimum_edited':True,'optional_supports':2,'group_disable_suspends_children':True,'edits_clear_prices':True,'pointer_scroll':True,'browser_fallback_keeps_query':True}
    command('hide');focus_game()
    show_price();fixture_command('destroy');s=wait_for(lambda:state(state='Hidden',game_active=False),'game destroyed');assert s['last_hide_reason']=='GameGone',s
    new=fixture_command('create');assert new['game']!=first['game'];focus_game();assert status()['state']=='Hidden'
    show_price();command('hide');assert status()['keyboard_mode']=='None'
    report['new_window_rediscovered']=True
    report['network']='external services blocked by private proxy environment'
    report['scope']='private headless River/XWayland at 2560x1600; fake game only; no real PoE input'
    report['binary_version']=subprocess.check_output([str(app),'--version'],env=env,text=True).strip()
    report['source_sha256']={name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in ['scripts/test-overlay-lifecycle.py','examples/overlay-fixture.rs','src/desktop/wayland.rs','src/overlay.rs','src/ui.rs','src/ui/stat_row.rs','src/ui/mercenary_panel.rs','src/item/mercenary.rs','src/item/mercenary/query.rs']}
    if len(sys.argv)>2:pathlib.Path(sys.argv[2]).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2));success=True
finally:
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
