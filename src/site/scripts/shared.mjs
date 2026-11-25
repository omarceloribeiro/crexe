import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const siteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const repoRoot = path.resolve(siteRoot, '../..');
export const distRoot = path.join(siteRoot, 'dist');

export const staticFiles = ['index.html', 'en/index.html', '404.html', '_headers', '_routes.json', 'styles.css', 'publications.css', 'main.js', 'quickstart.js', 'download-counts.js', 'library.js', 'downloads.json', 'robots.txt'];
export const examples = [
  ['examples/plaintext/calculadora.crexe', 'examples/calculadora.crexe'],
  ['examples/markdown/regra-de-tres.crexe', 'examples/regra-de-tres.crexe'],
  ['examples/yaml/calculator_native.crexe', 'examples/calculator_native.crexe'],
  ['examples/crexp/calculadora.crexp', 'examples/calculadora.crexp'],
];

export function git(args, options = {}) {
  return execFileSync('git', args, {
    cwd: repoRoot,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    windowsHide: true,
    ...options,
  });
}

export async function readJson(filename) {
  return JSON.parse(await readFile(filename, 'utf8'));
}

export function markdownLinks(input) {
  // Ignore fenced examples; keep inline, reference-style and HTML destinations.
  const text = input.replace(/^\s*(`{3,}|~{3,})[^\n]*\n[\s\S]*?^\s*\1\s*$/gm, '');
  return [
    ...[...text.matchAll(/\[[^\]\n]*\]\(\s*(?:<([^>]+)>|([^\s)]+))(?:\s+[^)]*)?\)/g)]
      .map((match) => match[1] ?? match[2]),
    ...[...text.matchAll(/^\s{0,3}\[[^\]\n]+\]:\s*(?:<([^>]+)>|(\S+))/gm)]
      .map((match) => match[1] ?? match[2]),
    ...[...text.matchAll(/\b(?:href|src)\s*=\s*["']([^"']+)["']/g)].map((match) => match[1]),
  ];
}

export async function collectDocumentation() {
  const tracked = new Set(git(['ls-files', '-z']).split('\0').filter(Boolean));
  // This new, explicitly authored site guide is a build input before its first
  // commit. Other discovered documentation must already be tracked by Git.
  const allowed = new Set([...tracked, 'src/site/README.md']);
  const selected = new Set([
    ...[...tracked].filter((file) => file.startsWith('docs/')),
    'README.md', 'SECURITY.md', 'CONTRIBUTING.md', 'LICENSE', 'config.example.toml',
    ...examples.map(([source]) => source),
  ]);
  const links = [];
  // The queue can grow when maintained Markdown links to another tracked file.
  // This preserves source-relative paths without exposing an entire checkout.
  for (const file of selected) {
    if (!allowed.has(file)) throw new Error(`Documentation input is not tracked: ${file}`);
    if (!file.endsWith('.md')) continue;
    for (const link of markdownLinks(await readFile(path.join(repoRoot, file), 'utf8'))) {
      if (/^(?:[a-z][a-z\d+.-]*:|\/\/)/i.test(link)) continue;
      const relative = decodeURIComponent(link.split(/[?#]/)[0]);
      if (relative.startsWith('/') || relative.includes('\\')) {
        throw new Error(`Documentation requires a repository-relative link: ${file} -> ${link}`);
      }
      const target = relative ? path.posix.normalize(path.posix.join(path.posix.dirname(file), relative)) : file;
      if (target === '..' || target.startsWith('../') || !allowed.has(target)) {
        throw new Error(`Documentation link must resolve to a tracked file: ${file} -> ${link}`);
      }
      selected.add(target);
      links.push({ from: file, url: link });
    }
  }
  return {
    files: [...selected].map((file) => [file, file]).concat([['LICENSE', 'LICENSE.txt']]),
    links,
  };
}

function requireHttps(value, field) {
  let url;
  try { url = new URL(value); } catch { throw new Error(`${field} must be an absolute HTTPS URL.`); }
  if (url.protocol !== 'https:' || url.username || url.password) {
    throw new Error(`${field} must be an HTTPS URL without credentials.`);
  }
  return url;
}

function requireGithubTag(url, version, field, asset) {
  if (url.hostname.toLowerCase() !== 'github.com') return;
  const match = url.pathname.match(asset
    ? /^\/[^/]+\/[^/]+\/releases\/download\/([^/]+)\/[^/]+$/
    : /^\/[^/]+\/[^/]+\/releases\/tag\/([^/]+)$/);
  if (!match || decodeURIComponent(match[1]) !== version) {
    throw new Error(`${field} must point to the GitHub release tag ${version}; latest/unversioned links are not allowed.`);
  }
}

export function validateDownloads(manifest) {
  if (!manifest || typeof manifest !== 'object' || !manifest.platforms) {
    throw new Error('downloads.json must declare platforms.');
  }
  if (manifest.version !== null && (typeof manifest.version !== 'string' || !manifest.version.trim())) {
    throw new Error('downloads.json version must be null or a nonempty release tag.');
  }
  let release;
  if (manifest.releaseUrl !== null) {
    release = requireHttps(manifest.releaseUrl, 'releaseUrl');
    if (!manifest.version) throw new Error('A releaseUrl requires version.');
    requireGithubTag(release, manifest.version, 'releaseUrl', false);
  }
  for (const platform of ['windows', 'linux', 'macos']) {
    const entry = manifest.platforms[platform];
    if (!entry || typeof entry !== 'object') throw new Error(`Missing platform ${platform}.`);
    if (entry.url === null && entry.checksumUrl === null) continue;
    if (!entry.url || !entry.checksumUrl || !manifest.version || !release) {
      throw new Error(`${platform}: a download requires version, releaseUrl, url and checksumUrl.`);
    }
    const url = requireHttps(entry.url, `${platform}.url`);
    const checksum = requireHttps(entry.checksumUrl, `${platform}.checksumUrl`);
    requireGithubTag(url, manifest.version, `${platform}.url`, true);
    requireGithubTag(checksum, manifest.version, `${platform}.checksumUrl`, true);
  }
  return manifest;
}

export function within(root, filename) {
  const relative = path.relative(root, filename);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}
