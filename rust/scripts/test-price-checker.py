#!/usr/bin/env python3
"""Native settings on a private River/XWayland desktop. Never touches the real game."""
from PIL import Image
import hashlib,json,os,pathlib,select,shutil,subprocess,sys,tempfile,time,urllib.parse,http.server,threading
root=pathlib.Path(__file__).resolve().parent.parent
os.chdir(root)
for name in ['river','riverctl','wlr-randr','xdotool','wl-copy','wtype']:
    assert shutil.which(name),name+' required'
subprocess.run(['cargo','build','--locked','--example','pointer','--example','overlay-fixture','--example','stash-fixture','--example','native-key','-j','3'],check=True)
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
env=os.environ.copy();env.update(XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(runtime/'config'),XDG_CACHE_HOME=str(runtime/'cache'),XDG_STATE_HOME=str(runtime/'state'),APT_FIXTURE_ITEM=os.environ.get('APT_FAMILY_CASE','ring'),GSETTINGS_BACKEND='memory',WLR_BACKENDS='headless',WLR_HEADLESS_OUTPUTS='1',WLR_RENDERER='pixman',GDK_BACKEND='wayland',HTTPS_PROXY='http://127.0.0.1:9',HTTP_PROXY='http://127.0.0.1:9',ALL_PROXY='http://127.0.0.1:9')
logs=[];processes=[];success=False;report={};httpd=None;persistent=None
posts=[]
ninja_requests=[]
from trade_fixture import make_handler
TradeFixture=make_handler(root,env,posts,ninja_requests)

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
def pointer_command(line):
    assert persistent is not None
    persistent.stdin.write(line+'\n');persistent.stdin.flush()
    assert select.select([persistent.stdout],[],[],3)[0],'persistent pointer stalled'
    assert persistent.stdout.readline().strip()=='ok'
def pointer(x,y,click=False):
    if persistent is not None:
        pointer_command(f'move {int(x)} {int(y)}')
        if click:
            pointer_command('down');time.sleep(.08);pointer_command('up')
    else:
        args=[str(target/'debug/examples/pointer'),str(int(x)),str(int(y)),'2560','1600']
        if click:args+=['--click']
        subprocess.run(args,env=env,check=True)
def drag_slider(rect,start,end):
    o=status()['origin'];left=o['x']+rect['x']+6;span=rect['width']-12;y=o['y']+rect['y']+rect['height']/2
    pointer(left+span*start,y);pointer_command('down');time.sleep(.08)
    pointer(left+span*end,y);time.sleep(.08);pointer_command('up')

