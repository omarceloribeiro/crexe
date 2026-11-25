import assert from 'node:assert/strict';
import test from 'node:test';
import { createDownloadCountsWorker } from '../download-counts-worker.js';

const root = 'https://github.com/omarceloribeiro/crexe/releases/';
const api = 'https://api.github.com/repos/omarceloribeiro/crexe/releases';
const origin = 'https://crexe.example';
const checkedAt = '2026-11-25T16:00:00.000Z';
const platformNames = ['windows', 'linux', 'macos'];
function manifest(version = 'v1.0.0') {
  return {
    version, releaseUrl: `${root}tag/${version}`,
    platforms: Object.fromEntries(platformNames.map((platform) => {
      const url = `${root}download/${version}/crexe-${platform}.zip`;
      return [platform, { url, checksumUrl: `${url}.sha256` }];
    })),
  };
}
function asset(url, count = 5) {
  return { browser_download_url: url, state: 'uploaded', download_count: count };
}
function release(catalog = manifest()) {
  return {
    id: 42, draft: false, prerelease: false, tag_name: catalog.version,
    html_url: catalog.releaseUrl, published_at: '2026-11-24T00:00:00Z',
    assets: Object.values(catalog.platforms).filter((entry) => entry.url)
      .flatMap((entry, index) => [asset(entry.url, 10 + index), asset(entry.checksumUrl, 1000)]),
  };
}
function harness({ catalog = manifest(), metadata, fetcher, cache = null, timeoutMs } = {}) {
  let current = catalog;
  const requests = [];
  const assetRequests = [];
  const waits = [];
  const env = { ASSETS: { fetch: async (request) => {
    assetRequests.push(request);
    return new URL(request.url).pathname === '/downloads.json'
      ? Response.json(current) : new Response('static asset', { headers: { 'X-Static': 'yes' } });
  } } };
  const worker = createDownloadCountsWorker({
    fetch: async (...args) => {
      requests.push(args);
      return fetcher ? fetcher(...args) : Response.json(metadata ?? release(current));
    }, cache, now: () => Date.parse(checkedAt), timeoutMs,
  });
  return {
    requests, assetRequests, setCatalog: (next) => { current = next; },
    request: async (path = '/api/download-counts', init) => {
      const response = await worker.fetch(new Request(`${origin}${path}`, init), env,
        { waitUntil: (promise) => waits.push(promise) });
      await Promise.all(waits);
      return response;
    },
  };
}
function memoryCache() {
  const records = new Map();
  return {
    records,
    match: async (request) => records.get(request.url)?.clone(),
    put: async (request, response) => { records.set(request.url, response.clone()); },
  };
}

test('counts only advertised engine archives, excluding checksums and unrelated files', async () => {
  const metadata = release();
  metadata.assets.push(asset(`${root}download/v1.0.0/crexe-source.zip`, 10000));
  const app = harness({ metadata });
  const response = await app.request();
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), {
    schemaVersion: 1, status: 'available', version: 'v1.0.0',
    releaseUrl: `${root}tag/v1.0.0`, total: 33,
    platforms: { windows: 10, linux: 11, macos: 12 }, updatedAt: checkedAt,
  });
  assert.equal(app.requests.length, 1);
  assert.equal(app.requests[0][0], `${api}/tags/v1.0.0`);
  assert.equal(response.headers.get('content-type'), 'application/json; charset=utf-8');
  assert.equal(response.headers.get('x-content-type-options'), 'nosniff');
  assert.equal(response.headers.get('access-control-allow-origin'), null);
});

test('real zero stays zero; a disabled platform stays null; prereleases are supported', async () => {
  const catalog = manifest('v1.0.0-preview.1');
  catalog.platforms.macos = { url: null, checksumUrl: null };
  const metadata = release(catalog);
  metadata.prerelease = true;
  for (const entry of metadata.assets) entry.download_count = 0;
  const result = await (await harness({ catalog, metadata }).request()).json();
  assert.equal(result.status, 'available');
  assert.equal(result.total, 0);
  assert.deepEqual(result.platforms, { windows: 0, linux: 0, macos: null });
});

test('unpublished catalog does not contact GitHub or invent a zero', async () => {
  const catalog = { version: null, releaseUrl: null,
    platforms: Object.fromEntries(platformNames.map((name) => [name, { url: null, checksumUrl: null }])) };
  const app = harness({ catalog });
  const response = await app.request();
  const data = await response.json();
  assert.equal(response.status, 200);
  assert.equal(data.status, 'unpublished');
  assert.equal(data.total, null);
  assert.equal(data.updatedAt, null);
  assert.deepEqual(data.platforms, { windows: null, linux: null, macos: null });
  assert.equal(app.requests.length, 0);
});

