#!/usr/bin/env node
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const host=require('./upstream-core.cjs');
const relative='renderer/public/data/en/client_strings.js';
const strings=host.load(relative).default;
const groups=[['SHAPER_MODS','explicit-shaper'],['ELDER_MODS','explicit-elder'],['HUNTER_MODS','explicit-hunter'],['WARLORD_MODS','explicit-warlord'],['REDEEMER_MODS','explicit-redeemer'],['CRUSADER_MODS','explicit-crusader'],['DELVE_MODS','explicit-delve'],['VEILED_MODS','explicit-veiled'],['INCURSION_MODS','explicit-incursion'],['ESSENCE_MODS','explicit-essence'],['INFAMOUS_MODS','explicit-infamous']];
const rules=groups.flatMap(([key,mechanic])=>strings[key].map(name=>({name,mechanic})));
const data={source:relative,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',source_sha256:crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,relative))).digest('hex'),rules};
const output=path.join(host.root,'rust/src/data/mod-mechanics.json');
if(process.argv.includes('--check'))assert.deepEqual(JSON.parse(fs.readFileSync(output,'utf8')),data);
else fs.writeFileSync(output,JSON.stringify(data,null,2)+'\n');
console.log(`${rules.length} modifier names ${process.argv.includes('--check')?'verified':'exported'}`);
