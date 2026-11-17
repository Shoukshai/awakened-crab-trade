#!/usr/bin/env python3
"""Build the current official Trade supplement; retain the pinned upstream rules.
Run with --download to refresh the public input snapshots. No game interaction.
"""
from html.parser import HTMLParser
import urllib.parse
import argparse,concurrent.futures,datetime,hashlib,json,pathlib,re,urllib.request
p=argparse.ArgumentParser();p.add_argument('--download',action='store_true');p.add_argument('--league',default='Covens');a=p.parse_args()
r=pathlib.Path(__file__).resolve().parents[1];cache=r/'dist/research/current';cache.mkdir(parents=True,exist_ok=True)
urls={'trade-stats':'https://www.pathofexile.com/api/trade/data/stats','trade-items':'https://www.pathofexile.com/api/trade/data/items','trade-filters':'https://www.pathofexile.com/api/trade/data/filters','poedb-autocomplete':'https://cdn.poedb.tw/json/autocomplete_us.e544844442caabb0.json'}
kinds=['SkillGem','ImbuedGem','UniqueWeapon','UniqueArmour','UniqueAccessory','UniqueFlask','UniqueJewel','UniqueTincture','UniqueRelic','Map','BlightedMap','BlightRavagedMap','UniqueMap','Viscera','Wombgift','BaseType']
urls.update({f'ninja-{kind}':f'https://poe.ninja/poe1/api/economy/stash/current/item/overview?league={urllib.parse.quote(a.league)}&type={kind}' for kind in kinds})
urls.update({f'ninja-{kind}':f'https://poe.ninja/poe1/api/economy/exchange/current/overview?league={urllib.parse.quote(a.league)}&type={kind}' for kind in ['Currency','Fragment']})
def fetch(kv):
 key,url=kv;file=cache/(key+'.json')
 if a.download or not file.exists():
  req=urllib.request.Request(url,headers={'User-Agent':'Awakened-Crab-Trade'})
  with urllib.request.urlopen(req,timeout=25) as response:data=response.read(32*1024*1024+1)
  if len(data)>32*1024*1024:raise RuntimeError('response too large: '+key)
  json.loads(data);file.write_bytes(data)
 return key,json.loads(file.read_text())
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:inputs=dict(pool.map(fetch,urls.items()))
original=[json.loads(x) for x in (r/'src/data/base-stats.ndjson').read_text().splitlines()]
known_ids={id.split('|')[0] for row in original for stat in row.get('stats',[row]) for ids in stat['trade']['ids'].values() for id in ids}
known_pairs={(matcher.get('advanced',matcher['string']),kind) for row in original for stat in row.get('stats',[row]) for matcher in stat['matchers'] for kind in stat['trade']['ids']}
records={};official={}
for group in inputs['trade-stats']['result']:
 for entry in group['entries']:
  official[entry['id']]=entry
  text=entry['text'];kind=entry['type'];option=entry.get('option',{}).get('options')
  if (text,kind) in known_pairs and entry['id'] in known_ids:continue
  record=records.setdefault((text,bool(option)),{'ref':text,'better':0,'matchers':[],'trade':{'ids':{},'option':bool(option)}})
  ids=record['trade']['ids'].setdefault(kind,[])
  if entry['id'] not in ids:ids.append(entry['id'])
  matchers=[{'string':text.replace('#',str(opt['text']),1).replace('+#','#'),'value':int(opt['id'])} for opt in option if str(opt['id']).lstrip('-').isdigit()] if option else [{'string':text.replace('+#','#')}]
  for matcher in matchers:
   if matcher not in record['matchers']:record['matchers'].append(matcher)
# Every current ID must be represented, while old disambiguation is preserved.
covered=known_ids | {id for record in records.values() for ids in record['trade']['ids'].values() for id in ids}
assert not set(official)-covered
out=r/'src/data';(out/'current-stats.ndjson').write_text(''.join(json.dumps(x,ensure_ascii=False,separators=(',',':'))+'\n' for x in records.values()))
old=[json.loads(x) for x in (r/'src/data/base-items.ndjson').read_text().splitlines()];seen={(x['namespace'],x['name'],x.get('unique',{}).get('base')) for x in old}
class_by_name={v['label']:v['desc'] for v in inputs['poedb-autocomplete']};icons={};ninja={}
for kind in kinds:
 for v in inputs['ninja-'+kind].get('lines',[]):
  if v.get('icon'):icons.setdefault(v['name'],v['icon'])
  ninja.setdefault(v['name'],v)
for kind in ['Currency','Fragment']:
 for v in inputs['ninja-'+kind].get('items',[]):
  if v.get('image'):icons[v['name']]='https://web.poecdn.com'+v['image'] if v['image'].startswith('/') else v['image']
  ninja.setdefault(v['name'],v)