test('configured release without packages is still unpublished', async () => {
  const catalog = manifest();
  for (const name of platformNames) catalog.platforms[name] = { url: null, checksumUrl: null };
  const app = harness({ catalog });
  assert.equal((await (await app.request()).json()).status, 'unpublished');
  assert.equal(app.requests.length, 0);
});

test('unknown, private, limited and failed requests are unavailable, never zero', async (t) => {
  for (const status of [404, 403, 429, 500]) await t.test(`HTTP ${status}`, async () => {
    const response = await harness({ fetcher: async () => new Response('upstream', { status }) }).request();
    const data = await response.json();
    assert.equal(response.status, 503);
    assert.equal(data.status, 'unavailable');
    assert.equal(data.total, null);
    assert.equal(data.updatedAt, null);
  });
  await t.test('network failure', async () => {
    const response = await harness({ fetcher: async () => { throw new Error('offline'); } }).request();
    assert.equal(response.status, 503);
  });
  await t.test('malformed JSON', async () => {
    assert.equal((await harness({ fetcher: async () => new Response('{') }).request()).status, 503);
  });
  await t.test('bounded timeout aborts the entire query', async () => {
    let signal;
    const app = harness({ timeoutMs: 5, fetcher: async (_, options) => {
      signal = options.signal;
      return new Promise(() => {});
    } });
    const response = await app.request();
    assert.equal(response.status, 503);
    assert.equal(signal.aborted, true);
  });
});

test('rejects unpublished or mismatching release metadata', async (t) => {
  const mutations = [
    (value) => { value.draft = true; },
    (value) => { value.private = true; },
    (value) => { value.repository = { private: true }; },
    (value) => { value.tag_name = 'v0.9.0'; },
    (value) => { value.html_url = `${root}tag/v0.9.0`; },
    (value) => { value.published_at = null; },
    (value) => { value.published_at = 'bad date'; },
    (value) => { value.id = '../another'; },
    (value) => { value.assets = null; },
  ];
  for (const [index, mutate] of mutations.entries()) await t.test(`invalid metadata ${index}`, async () => {
    const metadata = release(); mutate(metadata);
    assert.equal((await harness({ metadata }).request()).status, 503);
  });
});

test('rejects invalid counts, duplicate assets and totals outside safe integer precision', async (t) => {
  for (const value of [-1, 0.5, '12', null, Number.MAX_SAFE_INTEGER + 1]) {
    await t.test(`count ${value}`, async () => {
      const metadata = release(); metadata.assets[0].download_count = value;
      assert.equal((await harness({ metadata }).request()).status, 503);
    });
  }
  const duplicate = release(); duplicate.assets.push(duplicate.assets[0]);
  assert.equal((await harness({ metadata: duplicate }).request()).status, 503);
  const pending = release(); pending.assets[0].state = 'starter';
  assert.equal((await harness({ metadata: pending }).request()).status, 503);
  const overflow = release(); overflow.assets[0].download_count = Number.MAX_SAFE_INTEGER;
  assert.equal((await harness({ metadata: overflow }).request()).status, 503);
});

test('retrieves further asset pages when the embedded release list is incomplete', async () => {
  const catalog = manifest();
  const metadata = release(); metadata.assets = [];
  const first = Array.from({ length: 100 }, (_, index) => asset(`${root}download/v1.0.0/other-${index}.zip`));
  const app = harness({ fetcher: async (url) => {
    if (url === `${api}/tags/v1.0.0`) return Response.json(metadata);
    if (url === `${api}/42/assets?per_page=100&page=1`) return Response.json(first);
    assert.equal(url, `${api}/42/assets?per_page=100&page=2`);
    return Response.json(Object.values(catalog.platforms).map((entry) => asset(entry.url, 3)));
  } });
  const data = await (await app.request()).json();
  assert.equal(data.status, 'available');
  assert.equal(data.total, 9);
  assert.equal(app.requests.length, 3);
});

test('missing advertised assets produce unavailable rather than a partial total', async () => {
  const metadata = release(); metadata.assets = metadata.assets.slice(2);
  const app = harness({ fetcher: async (url) => Response.json(url.includes('/tags/') ? metadata : metadata.assets) });
  const response = await app.request();
  assert.equal(response.status, 503);
  assert.equal((await response.json()).total, null);
  assert.equal(app.requests.length, 2);
});

