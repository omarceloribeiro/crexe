import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { distRoot, git, repoRoot, siteRoot, within } from './shared.mjs';
import { inventory } from './hosting.mjs';
import { checkHttp } from './http-check.mjs';

const [destination, reportDirectory] = process.argv.slice(2);
if (!destination || !reportDirectory || process.argv.length !== 4) throw new Error('Usage: node scripts/package.mjs NEW_OR_EMPTY_SITE_DIRECTORY OPERATION_DIRECTORY');
const output = path.resolve(destination);
const reports = path.resolve(reportDirectory);
if (within(repoRoot, output) || within(repoRoot, reports) || within(output, reports) || within(reports, output)) throw new Error('Site and operation directories must be separate and outside the checkout.');
if (git(['status', '--porcelain', '--untracked-files=all']).trim()) throw new Error('Use a clean, committed checkout to prepare a source-linked publication.');
await mkdir(output, { recursive: true });
if ((await readdir(output)).length) throw new Error('Publication output is not empty. Refusing to overwrite.');
await mkdir(reports, { recursive: true });
execFileSync(process.execPath, [path.join(siteRoot, 'scripts/check.mjs')], { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
await cp(distRoot, output, { recursive: true, force: false, errorOnExist: true });
const records = [];
for (const file of await inventory(output)) {
  const contents = await readFile(path.join(output, file));
  if (contents.length > 25 * 1024 * 1024) throw new Error(`Asset exceeds 25 MiB: ${file}`);
  records.push({ file, bytes: contents.length, sha256: createHash('sha256').update(contents).digest('hex') });
}
if (records.length > 20000) throw new Error('Publication exceeds the 20,000-file Pages Free limit.');
const revision = git(['rev-parse', 'HEAD']).trim();
await writeFile(path.join(reports, 'SITE-INVENTORY.json'), `${JSON.stringify({ revision, output, files: records, totalBytes: records.reduce((total, file) => total + file.bytes, 0), maxFileBytes: Math.max(...records.map((file) => file.bytes)) }, null, 2)}\n`, { flag: 'wx' });
const http = await checkHttp(output);
await writeFile(path.join(reports, 'SITE-HTTP.json'), `${JSON.stringify(http, null, 2)}\n`, { flag: 'wx' });
console.log(`Publication ready: ${output}\nRevision: ${revision}\nReports: ${reports}`);
