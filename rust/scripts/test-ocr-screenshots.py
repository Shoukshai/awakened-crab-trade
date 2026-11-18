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
report={'scope':'User-provided basic and advanced tooltips; actual bundled OCR under private Wine, no real-game input. Windows.Media.Ocr and hardware capture still require Windows validation.','cases':[]}
for case in json.loads((r/'assets/fixtures/ocr/sources.json').read_text()):
 if sys.argv[2:] and case['file'] not in sys.argv[2:]:continue
 image=r/'assets/fixtures/ocr'/case['file'];assert hashlib.sha256(image.read_bytes()).hexdigest()==case['sha256']
 # The executable receives only a complete frame, never the insertion coordinates.
 from PIL import Image,ImageDraw,ImageFont
 tip=Image.open(image).convert('RGB');canvas=Image.new('RGB',(1920,1080),(24,28,34));draw=ImageDraw.Draw(canvas)
 font_path=subprocess.check_output(['fc-match','-f','%{file}','sans-serif'],text=True).strip();font=ImageFont.truetype(font_path,18)
 for i in range(12):draw.text((720,60+i*36),'Sold: Brimstone Nail Topaz Ring - For: 200 x Chaos Orb',fill=(185,170,130),font=font)
 origin=(50,650) if case['file']=='gloves.png' else (1300,30)
 canvas.paste(tip,origin);image=root/('full-frame-'+case['file']);canvas.save(image)
 start=time.monotonic();result=subprocess.run([s['wine'],wp(app),'ocr-image',wp(image)],env=env,capture_output=True,timeout=90)
 assert result.returncode==0,(case['file'],result.stderr.decode(errors='replace'))
 data=json.loads(result.stdout);(root/('ocr-result-'+case['file']+'.json')).write_text(json.dumps(data,indent=2)+'\n')
 text=data['text'];item=data['item'];assert item['item_level']==case['expected_ilvl'],'Wrong or invented item level'
 if case['file']=='gloves.png':
  assert "Assassin's Mitts" in text and 'Regenerate 56.5 Life per second' in text
  assert len(item['modifiers'])>=7,text
 elif case['file']=='body-armour.png':
  assert 'Varnished Coat' in text and 'Corrupted' in text and '32% to Chaos Resistance' in text
  assert len(item['modifiers'])>=5,text
 else:
  assert item['base']=='Necrotic Armour' and item['rarity']=='Rare',text
  mods=item['modifiers'];assert len(mods)==9,text
  assert any(m['kind']=='implicit' and m['value']==10 for m in mods),text
  assert any(m['kind']=='fractured' and m['value']==108 and m['min']==101 and m['max']==110 and m['tier']==1 for m in mods),text
  assert any(m['kind']=='crafted' and m['value']==33 for m in mods),text
  assert any(m['value']==318 and m['min']==301 and m['max']==375 for m in mods),text
  for value in [38,14,74,46,37]:assert any(m['kind']=='explicit' and m['value']==value for m in mods),(value,text)
 report['cases'].append({'file':case['file'],'full_frame':True,'elapsed_ms':round((time.monotonic()-start)*1000),'engine':data['engine'],'text':text,'unknown':data['unknown'],'ilvl':item['item_level'],'modifiers':item['modifiers']})
(r/'dist/ocr-screenshot-report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
