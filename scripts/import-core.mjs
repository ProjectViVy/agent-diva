import { readFile, mkdir, copyFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve, join } from 'node:path';

const input = process.argv[2];
if (!input) throw new Error('Usage: node scripts/import-core.mjs <explicit-core-artifact-directory>');
const root = resolve(input);
const metadata = JSON.parse(await readFile(join(root, 'metadata.json'), 'utf8'));
if (metadata.schema !== 1 || metadata.protocol_version !== 1 || metadata.target !== 'x86_64-pc-windows-msvc')
  throw new Error('Incompatible core artifact');
if (metadata.rig_commit !== '810d7dfd783c59f309892bbfeb1d73dbf65d51b6') throw new Error('Unexpected Rig revision');
for (const name of ['diva-core-x86_64-pc-windows-msvc.exe', 'protocol.ts', 'examples.json', 'version.json', 'recipe.yaml', 'Cargo.lock','licenses/dependencies.json']) {
  if(!metadata.files[name])throw new Error(`Missing artifact entry: ${name}`);
}
for (const name of Object.keys(metadata.files)) {
  if(name.includes('\\')||name.startsWith('/')||name.split('/').some(p=>p==='..'||p==='.'||p.includes(':')))throw new Error('Invalid artifact path');
  const bytes = await readFile(join(root, name));
  if (createHash('sha256').update(bytes).digest('hex') !== metadata.files[name]) throw new Error(`Artifact mismatch: ${name}`);
}
await mkdir('src-tauri/binaries', { recursive: true });
await mkdir('src/generated', { recursive: true });
await copyFile(join(root, 'diva-core-x86_64-pc-windows-msvc.exe'), 'src-tauri/binaries/diva-core-x86_64-pc-windows-msvc.exe');
await copyFile(join(root, 'protocol.ts'), 'src/generated/protocol.ts');
await copyFile(join(root, 'metadata.json'), 'src-tauri/binaries/metadata.json');
for(const name of Object.keys(metadata.files).filter(n=>n.startsWith('licenses/'))){
  const target=join('src-tauri/core-notices',name.slice('licenses/'.length));
  await mkdir(resolve(target,'..'),{recursive:true});await copyFile(join(root,name),target);
}
await writeFile('src/generated/core-metadata.json', JSON.stringify(metadata, null, 2) + '\n');
console.log(`Imported core ${metadata.backend_commit}`);
