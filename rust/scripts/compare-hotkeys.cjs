#!/usr/bin/env node
// Canonical names from unchanged ipc/KeyToCode.ts; X11 symbols from xorgproto.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs');
const source=host.load('ipc/KeyToCode.ts');
const headerPath=process.env.X11_KEYSYM_HEADER??'/usr/include/X11/keysymdef.h';
const header=fs.readFileSync(headerPath,'utf8');
const symbols=Object.fromEntries([...header.matchAll(/^#define XK_(\w+)\s+(0x[0-9a-fA-F]+)/gm)].map(m=>[m[1],Number(m[2])]));
const aliases={Backspace:'BackSpace',Enter:'Return',CapsLock:'Caps_Lock',Space:'space',PageUp:'Prior',PageDown:'Next',ArrowLeft:'Left',ArrowUp:'Up',ArrowRight:'Right',ArrowDown:'Down',NumpadMultiply:'KP_Multiply',NumpadAdd:'KP_Add',NumpadSubtract:'KP_Subtract',NumpadDecimal:'KP_Decimal',NumpadDivide:'KP_Divide',Semicolon:'semicolon',Equal:'equal',Comma:'comma',Minus:'minus',Period:'period',Slash:'slash',Backquote:'grave',BracketLeft:'bracketleft',Backslash:'backslash',BracketRight:'bracketright',Quote:'apostrophe'};
const inputs=[],expected=[];
for(const name of Object.keys(source.KeyToCode).filter(n=>!source.isModKey(n)))for(let flags=0;flags<8;flags++){
 const text=source.hotkeyToString([name],!!(flags&1),!!(flags&2),!!(flags&4));
 const symbol=name.length===1?name.toLowerCase():name.startsWith('Numpad')?'KP_'+name.slice(6):aliases[name]??name;
 assert(symbol in symbols,`Missing primary X11 symbol ${symbol}`);
 inputs.push(text);expected.push({text,key:name,keysym:symbols[symbol]});
}
const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/hotkeys'));
const native=cp.spawnSync(binary,{input:JSON.stringify(inputs),encoding:'utf8',maxBuffer:4*1024*1024});assert.equal(native.status,0,native.stderr);const output=JSON.parse(native.stdout);assert.equal(output.length,expected.length);
let mismatch_count=0;const failures=[];
for(let i=0;i<expected.length;i++)try{assert.deepEqual(output[i].Ok,expected[i]);}catch(e){mismatch_count++;if(failures.length<10)failures.push({input:inputs[i],expected:expected[i],actual:output[i]});}
const report={cases:inputs.length,key_names:inputs.length/8,modifier_combinations:8,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Unchanged upstream hotkeyToString names and modifier order; native X11 keysyms checked against installed xorgproto primary header. Registration, game reservations and Super extension tested separately; no Windows backend claim.',source_sha256:{'ipc/KeyToCode.ts':crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,'ipc/KeyToCode.ts'))).digest('hex')},x11_header:{path:headerPath,sha256:crypto.createHash('sha256').update(header).digest('hex')}};
if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
