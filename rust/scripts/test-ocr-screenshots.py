#!/usr/bin/env python3
"""Recognize the supplied PoE tooltip pixels using the packaged Windows app."""
import fcntl, hashlib, json, os, pathlib, subprocess, sys, time
r=pathlib.Path(__file__).resolve().parents[1]
s=json.loads((r/'dist/windows-sandbox.json').read_text());root=pathlib.Path(s['root'])
assert root.name.startswith('apt-windows-desktop-') and (root/'fixture.guard').is_file()
env=os.environ.copy();env.update(s['env']);assert env['DISPLAY']!=os.environ.get('DISPLAY')
assert pathlib.Path(env['WINEPREFIX']).parent==root
lock=(root/'test.lock').open('w');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
app=pathlib.Path(sys.argv[1]).resolve();assert app.is_file()
wp=lambda p:'Z:'+str(p.resolve()).replace('/','\\')
rust_backend=os.environ.get('APT_FIXTURE_EXPECT_RUST')=='1'
expected_engine='PP-OCRv5 English (Rust CPU)' if rust_backend else 'PP-OCRv5 English (CPU)'
custom_model=(app.parent/'licenses/neural-ocr/custom-model.json').is_file()
if custom_model:
 expected_engine='Crab OCR fine-tuned (Rust CPU)' if rust_backend else 'Crab OCR fine-tuned (CPU)'
probe_text='ITEM LEVEL: 84' if custom_model else 'Energy Shield: 149'
report={'scope':'User-provided basic and advanced tooltips; actual bundled OCR under private Wine, no real-game input. Windows.Media.Ocr and hardware capture still require Windows validation.','cases':[]}
for case in json.loads((r/'assets/fixtures/ocr/sources.json').read_text()):
 if sys.argv[2:] and case['file'] not in sys.argv[2:]:continue
 image=r/'assets/fixtures/ocr'/case['file'];assert hashlib.sha256(image.read_bytes()).hexdigest()==case['sha256']
 # The executable receives only a complete frame, never the insertion coordinates.
 from PIL import Image,ImageDraw,ImageFont
 tip=Image.open(image).convert('RGB');canvas=Image.new('RGB',(1920,1080),(24,28,34));draw=ImageDraw.Draw(canvas)
 font_path=subprocess.check_output(['fc-match','-f','%{file}','sans-serif'],text=True).strip();font=ImageFont.truetype(font_path,18)
 for i in range(12):draw.text((720,60+i*36),'Sold: Brimstone Nail Topaz Ring - For: 200 x Chaos Orb',fill=(185,170,130),font=font)
 origin=(50,650) if case['file']=='gloves.png' else ((900,0) if case['file']=='belt-inventory-local.png' else (1300,30))
 canvas.paste(tip,origin);image=root/('full-frame-'+case['file']);canvas.save(image)
 start=time.monotonic();result=subprocess.run([s['wine'],wp(app),'ocr-image',wp(image)],env=env,capture_output=True,timeout=90)
 assert result.returncode==0,(case['file'],result.stderr.decode(errors='replace'))
 data=json.loads(result.stdout);(root/('ocr-result-'+case['file']+'.json')).write_text(json.dumps(data,indent=2)+'\n')
 assert f'text="{probe_text}"' in result.stderr.decode(errors='replace'),result.stderr.decode(errors='replace')
 assert 'ocr probe input shape=' in result.stderr.decode(errors='replace') and 'ocr probe output shape=' in result.stderr.decode(errors='replace'),result.stderr.decode(errors='replace')
 assert data['engine']==expected_engine,'Packaged neural runtime/model was not used'
 text=data['text'];item=data['item'];assert item['item_level']==case['expected_ilvl'],'Wrong or invented item level'
 if case['file']=='gloves.png':
  assert "Assassin's Mitts" in text and 'Regenerate 56.5 Life per second' in text
  assert len(item['modifiers'])>=7,text
 elif case['file']=='body-armour.png':
  assert 'Varnished Coat' in text and 'Corrupted' in text and '32% to Chaos Resistance' in text
  if custom_model:assert item['properties']['evasion']==1606,text
  assert len(item['modifiers'])>=5,text
 elif case['file']=='advanced-armour.png':
  assert item['base']=='Necrotic Armour' and item['rarity']=='Rare',text
  mods=item['modifiers'];assert len(mods)==9,text
  assert any(m['kind']=='implicit' and m['value']==10 for m in mods),text
  assert any(m['kind']=='fractured' and m['value']==108 and m['min']==101 and m['max']==110 and m['tier']==1 for m in mods),text
  assert any(m['kind']=='crafted' and m['value']==33 for m in mods),text
  assert any(m['value']==318 and m['min']==301 and m['max']==375 for m in mods),text
  for value in [38,14,74,46,37]:assert any(m['kind']=='explicit' and m['value']==value for m in mods),(value,text)
 elif case['file']=='boots-stream.png':
  assert item['base']=='Warlock Boots',text
  flat=[m for m in item['modifiers'] if m['reference']=='+# to maximum Energy Shield']
  # Discord compression may erase the final digit of +41. Accept a correct
  # reading or explicit uncertainty; never accept the physically impossible +4.
  assert not flat or all(m['value']==41 for m in flat),text
  if flat:assert item['properties']['energy_shield']==308,text
  else:assert 'Uncertain defence values:' in text and item['properties']['energy_shield'] is None,text
  assert any(m['reference']=='Has # Abyssal Sockets' and m['value']==1 for m in item['modifiers']),text
 elif case['file'] in ['leather-belt-local.png','belt-inventory-local.png']:
  assert item['base']=='Leather Belt' and item['rarity']=='Rare',text
  assert not data['unknown'],text
  mods=item['modifiers'];assert len(mods)==7,text
  assert any(m['kind']=='implicit' and m['value']==25 and 'maximum Life' in m['reference'] for m in mods),text
  for value in [456,41,40,32,29,14]:assert any(m['value']==value and m['kind']!='implicit' for m in mods),(value,text)
 elif case['file']=='paladin-gloves-local.png':
  assert item['base']=='Paladin Gloves' and item['rarity']=='Rare',text
  assert item['quality']==20,text
  assert not data['unknown'],text
  mods=item['modifiers']
  assert any(m['kind']=='implicit' and m['value']==6 and 'Cold Damage over Time' in m['reference'] for m in mods),text
  assert any(m['kind']=='implicit' and m['value']==-11 and 'Cold Exposure' in m['reference'] for m in mods),text
  for value in [32,65,10,22,25,23,20]:assert any(m['value']==value and m['kind']!='implicit' for m in mods),(value,text)
 elif case['file']=='kinetic-blast-local.png':
  assert item['base']=='Kinetic Blast' and item['rarity']=='Gem',text
  assert item['gem_level']==21 and item['quality']==20 and item['corrupted'],text
  assert not data['unknown'] and not item['modifiers'],text
 elif case['file']=='paladin-boots-local.png':
  assert item['base']=='Paladin Boots' and item['rarity']=='Rare',text
  assert item['quality']==20 and item['properties']['armour']==516 and item['properties']['energy_shield']==113,text
  assert not data['unknown'],text
  mods=item['modifiers'];assert len(mods)==9,text
  for value in [31,33]:assert any(m['kind']=='implicit' and m['value']==value for m in mods),(value,text)
  assert any(m['value']==20 and 'Aspect of the Spider' in m['reference'] for m in mods),text
  for value in [23,74,9,48,25,24]:assert any(m['value']==value and m['kind']!='implicit' for m in mods),(value,text)
 else:raise AssertionError('Uncovered OCR fixture: '+case['file'])
 report['cases'].append({'file':case['file'],'full_frame':True,'elapsed_ms':round((time.monotonic()-start)*1000),'engine':data['engine'],'diagnostics':[line for line in result.stderr.decode(errors='replace').splitlines() if line.startswith('ocr ')],'text':text,'unknown':data['unknown'],'ilvl':item['item_level'],'quality':item['quality'],'gem_level':item['gem_level'],'corrupted':item['corrupted'],'modifiers':item['modifiers']})
