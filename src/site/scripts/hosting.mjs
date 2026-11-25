import { createServer } from 'node:http';
import { readFile, realpath, stat, readdir } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { within } from './shared.mjs';

export const types = {
  '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8', '.json': 'application/json; charset=utf-8',
  '.xml': 'application/xml; charset=utf-8', '.svg': 'image/svg+xml', '.png': 'image/png',
  '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.webp': 'image/webp', '.ico': 'image/x-icon',
  '.woff': 'font/woff', '.woff2': 'font/woff2', '.md': 'text/plain; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8', '.sha256': 'text/plain; charset=utf-8',
  '.crexe': 'application/octet-stream', '.crexp': 'application/octet-stream', '.crexb': 'application/octet-stream', '.zip': 'application/zip', '.gz': 'application/gzip',
};

// The site's _headers uses only relative paths with a single greedy splat,
// the documented Cloudflare Pages subset supported by this local preview.
export function parseHeaders(text) {
  const rules = [];
  let rule;
  for (const line of text.split(/\r?\n/)) {
    if (line.length > 2000) throw new Error('Header rule line exceeds 2000 characters.');
    if (!line.trim() || line.trimStart().startsWith('#')) continue;
    if (!/^\s/.test(line)) {
      if (!line.startsWith('/') || (line.match(/\*/g) ?? []).length > 1 || line.includes(':')) throw new Error(`Unsupported header pattern: ${line}`);
      const expression = line.split('*').map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('.*');
      rule = { pattern: new RegExp(`^${expression}$`), headers: {} };
      rules.push(rule);
    } else {
      const match = line.trim().match(/^([A-Za-z][A-Za-z0-9-]*):\s*(.+)$/);
      if (!rule || !match) throw new Error(`Invalid header definition: ${line}`);
      const name = match[1].toLowerCase();
      rule.headers[name] = rule.headers[name] ? `${rule.headers[name]}, ${match[2]}` : match[2];
    }
  }
  if (rules.length > 100) throw new Error('More than 100 header rules.');
  return rules;
}

export function headersForPath(rules, pathname) {
  const headers = {};
  for (const rule of rules) {
    if (!rule.pattern.test(pathname)) continue;
    for (const [name, value] of Object.entries(rule.headers)) headers[name] = headers[name] ? `${headers[name]}, ${value}` : value;
  }
  return headers;
}

export async function inventory(root) {
  const files = [];
  async function visit(directory, relative = '') {
    for (const item of await readdir(directory, { withFileTypes: true })) {
      const name = path.posix.join(relative, item.name);
      if (item.isSymbolicLink()) throw new Error(`Symlink not permitted in publication: ${name}`);
      if (item.isDirectory()) await visit(path.join(directory, item.name), name);
      else if (item.isFile()) files.push(name);
      else throw new Error(`Unsupported publication entry: ${name}`);
    }
  }
  await visit(root);
  return files.sort();
}

export async function createStaticServer(root, { counterFetch = globalThis.fetch } = {}) {
  const canonicalRoot = await realpath(root);
  const rules = parseHeaders(await readFile(path.join(canonicalRoot, '_headers'), 'utf8'));
  const notFound = await readFile(path.join(canonicalRoot, '404.html'));
  // Exercise the exact self-contained Pages Worker shipped with this build.
  const { createDownloadCountsWorker } = await import(pathToFileURL(path.join(canonicalRoot, '_worker.js')).href);
  const counter = createDownloadCountsWorker({ fetch: counterFetch });
  const counterEnv = { ASSETS: { fetch: async () => new Response(
    await readFile(path.join(canonicalRoot, 'downloads.json')), { headers: { 'Content-Type': 'application/json' } },
  ) } };
  return createServer(async (request, response) => {
    response.setHeader('X-Content-Type-Options', 'nosniff');
    response.setHeader('Cache-Control', 'no-store');
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      response.writeHead(405, { Allow: 'GET, HEAD' }); response.end('Method not allowed'); return;
    }
    try {
      const url = new URL(request.url, 'http://127.0.0.1');
      const pathname = decodeURIComponent(url.pathname);
      if (url.pathname === '/api/download-counts') {
        const result = await counter.fetch(new Request(url, { method: request.method }), counterEnv, { waitUntil: (promise) => promise.catch(() => {}) });
        response.writeHead(result.status, Object.fromEntries(result.headers));
        response.end(request.method === 'HEAD' ? undefined : Buffer.from(await result.arrayBuffer()));
        return;
      }
      if (pathname.includes('\0') || pathname.includes('\\') || ['/_headers', '/_worker.js', '/_routes.json'].includes(pathname)) throw new Error('Invalid path');
      let filename = path.resolve(canonicalRoot, `.${pathname}`);
      if (!within(canonicalRoot, filename)) throw new Error('Invalid path');
      let info;
      try { info = await stat(filename); } catch {
        if (path.extname(filename)) throw new Error('Missing file');
        filename += '.html'; info = await stat(filename);
      }
      if (info.isDirectory()) {
        if (!pathname.endsWith('/')) {
          response.writeHead(301, { Location: `${url.pathname}/${url.search}` }); response.end(); return;
        }
        filename = path.join(filename, 'index.html');
      }
      const canonicalFile = await realpath(filename);
      if (!within(canonicalRoot, canonicalFile) || !(await stat(canonicalFile)).isFile()) throw new Error('Invalid file');
      // Match Pages' HTML canonicalization; links are tested after redirects.
      if (pathname.endsWith('.html') && pathname !== '/404.html') {
        const destination = url.pathname.endsWith('/index.html') ? url.pathname.slice(0, -10) : url.pathname.slice(0, -5);
        response.writeHead(301, { Location: `${destination}${url.search}` }); response.end(); return;
      }
      const body = await readFile(canonicalFile);
      response.writeHead(200, { 'content-type': types[path.extname(canonicalFile).toLowerCase()] ?? 'application/octet-stream',
        'content-length': body.length, ...headersForPath(rules, pathname) });
      response.end(request.method === 'HEAD' ? undefined : body);
    } catch {
      response.writeHead(404, { 'Content-Type': 'text/html; charset=utf-8', 'Content-Length': notFound.length });
      response.end(request.method === 'HEAD' ? undefined : notFound);
    }
  });
}
