#!/usr/bin/env python3
"""Native settings on a private River/XWayland desktop. Never touches the real game."""
import hashlib,json,os,pathlib,select,shutil,subprocess,sys,tempfile,time,urllib.parse,http.server,threading
root=pathlib.Path(__file__).resolve().parent.parent
os.chdir(root)
for name in ['river','riverctl','wlr-randr','xdotool','wl-copy','wtype']:
    assert shutil.which(name),name+' required'
subprocess.run(['cargo','build','--locked','--example','pointer','--example','overlay-fixture','--example','stash-fixture','-j','3'],check=True)
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
touch "$XDG_RUNTIME_DIR/ready"
''');init.chmod(0o700)
env=os.environ.copy();env.update(XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(runtime/'config'),XDG_CACHE_HOME=str(runtime/'cache'),XDG_STATE_HOME=str(runtime/'state'),APT_FIXTURE_ITEM='ring',GSETTINGS_BACKEND='memory',WLR_BACKENDS='headless',WLR_HEADLESS_OUTPUTS='1',WLR_RENDERER='pixman',GDK_BACKEND='wayland',HTTPS_PROXY='http://127.0.0.1:9',HTTP_PROXY='http://127.0.0.1:9',ALL_PROXY='http://127.0.0.1:9')
logs=[];processes=[];success=False;report={};httpd=None
class TradeFixture(http.server.BaseHTTPRequestHandler):
    def log_message(self,*args):pass
    def reply(self,body):
        data=json.dumps(body).encode();self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
    def do_POST(self):
        assert self.path.startswith('/api/trade/search/'),self.path
        self.rfile.read(int(self.headers.get('Content-Length','0')))
        self.reply({'id':'fixtureSearch','total':2,'result':['offerA','offerB']})
    def do_GET(self):
        assert self.path.startswith('/api/trade/fetch/'),self.path
        self.reply({'result':[{'listing':{'price':{'amount':price,'currency':'chaos'},'account':{'name':f'FixtureSeller{suffix}'}},'item':{'name':f'Fixture offered ring {suffix}','typeLine':'Gold Ring','frameType':2,'ilvl':84,'properties':[{'name':'Quality','values':[['+20%',1]]}],'implicitMods':['15% increased Rarity of Items found'],'explicitMods':[f'+{life} to maximum Life','+42% to Fire Resistance'],'craftedMods':['+16% to Chaos Resistance'],'identified':True}} for suffix,price,life in [('A',5,73),('B',12,110)]]})
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
    wait_for(lambda:(runtime/'ready').exists(),'private compositor rules')
    env.update(DISPLAY=(runtime/'display').read_text().strip(),WAYLAND_DISPLAY=(runtime/'wayland').read_text().strip())
    assert env['DISPLAY']!=os.environ.get('DISPLAY') and env['XDG_RUNTIME_DIR']!=os.environ.get('XDG_RUNTIME_DIR')
    subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode','2560x1600@60Hz'],env=env,check=True)
    foreign=start([str(target/'debug/examples/overlay-fixture'),'--isolated'],'foreign',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    assert select.select([foreign.stdout],[],[],3)[0];first=json.loads(foreign.stdout.readline())
    foreign.stdin.write('unmap\n');foreign.stdin.flush();assert select.select([foreign.stdout],[],[],3)[0];json.loads(foreign.stdout.readline())
    keyboard=start(['wtype','-s','60000','-k','Escape'],'keyboard')
    fixture=start([str(target/'debug/examples/stash-fixture')],'fixture',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    assert select.select([fixture.stdout],[],[],5)[0];assert json.loads(fixture.stdout.readline())['ready']
    configdir=pathlib.Path(env['XDG_CONFIG_HOME'])/'awakened-poe-trade-rs';configdir.mkdir(parents=True)
    configpath=configdir/'config.json'
    original={'league':'Standard','automatic_league':False,'automatic_search':False,'stopwatches':[{'legacy':'kept'}],'stash_scroll':True,'hotkeys':{'delve':'Ctrl + D'}}
    configpath.write_text(json.dumps(original));original_bytes=configpath.read_bytes()
    httpd=http.server.ThreadingHTTPServer(('127.0.0.1',0),TradeFixture);threading.Thread(target=httpd.serve_forever,daemon=True).start()
    env['APT_RS_TEST_TRADE_BASE']=f'http://127.0.0.1:{httpd.server_address[1]}'
    env['NO_PROXY']='127.0.0.1,localhost'
    daemon=start([str(app),'daemon'],'overlay');wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'shortcuts');focus_game()
    def click(r):
        s=status();assert s['state']=='Interactive' and s['visible'] and s['rendered'];assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        o=s['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def copy(key='ctrl+d',interactive=False):
        command('hide');focus_game();subprocess.run(['xdotool','key','--clearmodifiers',key],env=env,check=True)
        return wait_for(lambda:state(state='Interactive' if interactive else 'Passive',rendered=True,visible=True) and status()['item'] and status()['item']['base']=='Gold Ring','ring captured')
    def current_life():return next(r for r in status()['stat_controls'] if r['id']=='pseudo.pseudo_total_life')
    def reveal(row):
        for _ in range(40):
            r=row();v=status()['mod_scroll'];y=r['min']['y']+r['min']['height']/2
            if v['y']+8<y<v['y']+v['height']-8:return r
            o=status()['origin'];subprocess.run([str(target/'debug/examples/pointer'),str(int(o['x']+v['x']+v['width']-10)),str(int(o['y']+v['y']+v['height']/2)),'2560','1600','--scroll','60' if y>v['y']+v['height']/2 else '-60'],env=env,check=True);time.sleep(.06)
        raise RuntimeError('stat row unreachable')
    copy();s=status();assert s['keyboard_mode']=='None' and s['game_active'] and not s['overlay_focused'] and s['mod_expander']['expanded'];assert s['request'] is None
    assert not any(k in s for k in ['widget_canvas','widget_menu','delve','images','stash','stopwatches','catalogue'])
    report['ctrl_d_copies_rare_into_passive_checker_with_mods_expanded_and_no_auxiliary_surfaces']=True
    assert s['height']==1600 and s['origin']['y']==0 and s['panel_bounds']['height']==1600,s
    geom=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowfocus','getwindowgeometry','--shell'],env=env,text=True).splitlines())
    game_window=geom['WINDOW']
    edge=int(geom['X'])+int(geom['WIDTH'])-round(int(geom['HEIGHT'])*370/600)
    assert s['origin']['x']+s['width']==edge,(s['origin'],s['width'],edge)
    report['full_height_panel_right_edge_matches_inventory_left_edge']=True
    row=current_life()['select'];o=status()['origin'];pointer(o['x']+row['x']+row['width']-10,o['y']+row['y']+10,True);time.sleep(.1)
    assert status()['state']=='Passive' and status()['game_active'] and not status()['overlay_focused'] and current_life()['selected']
    report['passive_rectangles_pass_clicks_to_fake_game_without_selection_or_focus']=True
    copy('ctrl+alt+d',True);assert status()['keyboard_mode']=='OnDemand';time.sleep(.15)
    r=reveal(current_life);assert r['selected'] and r['min_enabled'];field=r['min'];click(dict(field,x=field['x']+6,width=28));time.sleep(.1)
    subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--','60'],env=env,check=True)
    assert status()['request'] is None
    if os.environ.get('APT_PRICE_SCREENSHOT'):subprocess.run(['grim',os.environ['APT_PRICE_SCREENSHOT']],env=env,check=True)
    click(status()['search_button']);wait_for(lambda:status()['request'] is not None,'explicit search query')
    assert current_life()['min_value']==60
    q=status()['request'];life=[f for group in q['query']['stats'] for f in group['filters'] if f['id']=='pseudo.pseudo_total_life'];assert len(life)==1 and life[0]['value']['min']==60,q
    report['native_life_minimum_60_reaches_trade_query_only_after_search']=True
    wait_for(lambda:status()['query'] and status()['query'].get('listings')==2 and status()['listings']['mapped_rows']==2,'loopback Trade offers rendered')
    wait_for(lambda:status()['listings']['offers'][0]['rect']['width']>=status()['width']-44,'full-width offer rows allocated')
    offers=status()['listings']['offers'];assert all(o['has_item_details'] for o in offers)
    assert '+73 to maximum Life' in offers[0]['tooltip'] and '+110 to maximum Life' not in offers[0]['tooltip']
    assert offers[0]['rect']['y']<1000,'offers are still compressed at the bottom'
    r=offers[0]['rect'];o=status()['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2)
    wait_for(lambda:status()['listings']['tooltip_queries']>0 and status()['listings']['tooltip_visible'],'native tooltip painted at offer hover')
    assert status()['state']=='Interactive' and status()['overlay_focused'],'hover tooltip changed keyboard owner'
    if os.environ.get('APT_HOVER_SCREENSHOT'):subprocess.run(['grim',os.environ['APT_HOVER_SCREENSHOT']],env=env,check=True)
    report['native_hover_shows_fetched_offer_stats_and_preserves_parent_focus']=True
    report['first_offered_item_life']=73;report['searched_item_life']=85
    r=status()['search_button'];o=status()['origin'];pointer(o['x']+r['x']+10,o['y']+r['y']+10)
    wait_for(lambda:not status()['listings']['tooltip_visible'],'tooltip closes on leaving offer')
    r=reveal(current_life)['select'];click(dict(r,x=r['x']+r['width']-14,y=r['y']+r['height']-14,width=8,height=8));wait_for(lambda:not current_life()['selected'] and status()['request'] is None,'deselect invalidates old query');assert not status()['listings']['visible']
    time.sleep(5.1);click(status()['search_button']);wait_for(lambda:status()['request'] is not None,'query without life')
    assert not any(f['id']=='pseudo.pseudo_total_life' for group in status()['request']['query']['stats'] for f in group['filters'])
    report['deselecting_a_mod_clears_prices_and_removes_it_from_next_query']=True
    click(status()['settings_button']);wait_for(lambda:state(view='Settings') and status()['preferences']['tabs'][1] and status()['preferences']['tabs'][1]['width']>5,'settings mapped');time.sleep(.15)
    p=status()['preferences'];assert len(p['tabs'])==2 and len(p['hotkeys'])==5
    click(p['cancel']);wait_for(lambda:state(view='Price'),'cancel returns checker');assert configpath.read_bytes()!=original_bytes # search persisted price defaults
    saved=json.loads(configpath.read_text());assert saved['stopwatches']==original['stopwatches'] and saved['stash_scroll']==original['stash_scroll'] and saved['hotkeys']['delve']=='Ctrl + D'
    click(status()['settings_button']);wait_for(lambda:state(view='Settings') and status()['preferences']['tabs'][1],'settings again');time.sleep(.15);click(status()['preferences']['tabs'][1]);wait_for(lambda:status()['preferences']['page']==1,'shortcut page');time.sleep(.15)
    click(status()['preferences']['record'][0]);wait_for(lambda:status()['preferences']['capture']==0,'key capture armed')
    subprocess.run(['wtype','-M','ctrl','-k','j','-m','ctrl'],env=env,check=True);wait_for(lambda:status()['preferences']['hotkeys'][0]['value']=='Ctrl + J','key captured')
    click(status()['preferences']['apply']);wait_for(lambda:state(state='Hidden') and not status()['preferences_pending'],'saved hidden')
    saved=json.loads(configpath.read_text());assert saved['hotkeys']['price']=='Ctrl + J' and saved['hotkeys']['delve']=='Ctrl + D' and saved['stopwatches']==original['stopwatches']
    focus_game();subprocess.run(['xdotool','key','--clearmodifiers','ctrl+d'],env=env,check=True);time.sleep(.2);assert status()['state']=='Hidden'
    copy('ctrl+j');report['price_only_settings_native_capture_rebind_and_historical_json_preservation']=True
    subprocess.run(['riverctl','set-focused-tags','2'],env=env,check=True)
    geo=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',str(first['other'])],env=env,text=True).splitlines());pointer(int(geo['X'])+int(geo['WIDTH'])/2,int(geo['Y'])+int(geo['HEIGHT'])/2)
    wait_for(lambda:state(state='Hidden',game_active=False),'foreign app hides');focus_game();assert status()['state']=='Hidden'
    report['switching_app_hides_and_return_does_not_reopen']=True
    for _ in range(10):copy('ctrl+j');command('hide');wait_for(lambda:state(state='Hidden',game_active=True),'repeat hidden')
    copy('ctrl+j');time.sleep(.3);frames=status()['frames'];time.sleep(1);assert status()['frames']==frames
    report['ten_capture_hide_cycles_and_static_panel_without_periodic_frames']=True
    def game_geometry():return dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',game_window],env=env,text=True).splitlines())
    subprocess.run(['riverctl','toggle-fullscreen'],env=env,check=True)
    wait_for(lambda:(int(game_geometry()['WIDTH']),int(game_geometry()['HEIGHT']))!=(2560,1600),'private fake game leaves fullscreen')
    def follows_game():
        g=game_geometry();s=status();edge=int(g['X'])+int(g['WIDTH'])-round(int(g['HEIGHT'])*370/600)
        return s['panel_bounds']['x']==max(0,min(edge-s['width'],2560-s['width']))
    wait_for(follows_game,'column follows X11 ConfigureNotify without output change')
    report['private_game_configure_event_updates_inventory_side_position']=True
    subprocess.run(['riverctl','toggle-fullscreen'],env=env,check=True)
    wait_for(lambda:(int(game_geometry()['WIDTH']),int(game_geometry()['HEIGHT']))==(2560,1600) and follows_game(),'private fake game fullscreen restored')
    layouts=[]
    for w,h,scale,logical_h in [(1920,1080,1,1080),(2560,1600,2,800),(2560,1600,1,1600)]:
        subprocess.run(['wlr-randr','--output','HEADLESS-1','--custom-mode',f'{w}x{h}@60Hz','--scale',str(scale)],env=env,check=True)
        wait_for(lambda:status()['height']==logical_h and status()['origin']['y']==0,'full-height output resize')
        def aligned():
            g=dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',game_window],env=env,text=True).splitlines())
            s=status();edge=int(g['X'])//scale+int(g['WIDTH'])//scale-round((int(g['HEIGHT'])//scale)*370/600)
            expected=max(0,min(edge-s['width'],w//scale-s['width']))
            return s['panel_bounds']['x']==expected
        wait_for(aligned,'column follows final XWayland game geometry after output change')
        layouts.append({'physical':[w,h],'output_scale':scale,'panel':status()['panel_bounds'],'allocated_width':status()['width'],'observed_x_client':game_geometry()})
    report['private_output_sizes_and_factor_2_keep_full_height']=layouts
    command('hide');fixture.terminate();fixture.wait(timeout=3);wait_for(lambda:not status()['desktop']['windows'] or not status()['game_active'],'fake game gone')
    fixture=start([str(target/'debug/examples/stash-fixture')],'fixture-recreated',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True);assert select.select([fixture.stdout],[],[],5)[0];json.loads(fixture.stdout.readline());focus_game();copy('ctrl+j');command('hide')
    report['fake_game_recreation_can_copy_again_without_restarting_service']=True
    focus_game();command('settings');wait_for(lambda:state(view='Settings',rendered=True) and status()['preferences']['automatic']['rect'] and status()['preferences']['automatic']['rect']['mapped'],'automatic-search settings');command('interact');wait_for(lambda:state(state='Interactive'),'settings interaction');time.sleep(.15)
    click(status()['preferences']['automatic']['rect']);wait_for(lambda:status()['preferences']['automatic']['value'],'automatic enabled')
    cache=pathlib.Path(env['XDG_CACHE_HOME'])/'awakened-poe-trade-rs'/'leagues.json';cache.parent.mkdir(parents=True,exist_ok=True);cache.write_text(json.dumps({'fetched':int(time.time()),'leagues':[{'id':'Standard'},{'id':'Fixture Challenge','category':{'current':True}},{'id':'Hardcore Fixture','category':{'current':True},'rules':[{'id':'Hardcore'}]}]}))
    click(status()['preferences']['automatic_league']['rect']);wait_for(lambda:status()['preferences']['automatic_league']['value'],'current league auto enabled')
    click(status()['preferences']['apply']);wait_for(lambda:state(state='Hidden') and not status()['preferences_pending'] and status()['league_selection']['ready'] and status()['league_selection']['label']=='Fixture Challenge','automatic current league selected without reopening')
    report['current_league_selected_automatically_from_private_cached_api_list_without_reopening']=True
    fixture.terminate();fixture.wait(timeout=3);wait_for(lambda:not status()['game_active'],'ring fixture gone')
    env['APT_FIXTURE_ITEM']='wand';fixture=start([str(target/'debug/examples/stash-fixture')],'fixture-wand',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True);assert select.select([fixture.stdout],[],[],5)[0];json.loads(fixture.stdout.readline());focus_game()
    subprocess.run(['xdotool','key','--clearmodifiers','ctrl+alt+d'],env=env,check=True)
    wait_for(lambda:state(state='Interactive',rendered=True,visible=True) and status()['item'] and status()['item']['base']=='Driftwood Wand' and status()['request'] is not None,'locked price checker after automatic search advances generation')
    report['locked_copy_interacts_after_first_paint_even_when_automatic_search_changes_generation']=True
    command('hide')
    log=(runtime/'overlay.log').read_text();assert '-CRITICAL' not in log
    lines=log.splitlines();lock_lines=[i for i,l in enumerate(lines) if 'state=Passive first_frame=painted' in l];assert lock_lines
    assert fixture_command('input')['submitted']==[]
    report.update(scope='Private River/XWayland with GTK fake game and synthetic rare ring. No real-game input; Trade uses a guarded loopback HTTP fixture with two offered items, other public network blocked by local invalid proxy. No live prices claimed.',binary_version=subprocess.check_output([str(app),'--version'],text=True).strip())
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-price-checker.py','examples/stash-fixture.rs','src/ui.rs','src/ui/stat_row.rs','src/ui/listings_panel.rs','src/trade_item.rs','src/trade.rs','src/ui/settings_panel.rs','src/settings.rs','src/price_layout.rs','src/leagues.rs','src/desktop/game_geometry.rs','src/desktop/preferences.rs','src/desktop/clipboard.rs','src/desktop/xwayland_shortcuts.rs','src/hotkeys.rs']}
    if len(sys.argv)>2:pathlib.Path(sys.argv[2]).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2));success=True

finally:
    if not success:
        try:(runtime/'status.json').write_text(command('status'))
        except Exception:pass
    if not success and shutil.which('grim') and 'WAYLAND_DISPLAY' in env:
        subprocess.run(['grim',str(runtime/'failure.png')],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=3)
    if httpd is not None:httpd.shutdown();httpd.server_close()
    for p in reversed(processes):
        if p.poll() is None:
            p.terminate()
            try:p.wait(timeout=3)
            except subprocess.TimeoutExpired:p.kill();p.wait()
    for log in logs:log.close()
    if success:shutil.rmtree(runtime)
    else:print('Diagnostic logs preserved at '+str(runtime),file=sys.stderr)