image=r/'assets/fixtures/ocr/boots-stream.png'
result=subprocess.run([s['wine'],wp(app),'ocr-neural',wp(image),wp(app.parent),'6'],env=env,capture_output=True,timeout=90)
assert result.returncode==0,result.stderr.decode(errors='replace')
neural=json.loads(result.stdout)
assert neural['threads']==(1 if rust_backend else 2) and 0<neural['peak_process_mib']<1024,neural
import re
assert any(re.search(r'\+41\b.*maximum.*energy shield',line['text'],re.I) for line in neural['runs'][0]['lines']),neural
report['neural_diagnostic']=neural
# Wrong scout coordinates must not silently disable neural recognition.
hints=root/'blank-neural-hints.json';hints.write_text('[[0,100,1,1]]')
recovery_frame=root/'blank-scout-recovery.png'
recovery_image=Image.new('RGB',(1920,1080),(24,28,34));recovery_image.paste(Image.open(r/'assets/fixtures/ocr/gloves.png').convert('RGB'),(50,650));recovery_image.save(recovery_frame)
recovery=subprocess.run([s['wine'],wp(app),'ocr-neural',wp(recovery_frame),wp(app.parent),'1',wp(hints)],env=env,capture_output=True,timeout=90)
assert recovery.returncode==0,recovery.stderr.decode(errors='replace')
assert 'ocr neural retry=projection' in recovery.stderr.decode(errors='replace'),recovery.stderr.decode(errors='replace')
recovered=json.loads(recovery.stdout)
assert any(re.search(r'regenerate\s+56\.5',line['text'],re.I) for line in recovered['runs'][0]['lines']),recovered
report['blank_scout_regions_recovered_by_projection']=True
(r/'dist/ocr-screenshot-report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
