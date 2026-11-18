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
report={'scope':'User-provided basic tooltips; actual bundled OCR under private Wine, no real-game input. Windows.Media.Ocr and hardware capture still require Windows validation.','cases':[]}
for case in json.loads((r/'assets/fixtures/ocr/sources.json').read_text()):
 image=r/'assets/fixtures/ocr'/case['file'];assert hashlib.sha256(image.read_bytes()).hexdigest()==case['sha256']
 # The executable receives only a complete frame, never the insertion coordinates.
 from PIL import Image,ImageDraw,ImageFont
 tip=Image.open(image).convert('RGB');canvas=Image.new('RGB',(1920,1080),(24,28,34));draw=ImageDraw.Draw(canvas)
 font_path=subprocess.check_output(['fc-match','-f','%{file}','sans-serif'],text=True).strip();font=ImageFont.truetype(font_path,18)
 for i in range(12):draw.text((720,60+i*36),'Sold: Brimstone Nail Topaz Ring - For: 200 x Chaos Orb',fill=(185,170,130),font=font)
 origin=(50,650) if case['file']=='gloves.png' else (1300,30)
 canvas.paste(tip,origin);image=root/('full-frame-'+case['file']);canvas.save(image)
 start=time.monotonic();result=subprocess.run([s['wine'],wp(app),'ocr-image',wp(image)],env=env,capture_output=True,timeout=40)
 assert result.returncode==0,(case['file'],result.stderr.decode(errors='replace'))
 data=json.loads(result.stdout);text=data['text'];assert 'Item Level:' not in text,'Required level was mistaken for item level'
 if case['file']=='gloves.png':
  assert "Assassin's Mitts" in text and 'Regenerate 56.5 Life per second' in text
  assert text.count('{ Explicit Modifier }')>=7,text
 else:
  assert 'Varnished Coat' in text and 'Corrupted' in text and '32% to Chaos Resistance' in text
  assert text.count('{ Explicit Modifier }')>=5,text
 report['cases'].append({'file':case['file'],'full_frame':True,'elapsed_ms':round((time.monotonic()-start)*1000),'engine':data['engine'],'text':text,'unknown':data['unknown'],'ilvl':None})
(r/'dist/ocr-screenshot-report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
