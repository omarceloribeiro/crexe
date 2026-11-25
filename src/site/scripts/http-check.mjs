import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { createStaticServer, inventory, types } from './hosting.mjs';
import { distRoot } from './shared.mjs';

export async function checkHttp(root = distRoot) {
  const server = await createStaticServer(root);
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  const base = `http://127.0.0.1:${server.address().port}/`;
  const results = [];
  let linkCount = 0;
  const pages = [];
  try {
    const files = (await inventory(root)).filter((file) => file !== '_headers');
    for (let index = 0; index < files.length; index += 8) {
      await Promise.all(files.slice(index, index + 8).map(async (file) => {
        const url = new URL(file.split('/').map(encodeURIComponent).join('/'), base);
        const response = await fetch(url, { signal: AbortSignal.timeout(15000) });
        assert.equal(response.status, 200, `HTTP download failed: ${file}`);
        const body = Buffer.from(await response.arrayBuffer());
        assert(body.equals(await readFile(path.join(root, file))), `HTTP bytes differ from publication: ${file}`);
        const extension = path.extname(file).toLowerCase();
        assert.equal(response.headers.get('content-type'), types[extension] ?? 'application/octet-stream', `Wrong MIME: ${file}`);
        if (['.md', '.crexe', '.zip', '.gz', '.sha256'].includes(extension)) assert.match(response.headers.get('content-disposition') ?? '', /^attachment/, `Missing download disposition: ${file}`);
        results.push({ file, status: response.status, bytes: body.length, sha256: createHash('sha256').update(body).digest('hex'), contentType: response.headers.get('content-type'), contentDisposition: response.headers.get('content-disposition'), path: new URL(response.url).pathname });
        if (extension === '.html') pages.push({ file, url: response.url, html: body.toString('utf8') });
      }));
    }
    const targets = new Map();
    for (const page of pages) {
      const canonical = page.html.match(/<link\b[^>]*rel="canonical"[^>]*href="([^"]+)"/)?.[1];
      if (canonical) assert.equal(new URL(canonical).pathname, new URL(page.url).pathname, `Canonical must match the final served URL: ${page.file}`);
      for (const match of page.html.matchAll(/\b(?:href|src)\s*=\s*["']([^"']*)["']/g)) {
        const url = new URL(match[1].replaceAll('&amp;', '&'), page.url);
        if (url.origin !== new URL(base).origin) continue;
        linkCount++;
        const fragment = decodeURIComponent(url.hash.slice(1));
        url.hash = '';
        const key = url.href;
        if (!targets.has(key)) targets.set(key, { url, fragments: new Set() });
        if (fragment) targets.get(key).fragments.add(fragment);
      }
    }
    const allTargets = [...targets.values()];
    for (let index = 0; index < allTargets.length; index += 8) {
      await Promise.all(allTargets.slice(index, index + 8).map(async ({ url, fragments }) => {
        const response = await fetch(url, { signal: AbortSignal.timeout(15000) });
        assert.equal(response.status, 200, `Broken HTTP link: ${url.pathname}`);
        const body = await response.text();
        if (!response.headers.get('content-type')?.startsWith('text/html')) return;
        const ids = new Set([...body.matchAll(/\bid\s*=\s*["']([^"']+)["']/g)].map((match) => match[1]));
        for (const fragment of fragments) assert(ids.has(fragment), `Missing HTTP anchor ${url.pathname}#${fragment}`);
      }));
    }
    for (const route of ['not-a-real-page', 'en/library/not-a-real-publication', 'downloads/missing.zip', '_headers']) {
      const response = await fetch(new URL(route, base));
      assert.equal(response.status, 404, `Missing path must be a real 404: ${route}`);
      assert.equal(response.headers.get('content-type'), 'text/html; charset=utf-8');
      assert((await response.text()).includes('Page not found.'), 'Expected bilingual 404.');
    }
    const source = JSON.parse(await readFile(path.join(root, 'source.json'), 'utf8'));
    const result = { revision: source.revision, directory: path.resolve(root), checkedAt: new Date().toISOString(),
      htmlPages: pages.length, localLinks: linkCount, files: results.sort((a, b) => a.file.localeCompare(b.file)),
      notFoundChecks: 4, scope: 'Local HTTP preview with the site’s Cloudflare header rules and HTML redirects. Does not verify a deployed CDN or external URLs.' };
    console.log(`HTTP: ${pages.length} HTML pages, ${linkCount} local links, ${results.length} byte-identical files, MIME/download headers and 4 genuine 404s verified.`);
    return result;
  } finally {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const args = process.argv.slice(2);
  if (args.length > 2) throw new Error('Usage: node scripts/http-check.mjs [SITE_DIRECTORY] [NEW_REPORT_JSON]');
  const result = await checkHttp(args[0] ? path.resolve(args[0]) : distRoot);
  if (args[1]) await writeFile(path.resolve(args[1]), `${JSON.stringify(result, null, 2)}\n`, { flag: 'wx' });
}
