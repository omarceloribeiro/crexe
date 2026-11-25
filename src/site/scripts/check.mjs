import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, readdir, stat } from 'node:fs/promises';
import path from 'node:path';
import { build } from './build.mjs';
import { checkHttp } from './http-check.mjs';
import { canonicalPath } from './locales.mjs';
import {
  distRoot, examples, git, readJson, repoRoot, siteRoot,
  validateDownloads, within,
} from './shared.mjs';

const { documentation, publications } = await build();
for (const script of ['main.js', 'quickstart.js', 'library.js', ...(await readdir(path.join(siteRoot, 'scripts')))
  .filter((file) => file.endsWith('.mjs')).map((file) => `scripts/${file}`)]) {
  execFileSync(process.execPath, ['--check', path.join(siteRoot, script)], { stdio: 'pipe', windowsHide: true });
}
execFileSync(process.execPath, ['--test', path.join(siteRoot, 'scripts/publications.test.mjs'), path.join(siteRoot, 'scripts/quickstart.test.mjs')], { stdio: 'inherit', windowsHide: true });
const htmlPages = new Map();
const pageIds = new Map();
for (const page of ['index.html', 'en/index.html', ...publications.pages]) {
  const html = await readFile(path.join(distRoot, page), 'utf8');
  const ids = [...html.matchAll(/\bid\s*=\s*["']([^"']+)["']/g)].map((match) => match[1]);
  assert.equal(ids.length, new Set(ids).size, `Duplicate HTML IDs in ${page}.`);
  assert.equal([...html.matchAll(/<h1\b/g)].length, 1, `Expected one page title in ${page}.`);
  htmlPages.set(page, html);
  pageIds.set(path.join(distRoot, page), new Set(ids));
}
// Both editions must stay discoverable without JavaScript and retain the same
// document when changing language. Check destinations as well as link labels.
for (const [page, html] of htmlPages) {
  const language = page.startsWith('en/') ? 'en' : 'pt-BR';
  assert(html.includes(`<html lang="${language}">`), `Incorrect language in ${page}.`);
  const counterpart = page === 'index.html' ? 'en/index.html' : page === 'en/index.html' ? 'index.html'
    : page.startsWith('en/library/') ? page.replace('en/library/', 'biblioteca/') : page.replace('biblioteca/', 'en/library/');
  assert(htmlPages.has(counterpart), `Missing translated page: ${counterpart}`);
  const selector = html.match(/<div\b[^>]*data-language-switch[^>]*>([\s\S]*?)<\/div>/)?.[1];
  assert(selector, `Missing language selector in ${page}.`);
  const links = [...selector.matchAll(/<a\b([^>]+)>/g)].map((match) => Object.fromEntries(
    [...match[1].matchAll(/([\w-]+)="([^"]*)"/g)].map((attribute) => [attribute[1], attribute[2]])));
  assert.equal(links.length, 2, `Expected both languages in ${page}.`);
  for (const link of links) {
    const target = path.posix.normalize(path.posix.join(path.posix.dirname(page), link.href));
    assert.equal(target, link.hreflang === language ? page : counterpart, `Wrong language target in ${page}.`);
    assert.equal(link['aria-current'] === 'page', link.hreflang === language, `Wrong selected language in ${page}.`);
  }
  for (const target of [page, counterpart]) {
    const targetLanguage = target.startsWith('en/') ? 'en' : 'pt-BR';
    const canonical = canonicalPath(target);
    assert(html.includes(`rel="alternate" hreflang="${targetLanguage}" href="https://crexe.org/${canonical}"`), `Missing reciprocal alternate in ${page}.`);
  }
}
let localLinks = 0;

async function checkLink(raw, from = 'index.html') {
  const link = raw.replaceAll('&amp;', '&');
  if (!link || /^(?:[a-z][a-z\d+.-]*:|\/\/)/i.test(link)) return;
  const [beforeHash, fragment] = link.split('#');
  const localPath = decodeURIComponent(beforeHash.split('?')[0]);
  assert(!localPath.startsWith('/'), `Use a relative URL for folder hosting: ${raw}`);
  const filename = path.resolve(distRoot, path.dirname(from), localPath || path.basename(from));
  assert(within(distRoot, filename), `Link escapes the site: ${raw}`);
  assert((await stat(filename)).isFile(), `Missing local file in ${from}: ${raw}`);
  if (fragment && pageIds.has(filename)) {
    assert(pageIds.get(filename).has(decodeURIComponent(fragment)), `Missing anchor in ${from}: ${raw}`);
  }
  localLinks += 1;
}
for (const [page, html] of htmlPages) {
  for (const match of html.matchAll(/\b(?:href|src)\s*=\s*["']([^"']*)["']/g)) await checkLink(match[1], page);
}
for (const file of ['styles.css', 'publications.css']) {
  const css = await readFile(path.join(distRoot, file), 'utf8');
  for (const match of css.matchAll(/url\(\s*["']?([^"')]+)["']?\s*\)/g)) await checkLink(match[1].trim(), file);
}
const sitemap = await readFile(path.join(distRoot, 'sitemap.xml'), 'utf8');
const sitemapPages = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map((match) => {
  const url = new URL(match[1]);
  assert.equal(url.origin, 'https://crexe.org', 'Unexpected sitemap origin.');
  const pathname = url.pathname.slice(1);
  return pathname.endsWith('/') || !pathname ? `${pathname}index.html` : `${pathname}.html`;
});
assert.deepEqual(sitemapPages.sort(), [...htmlPages.keys()].sort(), 'Sitemap must include every public HTML page.');

for (const { from, url } of documentation.links) await checkLink(url, from);
for (const [source, destination] of [...documentation.files, ...examples]) {
  assert((await readFile(path.join(repoRoot, source))).equals(await readFile(path.join(distRoot, destination))),
    `${destination} must match its maintained repository source byte for byte.`);
}
// The public license file and every page footer must describe the same current
// engine license. Historical publication bodies retain their original wording.
const engineLicense = await readFile(path.join(distRoot, 'LICENSE.txt'), 'utf8');
assert.match(engineLicense, /^MIT License\r?$/m, 'The engine license download must be MIT.');
assert.match(engineLicense, /The above copyright notice and this permission notice shall be included in all\s+copies or substantial portions of the Software\./,
  'The MIT download must retain its copyright and permission notice condition.');
for (const [page, html] of htmlPages) {
  const footer = html.match(/<footer\b[^>]*>([\s\S]*?)<\/footer>/)?.[1];
  assert(footer, `Missing project footer in ${page}.`);
  const licenseLinks = [...footer.matchAll(/<a\b[^>]*href="([^"]*LICENSE\.txt)"[^>]*>([\s\S]*?)<\/a>/g)];
  assert.equal(licenseLinks.length, 1, `Expected one engine license link in ${page}.`);
  const [, href, label] = licenseLinks[0];
  assert.equal(path.posix.normalize(path.posix.join(path.posix.dirname(page), href)), 'LICENSE.txt',
    `The footer must link to the maintained engine license in ${page}.`);
  assert.equal(label.replace(/<[^>]*>/g, '').trim(), 'MIT', `Outdated engine license label in ${page}.`);
  assert.doesNotMatch(footer, /\bGPL(?:v?\d)?\b/i, `Outdated GPL footer in ${page}.`);
}
for (const [page, licenseUrl] of [['index.html', 'LICENSE-GENERATED'], ['en/index.html', '../LICENSE-GENERATED']]) {
  assert.doesNotMatch(htmlPages.get(page), /\bGPL(?:v?\d)?\b/i, `Outdated engine licensing in ${page}.`);
  assert(htmlPages.get(page).includes(`href="${licenseUrl}" download`),
    `The output-material license must be downloadable from ${page}.`);
}
assert((await readFile(path.join(repoRoot, 'LICENSE-GENERATED'))).equals(await readFile(path.join(distRoot, 'LICENSE-GENERATED'))),
  'The output-material license download must preserve the maintained file byte for byte.');
const originalRfc = 'docs/crexe_handoff_docs/01_RFC_CREXE_SPEC_DRAFT.md';
assert.equal(createHash('sha256').update(await readFile(path.join(distRoot, originalRfc))).digest('hex'),
  'b889ddc77e9d156da5a656c3abefaf49c0b46a5a54c3759542f83e12421bb1f8', 'The supplied English RFC original must remain unchanged.');
assert(htmlPages.get('biblioteca/rfc-draft.html').includes(`href="../${originalRfc}" download`), 'The Portuguese RFC must offer the original English download.');

const formatOriginals = new Map([
  ['CREXP-HANDOFF-COMPLETO.md', '7d5e67b9efc273aabecaaea846f2f2aa8920b695676ed156ea6ab27ade323f4c'],
  ['CREXB-HANDOFF-COMPLETO.md', 'f2c9223620edcd26f3345983dac83b889dde7ce80e77ac7ea8d0eebde4924166'],
  ['crexb-manifest-v1.schema.json', 'e39c24295d83f718660aefa77fcb66a81dde125fd684f19e9d1944e8cb14c0d6'],
  ['crexb-manifest.example.json', '2da8dc6e87d26b704532123aaf74447ff80c80f91933f213f6399ccac45dee4b'],
]);
for (const [file, hash] of formatOriginals) {
  assert.equal(createHash('sha256').update(await readFile(path.join(distRoot, 'docs/formatos/originais', file))).digest('hex'),
    hash, `The supplied format reference must remain byte-identical: ${file}`);
}
for (const [directory, prefix, label] of [['biblioteca', '../', 'Baixar handoff original completo em português'], ['en/library', '../../', 'Download complete Portuguese handoff']]) {
  for (const slug of ['crexp', 'crexb']) {
    assert(htmlPages.get(`${directory}/${slug}.html`).includes(`href="${prefix}docs/formatos/originais/${slug.toUpperCase()}-HANDOFF-COMPLETO.md" download hreflang="pt-BR" data-original-download>${label}</a>`),
      `The ${slug} guide must label and download its full Portuguese original in ${directory}.`);
  }
  assert(htmlPages.has(`${directory}/roadmap-formatos.html`), 'The formats roadmap must have both language editions.');
}
const draftSchema = await readFile(path.join(distRoot, 'schemas/crexb-manifest-v1.schema.json'));
assert.equal(createHash('sha256').update(draftSchema).digest('hex'), formatOriginals.get('crexb-manifest-v1.schema.json'));
assert.equal(JSON.parse(draftSchema).$id, 'https://crexe.org/schemas/crexb-manifest-v1.schema.json');

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
console.log(`Checked ${htmlPages.size} HTML pages, sitemap, JavaScript syntax, ${localLinks} local links (${documentation.links.length} in Markdown), ${documentation.files.length} documentation files and ${examples.length} example downloads.`);
console.log(`Checked release manifest rejection cases and SHA-256/inventory of ${archivedFiles.length} committed source files.`);
await checkHttp(distRoot);
