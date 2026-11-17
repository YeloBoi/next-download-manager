// Run on the hosted build runner after npm ci and Cargo's locked checks.
import { execFileSync } from 'node:child_process';
import { readdir, readFile, mkdir, copyFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
const output=path.resolve('.artifacts/THIRD_PARTY_LICENSES');
await mkdir(output,{recursive:true});
const inventory=[];
async function collect(kind,name,version,directory,license,extraFile) {
  const destination=path.join(output,kind,`${name.replaceAll('/','_')}-${version}`);
  const files=(await readdir(directory,{withFileTypes:true})).filter(entry=>entry.isFile()&&/^(licen[sc]e|copying|notice|ofl|unlicense)([.-]|$)/i.test(entry.name)).map(entry=>entry.name);
  if(extraFile&&!files.includes(extraFile))files.push(extraFile);
  if(files.length)await mkdir(destination,{recursive:true});
  for(const file of files) {
    // Cargo's license-file may be nested; flatten only the output filename.
    await copyFile(path.resolve(directory,file),path.join(destination,path.basename(file)));
  }
  inventory.push({kind,name,version,license:license??null,files:files.map(file=>path.basename(file))});
}
const metadata=JSON.parse(execFileSync('cargo',['metadata','--locked','--format-version','1'],{encoding:'utf8',maxBuffer:32*1024*1024}));
for(const pkg of metadata.packages)if(!metadata.workspace_members.includes(pkg.id))await collect('rust',pkg.name,pkg.version,path.dirname(pkg.manifest_path),pkg.license,pkg.license_file);
const seen=new Set();
async function nodeModules(directory) {
  for(const entry of await readdir(directory,{withFileTypes:true})) {
    if(!entry.isDirectory()||entry.name.startsWith('.'))continue;
    const location=path.join(directory,entry.name);
    if(entry.name.startsWith('@')) { await nodeModules(location); continue; }
    const pkg=JSON.parse(await readFile(path.join(location,'package.json'),'utf8'));
    const key=`${pkg.name}@${pkg.version}`;
    if(!seen.has(key)) { seen.add(key); await collect('npm',pkg.name,pkg.version,location,pkg.license); }
    try { await nodeModules(path.join(location,'node_modules')); } catch(error) { if(error.code!=='ENOENT')throw error; }
  }
}
await nodeModules(path.resolve('node_modules'));
if(!inventory.some(pkg=>pkg.name==='@fontsource-variable/inter'&&pkg.files.length)||!inventory.some(pkg=>pkg.name==='@fontsource-variable/vazirmatn'&&pkg.files.length))throw new Error('Bundled font licenses are missing.');
await writeFile(path.join(output,'inventory.json'),JSON.stringify(inventory,null,2)+'\n');
console.log(`Preserved license metadata for ${inventory.length} third-party packages and their supplied notice files.`);