def capture_panel(path):
    s=status();o=s['origin'];subprocess.run(['grim','-g',f"{o['x']},{o['y']} {s['width']}x{s['height']}",path],env=env,check=True)
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
    env['APT_RS_TEST_ECONOMY_BASE']=env['APT_RS_TEST_TRADE_BASE']
    env['NO_PROXY']='127.0.0.1,localhost'
    daemon=start([str(app),'daemon'],'overlay');wait_for(lambda:(runtime/'awakened-poe-trade-rs.sock').exists(),'daemon socket')
    if not os.environ.get('APT_TRANSIENT_POINTER'):
        persistent=start([str(target/'debug/examples/pointer'),'0','0','2560','1600','--persistent'],'persistent-pointer',stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    wait_for(lambda:'Ok' in status()['shortcut_backend']['status'],'shortcuts');focus_game()
    def click(r):
        s=status();assert s['state']=='Interactive' and s['visible'] and s['rendered'];assert r and r.get('mapped',True) and r['width']>5 and r['height']>5,r
        o=s['origin'];pointer(o['x']+r['x']+r['width']/2,o['y']+r['y']+r['height']/2,True)
    def choose_menu(name,index):
        click(status()['item_filter_controls'][name]['rect'])
        popup=wait_for(lambda:status()['popup_controls'] if status()['popup_controls'] and status()['popup_controls']['mapped'] else None,'menu mapped')
        item=popup['items'][index];o=status()['origin'];p=popup['origin']
        pointer(o['x']+p['x']+item['x']+item['width']/2,o['y']+p['y']+item['y']+item['height']/2,True)
        wait_for(lambda:not status()['owned_popup'] and state(state='Interactive'),'menu mouse choice keeps checker open')
    def copy(key='ctrl+d',interactive=False):
        command('hide');focus_game();time.sleep(.31);subprocess.run(['xdotool','key','--clearmodifiers',key],env=env,check=True)
        return wait_for(lambda:state(state='Interactive' if interactive else 'Passive',rendered=True,visible=True) and status()['item'] and status()['item']['base']=='Gold Ring','ring captured')
    def current_life():return next(r for r in status()['stat_controls'] if r['id']=='pseudo.pseudo_total_life')
    def reveal(row):
        for _ in range(40):
            r=row();v=status()['mod_scroll'];y=r['min']['y']+r['min']['height']/2
            if v['y']+8<y<v['y']+v['height']-8:return r
            o=status()['origin'];subprocess.run([str(target/'debug/examples/pointer'),str(int(o['x']+v['x']+v['width']-10)),str(int(o['y']+v['y']+v['height']/2)),'2560','1600','--scroll','60' if y>v['y']+v['height']/2 else '-60'],env=env,check=True);time.sleep(.06)
        raise RuntimeError('stat row unreachable')
    if os.environ.get('APT_FAMILY_CASE'):
        family=os.environ['APT_FAMILY_CASE']
        command('hide');focus_game();subprocess.run(['xdotool','key','--clearmodifiers','ctrl+alt+d'],env=env,check=True)
        wait_for(lambda:state(state='Interactive',visible=True,rendered=True) and status()['item'],'family copied')
        expected={'gem':'Gem','currency':'Currency','unique':'Unique','map':'Map','viscera':'Viscera','viscera-siren':'Viscera','influence':'Equipment','wand':'Equipment'}[family]
        assert status()['family']==expected,status()
        wait_for(lambda:status()['summary']['image_ready'],'item image fetched through fixture',10)
        if family not in ['influence','wand']:wait_for(lambda:status()['summary']['source'] and status()['summary']['price'] not in ['…','—'],'exact Ninja quote',10)
        s0=status()
        assert s0['summary']['mapped'] and s0['summary']['height']>=100,s0['summary']
        assert not posts,'copy must not automatically query Trade with automatic_search=false'
        if family=='wand':
            row=next(r for r in status()['stat_controls'] if r['id']=='implicit.stat_2974417149')
            assert row['bounds']=={'min':8.0,'max':12.0,'roll':8.0},row
            click(row['slider'])
            wait_for(lambda:next(r for r in status()['stat_controls'] if r['id']=='implicit.stat_2974417149')['min_value']==10,'slider single click sets min')
            row=next(r for r in status()['stat_controls'] if r['id']=='implicit.stat_2974417149')
            assert row['selected'] and not row['max_enabled']
            drag_slider(row['slider'],0.5,0.75)
            wait_for(lambda:next(r for r in status()['stat_controls'] if r['id']=='implicit.stat_2974417149')['min_value']==11,'slider drag sets min without toggling row')
            assert abs(row['min']['y']-row['max']['y'])<=1 and abs(row['min']['x']+row['min']['width']-row['max']['x'])<=1,row
            for field,text in [(row['min'],'9'),(row['max'],'40')]:
                click(field);subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--',text],env=env,check=True)
            assert status()['request'] is None
            row=next(r for r in status()['stat_controls'] if r['id']=='implicit.stat_2974417149')
            click(row['max']);subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--','11'],env=env,check=True)
            drag_slider(row['slider'],0.75,1.0)
            wait_for(lambda:next(r for r in status()['stat_controls'] if r['id']=='implicit.stat_2974417149')['max_value']==12,'second slider handle sets max')
            click(row['max']);subprocess.run([str(target/'debug/examples/native-key'),'ctrl-a'],env=env,check=True);subprocess.run([str(target/'debug/examples/native-key'),'backspace'],env=env,check=True)
            wait_for(lambda:not next(r for r in status()['stat_controls'] if r['id']=='implicit.stat_2974417149')['max_enabled'],'empty Max removes bound')
            click(row['min']);subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--','abc'],env=env,check=True)
            click(status()['search_button']);wait_for(lambda:status()['status_text']=='Valeur Min/Max invalide.','invalid number refuses search')
            assert not posts and status()['request'] is None
            click(row['min']);subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--','9'],env=env,check=True)
            click(row['max']);subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--','4'],env=env,check=True)
            click(status()['search_button']);wait_for(lambda:status()['status_text']=='Min doit être inférieur ou égal à Max.','inverted bounds refuse search')
            assert not posts and status()['request'] is None
            click(row['max']);subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--','40'],env=env,check=True)
            for fraction in [.12,.27,.53,.88]:
                drag_slider(row['slider'],fraction,fraction)
                assert all(r['min_value'] is None or float(r['min_value']).is_integer() for r in status()['stat_controls'])
            drag_slider(row['slider'],.5,.5)
            s1=status();o=s1['origin'];slider=next(r for r in s1['stat_controls'] if r['id']=='implicit.stat_2974417149')['slider']
            subprocess.run(['grim',str(runtime/'slider-colors.png')],env=env,check=True)
            picture=Image.open(runtime/'slider-colors.png').convert('RGB');y=o['y']+slider['y']+7
            gray=picture.getpixel((int(o['x']+slider['x']+slider['width']*.25),int(y)))
            yellow=picture.getpixel((int(o['x']+slider['x']+slider['width']*.75),int(y)))
            assert max(gray)<80 and yellow[0]>180 and yellow[1]>150 and yellow[2]<130,(gray,yellow)
            report['slider_integer_thresholds_and_yellow_gray_colors']={'gray':gray,'yellow':yellow}
            for field,text in [(row['min'],'9'),(row['max'],'40')]:
                click(field);subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--',text],env=env,check=True)
            report['slider_click_drag_both_handles_and_joined_fields']=True
            report['blank_bound_is_removed_and_invalid_ranges_send_no_trade_request']=True
        if family=='currency':
            assert not s0['search_actions_visible'] and s0['request'] is None and s0['economy']['stack']['have']==2,s0
            assert ninja_requests
            if os.environ.get('APT_PANEL_SCREENSHOT'):capture_panel(os.environ['APT_PANEL_SCREENSHOT'])
        else:
            click(status()['search_button']);wait_for(lambda:status()['query'] and status()['query'].get('listings')==2,'family Trade results')
            wait_for(lambda:status()['listings']['mapped_rows']==2,'family offers allocated')
            q=status()['request']['query'];misc=q.get('filters',{}).get('misc_filters',{}).get('filters',{})
            if family=='wand':
                mods=[f for g in q['stats'] for f in g['filters'] if f['id']=='implicit.stat_2974417149']
                assert len(mods)==1 and mods[0]['value']=={'min':9.0,'max':40.0},q
                tooltip=status()['listings']['offers'][0]['tooltip']
                assert all(t in tooltip for t in ['30% increased Spell Damage','+21 to Intelligence','+100 to maximum Mana']),tooltip
                assert '8% increased Spell Damage' not in tooltip,tooltip
            if family=='gem':
                assert misc['gem_level']=={'min':20,'max':20} and misc['quality']=={'min':20,'max':20} and misc['corrupted']['option']=='true',q
                assert 'ilvl' not in misc and not status()['stat_controls'],status()
            if family=='unique':assert q['name']=='Goldrim' and 'type' not in q,q
            if family=='map':assert 'ilvl' not in misc and q['filters']['map_filters']['filters']['map_tier']=={'min':16,'max':16},q
            if family=='viscera':
                assert q['filters']['type_filters']['filters']['category']['option']=='viscera.head',q
                assert status()['item']['reference']=='Severed Rhoa Head',status()['item']
                assert any(f['id']=='implicit.stat_1417918566' for g in q['stats'] for f in g['filters']),q
            if family=='viscera-siren':
                assert q['filters']['type_filters']['filters']['category']['option']=='viscera.leg',q
                assert status()['item']['reference']=="Siren's Tentacle",status()['item']
                assert any(f['id']=='implicit.stat_1793818220' for g in q['stats'] for f in g['filters']),q
                assert '27% increased Effect of Cold Ailments' in status()['listings']['offers'][0]['tooltip']
            if family=='influence':assert any(r['influence_icons'] for r in status()['stat_controls']),status()['stat_controls']
            pointer(status()['origin']['x']+8,1500)
            wait_for(lambda:not status()['listings']['tooltip_visible'],'no hover')
            if os.environ.get('APT_PANEL_SCREENSHOT'):capture_panel(os.environ['APT_PANEL_SCREENSHOT'])
            row=status()['listings']['offers'][0]['rect'];o=status()['origin'];pointer(o['x']+row['x']+row['width']-6,o['y']+row['y']+row['height']/2)
            wait_for(lambda:status()['listings']['tooltip_visible'],'whole row hover')
            if os.environ.get('APT_HOVER_SCREENSHOT'):capture_panel(os.environ['APT_HOVER_SCREENSHOT'])
        report.update(binary_version=subprocess.check_output([str(app),'--version'],text=True).strip(),binary_sha256=hashlib.sha256(app.read_bytes()).hexdigest(),family=family,summary=status()['summary'],item_filters=status()['item_filters'],request=status()['request'],trade_posts=len(posts),ninja_fetches=len(ninja_requests),scope='Private native compositor, synthetic family clipboard and synthetic market quotes; actual credited game artwork. No real game input or live prices.')
        assert fixture_command('input')['submitted']==[]
        assert '-CRITICAL' not in (runtime/'overlay.log').read_text()
        if len(sys.argv)>2:pathlib.Path(sys.argv[2]).write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps(report,indent=2));success=True
        sys.exit(0)
    command('hide');focus_game();time.sleep(.1)
    subprocess.run(['grim',str(runtime/'before-veil.png')],env=env,check=True)
    before_pixel=Image.open(runtime/'before-veil.png').getpixel((2300,1200))[:3]
    for interactive in [False,True]:
        copy('ctrl+alt+d' if interactive else 'ctrl+d',interactive)
        wait_for(lambda:status()['backdrop']['input_ready'],'painted backdrop ready')
        assert status()['backdrop']['keyboard_mode']=='None'
        time.sleep(.1)
        subprocess.run(['grim',str(runtime/'with-veil.png')],env=env,check=True)
        after_pixel=Image.open(runtime/'with-veil.png').getpixel((2300,1200))[:3]
        assert all(abs(b*.82-a)<=3 for a,b in zip(after_pixel,before_pixel)),(before_pixel,after_pixel)
        report['dark_veil_pixel_before_after']=[before_pixel,after_pixel]
        before=fixture_command('input')['clicks']
        pointer(2300,1200,True)
        wait_for(lambda:state(state='Hidden',visible=False) and not status()['backdrop']['visible'],'outside click closes both surfaces')
        assert fixture_command('input')['clicks']==before,'outside click leaked to fake game'
    report['outside_click_closes_passive_and_interactive_without_forwarding_to_game']=True
    for _ in range(3):
        copy('ctrl+alt+d',True)
        base=status()['item_filter_controls']['base'];click(base['rect'])
        wait_for(lambda:state(state='Interactive') and status()['item_filter_controls']['base']['active']!=base['active'],'first click after render remains interactive')
        command('hide')
    report['first_click_after_open_works_across_three_cycles']=True
    if os.environ.get('APT_PERSISTENT_ONLY'):
        print(json.dumps({'persistent_pointer_outside_then_inside_repeated':True,**report},indent=2));success=True;sys.exit(0)
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
    for dx,y in [(4,80),(status()['width']-4,80),(status()['width']/2,1250)]:
        o=status()['origin'];pointer(o['x']+dx,y,True);time.sleep(.08)
        assert status()['state']=='Interactive','inside click closed checker'
    # Wheel over a closed dropdown scrolls the enclosing list without editing filters.
    section=next(s for s in status()['stat_sections'] if s['label'].startswith('Autres stats'))
    click(section['header']);wait_for(lambda:status()['mod_scroll']['upper']>status()['mod_scroll']['page_size'],'expanded secondary stats overflow')
    before=status();combo=before['item_filter_controls']['corruption']['rect'];o=before['origin']
    pointer(o['x']+combo['x']+combo['width']/2,o['y']+combo['y']+combo['height']/2);time.sleep(.1);pointer_command('scroll 45')
    wait_for(lambda:status()['mod_scroll']['value']>before['mod_scroll']['value'],'wheel scrolls parent over dropdown')
    assert status()['generation']==before['generation'] and status()['item_filters']==before['item_filters']
    o=status()['origin'];v=status()['mod_scroll']
    pointer(o['x']+v['x']+v['width']-8,o['y']+v['y']+80);time.sleep(.1);pointer_command('scroll -1000')
    wait_for(lambda:status()['mod_scroll']['value']==0,'restore filter scroll')
    if os.environ.get('APT_SECTIONS_SCREENSHOT'):capture_panel(os.environ['APT_SECTIONS_SCREENSHOT'])
    click(next(s for s in status()['stat_sections'] if s['label'].startswith('Autres stats'))['header'])
    report['closed_dropdown_wheel_keeps_filters_and_scrolls_parent']=True
    click(status()['item_filter_controls']['corruption']['rect']);wait_for(lambda:status()['owned_popup'],'corruption dropdown grabbed')
    pointer(2300,1200,True)
    wait_for(lambda:not status()['owned_popup'] and state(state='Interactive'),'outside click dismisses dropdown only')
    choose_menu('corruption',2)
    choose_menu('corruption',1)
    report['inside_padding_fields_and_owned_dropdown_do_not_dismiss']=True
    report['outside_click_with_open_menu_dismisses_only_the_menu']=True
    if os.environ.get('APT_CLOSURE_ONLY'):
        print(json.dumps(report,indent=2));success=True;sys.exit(0)
    controls=status()['item_filter_controls'];base=controls['base'];assert base['sensitive']
    assert not base['active'] and base['label']=='Gold Ring'
    initial_ilvl=next(n for n in controls['numbers'] if n['name']=='ItemLevel')
    assert not initial_ilvl['active'] and initial_ilvl['value']=='83' and not initial_ilvl['field_sensitive']
    click(base['rect']);wait_for(lambda:status()['item_filter_controls']['base']['active'] != base['active'],'same base toggled')
    assert status()['request'] is None
    click(status()['search_button']);wait_for(lambda:status()['request'] is not None,'base filter query')
    q=status()['request']['query'];assert ('type' in q)==(not base['active']),q
    time.sleep(5.1)
    click(status()['item_filter_controls']['base']['rect'])
    ilvl=next(n for n in status()['item_filter_controls']['numbers'] if n['name']=='ItemLevel')
    click(ilvl['rect']);wait_for(lambda:next(n for n in status()['item_filter_controls']['numbers'] if n['name']=='ItemLevel')['active']!=ilvl['active'],'ilvl toggled')
    field=next(n for n in status()['item_filter_controls']['numbers'] if n['name']=='ItemLevel')['field'];click(field)
    subprocess.run(['wtype','-M','ctrl','-k','a','-m','ctrl','--','86'],env=env,check=True)
    click(status()['search_button']);wait_for(lambda:status()['request'] is not None,'ilvl query')
    assert ('ilvl' in status()['request']['query'].get('filters',{}).get('misc_filters',{}).get('filters',{})) == (not ilvl['active']),status()['request']
    assert status()['request']['query']['filters']['misc_filters']['filters']['ilvl']['min']==86
    time.sleep(5.1)
    click(next(n for n in status()['item_filter_controls']['numbers'] if n['name']=='ItemLevel')['rect'])
    report['direct_base_and_ilvl_buttons_change_trade_query_without_automatic_requests']=True
    for name in ['corruption','rarity']:
        assert status()['item_filter_controls'][name]['active']
        choose_menu(name,0)
    wait_for(lambda:status()['item_filters'].get('corrupted') is None and status()['item_filters'].get('rarity') is None,'corruption and rarity disabled')
    assert status()['request'] is None
    click(status()['search_button']);wait_for(lambda:status()['request'] is not None,'query without rarity or corruption')
    q=status()['request']['query']['filters']
    assert 'corrupted' not in q.get('misc_filters',{}).get('filters',{}) and 'rarity' not in q.get('type_filters',{}).get('filters',{}),q
    time.sleep(5.1)
    for name in ['corruption','rarity']:
        choose_menu(name,1)
    report['plain_label_dropdowns_remove_corruption_and_rarity_criteria']=True
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
    assert all(t not in offers[0]['tooltip'] for t in ['Mods','Implicites','Craftés','Quality','Prérequis']),offers[0]['tooltip']
    assert '5 Chaos Orb' in offers[0]['tooltip']
    assert status()['status_text']=='2 résultats'
    assert status()['header_text']=='Synthetic Fixture'
    pointer(status()['origin']['x']+8,1100)
    wait_for(lambda:not status()['listings']['tooltip_visible'],'clean panel capture')
    if os.environ.get('APT_PANEL_SCREENSHOT'):capture_panel(os.environ['APT_PANEL_SCREENSHOT'])
    assert offers[0]['rect']['y']<1000,'offers are still compressed at the bottom'
    click(offers[0]['rect']);time.sleep(.15);assert status()['state']=='Interactive','clicking an offer closed checker'
    pointer(status()['origin']['x']+8,1100);wait_for(lambda:not status()['listings']['tooltip_visible'],'leave offer before timed hover')
    report['clicking_offer_rows_keeps_checker_open']=True
    r=offers[0]['rect'];o=status()['origin'];pointer(o['x']+r['x']+r['width']-5,o['y']+r['y']+r['height']/2)
    hover_started=time.monotonic()
    wait_for(lambda:status()['listings']['tooltip_queries']>0 and status()['listings']['tooltip_visible'],'native tooltip painted at offer hover')
    report['whole_offer_right_edge_hover_ms']=round((time.monotonic()-hover_started)*1000)
    assert report['whole_offer_right_edge_hover_ms']<450,report
    assert status()['state']=='Interactive' and status()['overlay_focused'],'hover tooltip changed keyboard owner'
    time.sleep(.3);assert status()['listings']['tooltip_visible'],'hover detail flickered away'
    detail=status()['listings']['offers'][0]['detail_rect'];row=status()['listings']['offers'][0]['rect'];assert detail and detail['width']>100,detail
    assert detail['x']>=0 and detail['y']>=0 and detail['x']+detail['width']<=status()['width'] and detail['y']+detail['height']<=status()['height'],detail
    assert detail['y']+detail['height']<=row['y'] or detail['y']>=row['y']+row['height'],(detail,row)
    report['offer_preview_stays_in_window_and_does_not_cover_hovered_row']=True
    if os.environ.get('APT_HOVER_SCREENSHOT'):capture_panel(os.environ['APT_HOVER_SCREENSHOT'])
    report['native_hover_shows_fetched_offer_stats_and_preserves_parent_focus']=True
    report['first_offered_item_life']=73;report['searched_item_life']=85
    r=status()['search_button'];o=status()['origin'];pointer(o['x']+r['x']+10,o['y']+r['y']+10)
    wait_for(lambda:not status()['listings']['tooltip_visible'],'tooltip closes on leaving offer')
    r=reveal(current_life)['select'];click(dict(r,x=r['x']+r['width']-14,y=r['y']+8,width=8,height=8));wait_for(lambda:not current_life()['selected'] and status()['request'] is None,'deselect invalidates old query');assert not status()['listings']['visible']
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
    report['source_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in ['scripts/test-price-checker.py','examples/stash-fixture.rs','src/ui.rs','src/ui/stat_row.rs','src/ui/listings_panel.rs','src/ui/backdrop.rs','src/ui/style.css','src/ui/item_filters_panel.rs','examples/pointer.rs','examples/native-key.rs','src/item.rs','src/item_filters.rs','src/trade_item.rs','src/trade.rs','src/ui/settings_panel.rs','src/settings.rs','src/price_layout.rs','src/leagues.rs','src/desktop/game_geometry.rs','src/desktop/preferences.rs','src/desktop/clipboard.rs','src/desktop/xwayland_shortcuts.rs','src/hotkeys.rs','src/ui/summary_panel.rs','src/ui/roll_slider.rs','src/ui/icons.rs','src/economy.rs','src/economy/identity.rs','src/modifiers/translation.rs','src/data/current-stats.ndjson','src/data/current-items.ndjson','src/data/current-icons.json']}
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
