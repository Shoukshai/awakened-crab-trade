"""Synthetic public-market responses shared by isolated native UI tests."""
import json,http.server,urllib.parse

def make_handler(root,env,posts,ninja_requests):
    class TradeFixture(http.server.BaseHTTPRequestHandler):
        def log_message(self,*args):pass
        def reply(self,body):
            data=json.dumps(body).encode();self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
        def do_POST(self):
            assert self.path.startswith('/api/trade/search/'),self.path
            posts.append(json.loads(self.rfile.read(int(self.headers.get('Content-Length','0')))))
            self.reply({'id':'fixtureSearch','total':2,'result':['offerA','offerB']})
        def do_GET(self):
            if self.path.startswith('/image.png?'):
                url=urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)['url'][0]
                sources=json.loads((root/'assets/fixtures/sources.json').read_text())['items']
                entry=next((x for x in sources if x['url']==url),None)
                if entry is None:self.send_error(404);return
                data=(root/'assets/fixtures'/f"{entry['fixture']}.png").read_bytes()
                self.send_response(200);self.send_header('Content-Type','image/png');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data);return
            if self.path.startswith('/poe1/api/economy/current/dense/overviews?'):
                ninja_requests.append(self.path)
                def line(name,variant,chaos):return dict(name=name,variant=variant,chaos=chaos,graph=[0,1,2,3,4,5,6])
                self.reply({'currencyOverviews':[{'type':'Currency','lines':[line('Divine Orb',None,200),line('Chaos Orb',None,1)]}], 'itemOverviews':[
                  {'type':'SkillGem','lines':[line('Impale Support','20/20c',5)]},
                  {'type':'UniqueArmour','lines':[line('Goldrim','Leather Cap',8)]},
                  {'type':'Map','lines':[line('Map (Tier 16)','T0, Gen-24',3)]},
                  {'type':'Viscera','lines':[line('Greater Severed Rhoa Head','Every second, inflict Withered on Enemies within # metres for # seconds',5)]}]});return
            assert self.path.startswith('/api/trade/fetch/'),self.path
            def offered(suffix,life):
                item={'name':f'Fixture offered ring {suffix}','typeLine':'Gold Ring','frameType':2,'ilvl':84,'properties':[{'name':'Quality','values':[['+20%',1]]}],'implicitMods':['15% increased Rarity of Items found'],'explicitMods':[f'+{life} to maximum Life','+42% to Fire Resistance'],'craftedMods':['+16% to Chaos Resistance'],'identified':True,'corrupted':suffix=='B'}
                family=env['APT_FIXTURE_ITEM']
                if family=='wand':item.update(name='Fixture wand',typeLine='Spiraled Wand',properties=[],implicitMods=['30% increased Spell Damage'],explicitMods=['+21 to Intelligence','+100 to maximum Mana'],craftedMods=[])
                if family=='gem':item.update(name='',typeLine='Impale Support',frameType=4,ilvl=1,properties=[{'name':'Level','values':[['20 (Max)',0]]},{'name':'Quality','values':[['+20%',1]]}],implicitMods=[],explicitMods=['Supported Attacks have 40% chance to Impale Enemies on Hit'],craftedMods=[],corrupted=True)
                if family=='unique':item.update(name='Goldrim',typeLine='Leather Cap',frameType=3,explicitMods=['+35% to all Elemental Resistances'])
                if family=='map':item.update(name='',typeLine='Map (Tier 16)',frameType=0,properties=[{'name':'Map Tier','values':[['16',0]]},{'name':'Quality','values':[['+20%',1]]}])
                if family=='influence':item.update(name=f'Fixture circlet {suffix}',typeLine='Hubris Circlet',properties=[{'name':'Energy Shield','values':[['100',0]]}],implicitMods=[],explicitMods=['Nearby Enemies have -9% to Chaos Resistance'],craftedMods=[])
                if family=='viscera':item.update(name='',typeLine='Greater Severed Rhoa Head',frameType=0,ilvl=1,implicitMods=['Every second, inflict Withered on Enemies within 6 metres for 5 seconds'],explicitMods=[],craftedMods=[])
                # Current Trade uses structured mods; keep one legacy row for compatibility.
                if suffix=='A':
                    for key in ['implicitMods','explicitMods','craftedMods','fracturedMods','enchantMods']:
                        if key in item:item[key]=[{'description':text,'domain':key.removesuffix('Mods'),'mods':[{'tier':'P2','magnitudes':[{'min':'1','max':'999'}]}]} for text in item[key]]
                return item
            self.reply({'result':[{'listing':{'indexed':'2026-11-17T12:00:00Z','price':{'amount':price,'currency':'chaos'},'account':{'name':f'FixtureSeller{suffix}'}},'item':offered(suffix,life)} for suffix,price,life in [('A',5,73),('B',12,110)]]})
    return TradeFixture
