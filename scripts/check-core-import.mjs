import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';

const importer=resolve('scripts/import-core.mjs');
function fixture(run) {
  const root=mkdtempSync(join(tmpdir(),'diva-artifact-check-'));
  try {
    const artifact=join(root,'core');mkdirSync(join(artifact,'licenses'),{recursive:true});
    const files={};
    for(const name of ['diva-core-x86_64-pc-windows-msvc.exe','protocol.ts','examples.json','version.json','recipe.yaml','Cargo.lock','licenses/dependencies.json']){
      const bytes=Buffer.from(`synthetic ${name}`);writeFileSync(join(artifact,name),bytes);files[name]=createHash('sha256').update(bytes).digest('hex');
    }
    const metadata={schema:1,protocol_version:1,target:'x86_64-pc-windows-msvc',rig_commit:'810d7dfd783c59f309892bbfeb1d73dbf65d51b6',backend_commit:'synthetic',files};
    const save=()=>writeFileSync(join(artifact,'metadata.json'),JSON.stringify(metadata));save();
    const invoke=()=>spawnSync(process.execPath,[importer,artifact],{cwd:root,encoding:'utf8'});
    run({root,artifact,metadata,save,invoke});
  } finally {rmSync(root,{recursive:true,force:true});}
}
test('imports exact bytes only after verifying the complete artifact',()=>fixture(({root,artifact,invoke})=>{
  const result=invoke();assert.equal(result.status,0,result.stderr);
  assert.deepEqual(readFileSync(join(root,'src/generated/protocol.ts')),readFileSync(join(artifact,'protocol.ts')));
}));
for(const mode of ['protocol','checksum','missing_notice','traversal'])test(`rejects ${mode} before changing desktop files`,()=>fixture(({root,artifact,metadata,save,invoke})=>{
  if(mode==='protocol')metadata.protocol_version=999;
  if(mode==='checksum')writeFileSync(join(artifact,'protocol.ts'),'tampered');
  if(mode==='missing_notice')delete metadata.files['licenses/dependencies.json'];
  if(mode==='traversal')metadata.files['../escape']='bad';
  save();assert.notEqual(invoke().status,0);assert.equal(existsSync(join(root,'src-tauri/binaries')),false);
}));
