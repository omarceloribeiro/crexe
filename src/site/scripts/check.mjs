import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, readdir, stat } from 'node:fs/promises';
import path from 'node:path';
import { build } from './build.mjs';
import {
  distRoot, examples, git, readJson, repoRoot, siteRoot,
  validateDownloads, within,
} from './shared.mjs';

const { documentation } = await build();
for (const script of ['main.js', ...(await readdir(path.join(siteRoot, 'scripts')))
  .filter((file) => file.endsWith('.mjs')).map((file) => `scripts/${file}`)]) {
  execFileSync(process.execPath, ['--check', path.join(siteRoot, script)], { stdio: 'pipe', windowsHide: true });
}
const html = await readFile(path.join(distRoot, 'index.html'), 'utf8');
const ids = new Set([...html.matchAll(/\bid\s*=\s*["']([^"']+)["']/g)].map((match) => match[1]));
let localLinks = 0;

async function checkLink(raw, from = 'index.html') {
  const link = raw.replaceAll('&amp;', '&');
  if (!link || /^(?:[a-z][a-z\d+.-]*:|\/\/)/i.test(link)) return;
  const [beforeHash, fragment] = link.split('#');
  const localPath = decodeURIComponent(beforeHash.split('?')[0]);
  assert(!localPath.startsWith('/'), `Use a relative URL for folder hosting: ${raw}`);
  const filename = path.resolve(distRoot, path.dirname(from), localPath || path.basename(from));
  assert(within(distRoot, filename), `Link escapes the site: ${raw}`);
  assert((await stat(filename)).isFile(), `Missing local file: ${raw}`);
  if (fragment && filename === path.join(distRoot, 'index.html')) {
    assert(ids.has(decodeURIComponent(fragment)), `Missing anchor: ${raw}`);
  }
  localLinks += 1;
}
for (const match of html.matchAll(/\b(?:href|src)\s*=\s*["']([^"']*)["']/g)) await checkLink(match[1]);
const css = await readFile(path.join(distRoot, 'styles.css'), 'utf8');
for (const match of css.matchAll(/url\(\s*["']?([^"')]+)["']?\s*\)/g)) await checkLink(match[1].trim(), 'styles.css');

for (const { from, url } of documentation.links) await checkLink(url, from);
for (const [source, destination] of [...documentation.files, ...examples]) {
  assert((await readFile(path.join(repoRoot, source))).equals(await readFile(path.join(distRoot, destination))),
    `${destination} must match its maintained repository source byte for byte.`);
}

const manifest = validateDownloads(await readJson(path.join(distRoot, 'downloads.json')));
assert.deepEqual(manifest, await readJson(path.join(siteRoot, 'downloads.json')));
const readyManifest = {
  version: 'v1.0.0',
  releaseUrl: 'https://github.com/omarceloribeiro/crexe/releases/tag/v1.0.0',
  platforms: {
    windows: {
      url: 'https://github.com/omarceloribeiro/crexe/releases/download/v1.0.0/crexe.zip',
      checksumUrl: 'https://github.com/omarceloribeiro/crexe/releases/download/v1.0.0/crexe.zip.sha256',
    },
    linux: { url: null, checksumUrl: null }, macos: { url: null, checksumUrl: null },
  },
};
validateDownloads(readyManifest);
for (const mutate of [
  (value) => { value.version = null; },
  (value) => { value.platforms.windows.checksumUrl = null; },
  (value) => { value.platforms.windows.url = 'http://example.org/crexe.zip'; },
  (value) => { value.platforms.windows.url = 'https://github.com/omarceloribeiro/crexe/releases/latest/download/crexe.zip'; },
]) {
  const invalid = structuredClone(readyManifest);
  mutate(invalid);
  assert.throws(() => validateDownloads(invalid), 'An incomplete/unversioned release must not enable a download.');
}

const source = await readJson(path.join(distRoot, 'source.json'));
assert.equal(source.revision, git(['rev-parse', 'HEAD']).trim());
assert.equal(source.shortRevision, source.revision.slice(0, 12));
await checkLink(source.url);
await checkLink(source.checksumUrl);
const archive = await readFile(path.join(distRoot, source.url));
const checksum = createHash('sha256').update(archive).digest('hex');
assert.equal(await readFile(path.join(distRoot, source.checksumUrl), 'utf8'), `${checksum}  ${path.basename(source.url)}\n`);

// Read the ZIP central directory to verify its complete inventory, without
// extracting anything or trusting archive filenames as filesystem paths.
let end = archive.length - 22;
const lower = Math.max(0, archive.length - 65557);
while (end >= lower && archive.readUInt32LE(end) !== 0x06054b50) end -= 1;
assert(end >= lower, 'Missing ZIP end record.');
const entries = archive.readUInt16LE(end + 10);
let offset = archive.readUInt32LE(end + 16);
const archivedFiles = [];
const prefix = `crexe-${source.shortRevision}/`;
for (let index = 0; index < entries; index += 1) {
  assert.equal(archive.readUInt32LE(offset), 0x02014b50, 'Invalid ZIP directory.');
  const nameLength = archive.readUInt16LE(offset + 28);
  const extraLength = archive.readUInt16LE(offset + 30);
  const commentLength = archive.readUInt16LE(offset + 32);
  const name = archive.toString('utf8', offset + 46, offset + 46 + nameLength);
  assert(name.startsWith(prefix), 'Archive files must have a shared source root.');
  if (!name.endsWith('/')) archivedFiles.push(name.slice(prefix.length));
  offset += 46 + nameLength + extraLength + commentLength;
}
const committedFiles = git(['ls-tree', '-r', '-z', '--name-only', source.revision]).split('\0')
  .filter((name) => name && !name.startsWith('src/site/') && !name.startsWith('docs/archive/'));
assert.deepEqual(archivedFiles.sort(), committedFiles.sort(), 'The ZIP must contain only the selected committed sources.');
assert(archivedFiles.includes('Cargo.toml') && archivedFiles.includes('src/main.rs') && archivedFiles.includes('LICENSE'));
console.log(`Checked JavaScript syntax, ${localLinks} local links (${documentation.links.length} in Markdown), ${documentation.files.length} documentation files and ${examples.length} example downloads.`);
console.log(`Checked release manifest rejection cases and SHA-256/inventory of ${archivedFiles.length} committed source files.`);