alias={'One Hand Axes':'One-Handed Axe','One Hand Maces':'One-Handed Mace','One Hand Swords':'One-Handed Sword','Two Hand Axes':'Two-Handed Axe','Two Hand Maces':'Two-Handed Mace','Two Hand Swords':'Two-Handed Sword','Body Armours':'Body Armour','Boots':'Boots','Gloves':'Gloves','Helmets':'Helmet','Shields':'Shield','Wands':'Wand','Bows':'Bow','Claws':'Claw','Daggers':'Dagger','Rune Daggers':'Rune Dagger','Staves':'Staff','Warstaves':'Warstaff','Sceptres':'Sceptre','Rings':'Ring','Amulets':'Amulet','Belts':'Belt','Viscera':'Viscera','Wombgifts':'Wombgift','Quivers':'Quiver'}
fixed={'viscera':'Viscera','wombgift':'Wombgift','chart':'Chart','graft':'Graft','idol':'Idol','corpse':'Corpse','map':'Map','card':'Divination Card','currency':'Currency','gem':'Gem'}
class VisceraSlots(HTMLParser):
 def __init__(self):super().__init__();self.row=[];self.cell=None;self.slots={}
 def handle_starttag(self,tag,attrs):
  if tag=='tr':self.row=[]
  if tag=='td':self.cell=[]
 def handle_data(self,data):
  if self.cell is not None:self.cell.append(data)
 def handle_endtag(self,tag):
  if tag=='td' and self.cell is not None:self.row.append(''.join(self.cell).strip());self.cell=None
  if tag=='tr' and len(self.row)>=2 and self.row[1] in ['Head','Torso','Upper Limb','Lower Limb']:self.slots[self.row[0]]={'Head':'Head','Torso':'Body','Upper Limb':'Arm','Lower Limb':'Leg'}[self.row[1]]
slots=VisceraSlots()
if not (cache/'wickerman.html').exists() or a.download:
 req=urllib.request.Request('https://poedb.tw/us/Wickerman',headers={'User-Agent':'Awakened-Crab-Trade'})
 with urllib.request.urlopen(req,timeout=25) as response:(cache/'wickerman.html').write_bytes(response.read(8*1024*1024))
slots.feed((cache/'wickerman.html').read_text())
craftable={'One-Handed Axe','One-Handed Mace','One-Handed Sword','Two-Handed Axe','Two-Handed Mace','Two-Handed Sword','Body Armour','Boots','Gloves','Helmet','Shield','Wand','Bow','Claw','Dagger','Rune Dagger','Staff','Warstaff','Sceptre','Ring','Amulet','Belt','Quiver'}
items=[]
for group in inputs['trade-items']['result']:
 for entry in group['entries']:
  base=entry['type'];name=entry.get('name',base);unique=entry.get('flags',{}).get('unique',False) or 'name' in entry
  namespace='UNIQUE' if unique else {'gem':'GEM','card':'DIVINATION_CARD','monster':'CAPTURED_BEAST'}.get(group['id'],'ITEM')
  key=(namespace,name,base if unique else None)
  if key in seen:continue
  category=alias.get(class_by_name.get(base,''),fixed.get(group['id']))
  v={'name':name,'refName':name,'namespace':namespace}
  if category:v['category']=category
  if category in craftable and not unique:v['craftable']={'category':category}
  if name in slots.slots:v['visceraSlot']=slots.slots[name]
  if unique:v['unique']={'base':base}
  if namespace=='GEM':v['gem']={'maxLevel':3 if name in ['Empower Support','Enlighten Support','Enhance Support'] else 20}
  if group['id']=='currency':v['exchangeable']=True
  if entry.get('disc'):v['tradeDisc']=entry['disc']
  if name in icons:v['icon']=icons[name]
  items.append(v);seen.add(key)
(out/'current-items.ndjson').write_text(''.join(json.dumps(x,ensure_ascii=False,separators=(',',':'))+'\n' for x in items))
(out/'current-icons.json').write_text(json.dumps(icons,ensure_ascii=False,separators=(',',':'))+'\n')
report={'generated_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'league':a.league,'official_stats':len(official),'supplement_stats':len(records),'all_official_stat_ids_covered':True,'additional_items':len(items),'viscera_items':sum(x.get('category')=='Viscera' for x in items),'icons':len(icons),'wickerman':{'url':'https://poedb.tw/us/Wickerman','sha256':hashlib.sha256((cache/'wickerman.html').read_bytes()).hexdigest()},'inputs':{k:{'url':url,'sha256':hashlib.sha256((cache/(k+'.json')).read_bytes()).hexdigest()} for k,url in urls.items()},'notes':['Original upstream disambiguation remains authoritative. Current raw Trade labels fill absent text/types and IDs.','Supplement-only stats use exact numeric bounds; no guessed roll direction.','Game images are © Grinding Gear Games; URLs discovered through poe.ninja/PoEDB. PoE Wiki returned HTTP 403.']}
(r/'docs/current-data.json').write_text(json.dumps(report,indent=2)+'\n');print({k:v for k,v in report.items() if k not in ['inputs','notes']})
