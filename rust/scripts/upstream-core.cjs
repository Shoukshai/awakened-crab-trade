// Offline host for the upstream parser and selected filter functions. The only
// host adaptation is loading Vite's data URLs from the checked-in public assets.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const root=path.resolve(__dirname,'../..');
const ts=require(path.join(root,'renderer/node_modules/typescript'));
const modules=new Map();
const aliases={'@/parser':'renderer/src/parser/index.ts','@/assets/data':'renderer/src/assets/data/index.ts'};
function load(relative){
 if(modules.has(relative))return modules.get(relative);
 let source=fs.readFileSync(path.join(root,relative),'utf8');
 if(relative==='renderer/src/assets/data/index.ts'){
  source=source.replaceAll('import.meta.env.BASE_URL',"''")+'\nexport async function nativeTestLoad(){CLIENT_STRINGS=require("native_test_client");CLIENT_STRINGS_REF=CLIENT_STRINGS;await loadItems("en");await loadStats("en");}';
 }
 if(relative==='renderer/src/web/price-check/filters/create-stat-filters.ts' && !process.env.APT_FULL_FILTERS){
  // Execute these exact AST function bodies, avoiding UI/preset-only imports.
  const file=ts.createSourceFile(relative,source,ts.ScriptTarget.Latest,true);
  const names=new Set(['calculatedStatToFilter','filterFillMinMax','filterAdjustmentForNegate']);
  const functions=file.statements.filter(n=>ts.isFunctionDeclaration(n)&&names.has(n.name?.text)).map(n=>n.getText(file)).join('\n');
  source=`import {ItemRarity,ItemCategory} from '@/parser';
import {ModifierType,statSourcesTotal,translateStatWithRoll} from '@/parser/modifiers';
import {getPropQuality} from '@/parser/calc-q20';
import {StatBetter} from '@/assets/data';
import {FilterTag} from './interfaces';
import {decodeOils} from './pseudo/anointments';
import {percentRoll,percentRollDelta,roundRoll} from './util';\n${functions}`;
 }
 if(process.env.APT_FULL_FILTERS){
  // Execute unchanged pure function/constant AST nodes, omitting only desktop,
  // HTTP clients and reactive stores. No request is made by this host.
  const extract=(file,names)=>{
   const source=fs.readFileSync(path.join(root,file),'utf8');
   return ts.createSourceFile(file,source,ts.ScriptTarget.Latest,true).statements.filter(n=>
    (ts.isFunctionDeclaration(n)&&names.includes(n.name?.text)) ||
    (ts.isVariableStatement(n)&&n.declarationList.declarations.some(d=>names.includes(d.name.getText())))
   ).map(n=>n.getText()).join('\n');
  };
  if(relative==='renderer/src/web/price-check/filters/create-item-filters.ts')source=`import {ItemRarity,ItemCategory} from '@/parser';
import {MAGIC_ONLY_OR_UNIQUE_ITEM,CONSUMABLE_CRAFTABLE_ITEM,JEWELLERY} from '@/parser/meta';
import {getPropQuality} from '@/parser/calc-q20';
import {tradeTag} from '../trade/common';
import {ModifierType} from '@/parser/modifiers';
import {ITEM_BY_REF} from '@/assets/data';
import {CATEGORY_TO_TRADE_ID} from '../trade/pathofexile-trade';
import {PERMANENT_SC} from '../../background/Leagues';
${extract(relative,['SPECIAL_SUPPORT_GEM','createFilters','createGemFilters','t','floorToBracket','ceilToBracket'])}`;
  if(relative==='renderer/src/web/price-check/trade/common.ts')source=extract(relative,['tradeTag']);
  if(relative==='renderer/src/web/background/Leagues.ts')source=extract(relative,['PERMANENT_SC']);
  if(relative==='renderer/src/web/price-check/trade/pathofexile-trade.ts')source=`import {ItemInfluence,ItemCategory} from '@/parser';
import {INTERNAL_TRADE_IDS,FilterTag} from '../filters/interfaces';
import {setProperty as propSet} from 'dot-prop';
import {DateTime} from 'luxon';
import {stat,STAT_BY_REF_V2,pseudoStatByRef} from '@/assets/data';
import {decodeFamilyFromSource as decodeMercenarySupports,SearchMode as MercSearchMode} from '../filters/pseudo/mercenary';
import {ModifierType} from '@/parser/modifiers';
${extract(relative,['CATEGORY_TO_TRADE_ID','TOTAL_MODS_TEXT','INFLUENCE_PSEUDO_TEXT','createTradeRequest','getMinMax','weightedGroupToQuery','appendAndFilter','everyTradeIdToQuery','tradeIdToQuery','nameToQuery'])}`;
 }
 const exports={};modules.set(relative,exports);
 const js=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,esModuleInterop:true}}).outputText;
 function requireLocal(name){
  if(name==='native_test_client')return load('renderer/public/data/en/client_strings.js').default;
  let target=aliases[name];
  if(!target&&name.startsWith('@/'))target='renderer/src/'+name.slice(2)+'.ts';
  if(!target&&name.startsWith('.'))target=path.posix.normalize(path.posix.join(path.posix.dirname(relative),name))+'.ts';
  if(target){
   target=target.replace(/\.js\.ts$/,'.ts');
   if(!fs.existsSync(path.join(root,target)))target=target.replace(/\.ts$/,'/index.ts');
   return load(target);
  }
  if(['neverthrow','@sindresorhus/fnv1a','dot-prop','luxon'].includes(name))return require(path.join(root,'renderer/node_modules',name));
  throw new Error('Unexpected upstream dependency: '+name+' from '+relative);
 }
 const fetchLocal=async url=>{
  if(!String(url).startsWith('data/')||String(url).includes('..'))throw new Error('External fetch refused: '+url);
  const file=fs.readFileSync(path.join(root,'renderer/public',url));
  return {text:async()=>file.toString('utf8'),json:async()=>JSON.parse(file),arrayBuffer:async()=>file.buffer.slice(file.byteOffset,file.byteOffset+file.byteLength)};
 };
 vm.runInThisContext(`(function(exports,require,fetch){${js}\n})`,{filename:relative})(exports,requireLocal,fetchLocal);
 return exports;
}
module.exports={root,load,async init(){await load('renderer/src/assets/data/index.ts').nativeTestLoad();return load('renderer/src/parser/index.ts');}};