test('invalid or duplicate configuration never controls a GitHub request', async (t) => {
  const mutations = [
    (value) => { value.platforms.linux = value.platforms.windows; },
    (value) => { value.platforms.windows.checksumUrl = null; },
    (value) => { value.platforms.windows.checksumUrl = value.platforms.linux.checksumUrl; },
    (value) => { value.releaseUrl = 'https://other.example/'; },
    (value) => { value.releaseUrl += '?token=secret'; },
    (value) => { value.releaseUrl = value.releaseUrl.replace('github.com', 'github.com.evil.example'); },
    (value) => { value.releaseUrl = value.releaseUrl.replace('https://', 'https://user:password@'); },
    (value) => { value.platforms.windows.url = value.platforms.windows.url.replace('/crexe/', '/different/'); },
    (value) => { value.platforms.windows.url += '#fragment'; },
    (value) => { value.platforms.windows.url += '?download=1'; },
    (value) => { value.platforms.windows.url = `${root}download/v1.0.0/%2e%2e%2fsecret.zip`; },
    (value) => { value.version = null; },
    (value) => { value.version = ' v1.0.0'; },
    (value) => { delete value.platforms.macos; },
  ];
  for (const [index, mutate] of mutations.entries()) await t.test(`invalid catalog ${index}`, async () => {
    const catalog = manifest(); mutate(catalog);
    const app = harness({ catalog });
    assert.equal((await app.request()).status, 503);
    assert.equal(app.requests.length, 0);
  });
});

test('cache is shared across query strings but changes when the catalog changes', async () => {
  const cache = memoryCache();
  const app = harness({ cache });
  const first = await (await app.request('?ignored')).text().catch(() => '');
  assert.equal(first, 'static asset');
  await app.request('/api/download-counts?nonce=1');
  const cached = await app.request('/api/download-counts?nonce=2');
  assert.equal((await cached.json()).total, 33);
  assert.equal(app.requests.length, 1);
  assert.equal(cache.records.size, 1);
  const [key, response] = [...cache.records.entries()][0];
  assert.match(key, /\/api\/download-counts\?catalog=[0-9a-f]{64}$/);
  assert.equal(response.headers.get('cache-control'), 'public, max-age=900');
  assert.equal(cached.headers.get('cache-control'), 'public, max-age=60');
  app.setCatalog(manifest('v1.0.1'));
  assert.equal((await (await app.request()).json()).version, 'v1.0.1');
  assert.equal(app.requests.length, 2);
  assert.equal(cache.records.size, 2);
});

test('upstream errors are cached briefly and cache errors do not suppress fresh counts', async () => {
  const cache = memoryCache();
  const app = harness({ cache, fetcher: async () => new Response('', { status: 429 }) });
  assert.equal((await app.request()).status, 503);
  assert.equal((await app.request()).status, 503);
  assert.equal(app.requests.length, 1);
  assert.equal([...cache.records.values()][0].headers.get('cache-control'), 'public, max-age=60');
  const broken = { match: async () => { throw new Error('cache down'); }, put: async () => { throw new Error('cache full'); } };
  assert.equal((await harness({ cache: broken }).request()).status, 200);
});

test('visitor headers and credentials are never forwarded to catalog or GitHub', async () => {
  const app = harness();
  await app.request('/api/download-counts?token=visitor-secret', { headers: {
    Cookie: 'private=session', Authorization: 'Bearer visitor',
    'CF-Connecting-IP': '192.0.2.1', 'X-Forwarded-For': '192.0.2.2',
    Referer: 'https://private.example/document',
  } });
  assert.equal(app.assetRequests[0].url, `${origin}/downloads.json`);
  assert.deepEqual([...app.assetRequests[0].headers], []);
  const [url, options] = app.requests[0];
  assert.equal(new URL(url).hostname, 'api.github.com');
  assert.equal(options.redirect, 'error');
  assert.deepEqual(Object.keys(options.headers).sort(), ['Accept', 'User-Agent', 'X-GitHub-Api-Version']);
  assert.doesNotMatch(JSON.stringify(options.headers), /visitor|private|192\.0\.2/);
});

test('HEAD has no body; writes are rejected; static requests pass through unchanged', async () => {
  const app = harness();
  const head = await app.request('/api/download-counts', { method: 'HEAD' });
  assert.equal(head.status, 200);
  assert.equal(await head.text(), '');
  const post = await app.request('/api/download-counts', { method: 'POST' });
  assert.equal(post.status, 405);
  assert.equal(post.headers.get('allow'), 'GET, HEAD');
  assert.equal(app.requests.length, 1);
  const staticResponse = await app.request('/en/', { headers: { 'X-Test': 'static' } });
  assert.equal(await staticResponse.text(), 'static asset');
  assert.equal(staticResponse.headers.get('x-static'), 'yes');
  assert.equal(app.assetRequests.at(-1).headers.get('x-test'), 'static');
});
