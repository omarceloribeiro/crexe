import assert from 'node:assert/strict';
import test from 'node:test';
import { loadDownloadCounts, verifiedCounts } from '../download-counts.js';

const releaseUrl = 'https://github.com/omarceloribeiro/crexe/releases/tag/v1.0.0';
const manifest = {
  version: 'v1.0.0', releaseUrl,
  platforms: Object.fromEntries(['windows', 'linux', 'macos'].map((os) => [os, {
    url: `https://github.com/omarceloribeiro/crexe/releases/download/v1.0.0/${os}.zip`,
    checksumUrl: `https://github.com/omarceloribeiro/crexe/releases/download/v1.0.0/${os}.zip.sha256`,
  }])),
};
const counts = {
  schemaVersion: 1, status: 'available', version: 'v1.0.0', releaseUrl,
  total: 1234, platforms: { windows: 1000, linux: 230, macos: 4 }, updatedAt: '2026-11-25T12:00:00.000Z',
};
function documentFixture(lang = 'pt-BR') {
  const elements = new Map();
  for (const selector of ['#download-count-summary', '#download-count-total', '#download-count-status', '#download-count-version',
    ...['windows', 'linux', 'macos'].map((os) => `[data-download-count="${os}"]`)]) {
    elements.set(selector, { textContent: '', hidden: false, attributes: {},
      setAttribute(name, value) { this.attributes[name] = value; } });
  }
  return { documentElement: { lang }, querySelector: (selector) => elements.get(selector) };
}

test('renders the same verified aggregate and platform counts in Portuguese and English', async () => {
  for (const lang of ['pt-BR', 'en']) {
    const root = documentFixture(lang);
    await loadDownloadCounts(manifest, { root, fetchImpl: async (url, options) => {
      assert.match(url.pathname, /\/api\/download-counts$/);
      assert.equal(options.credentials, 'omit');
      assert.equal(options.referrerPolicy, 'no-referrer');
      assert(options.signal instanceof AbortSignal);
      return Response.json(counts);
    } });
    assert.equal(root.querySelector('#download-count-total').textContent, lang === 'en' ? '1,234' : '1.234');
    assert.equal(root.querySelector('[data-download-count="macos"]').textContent, '4 downloads');
    assert.equal(root.querySelector('[data-download-count="macos"]').hidden, false);
    assert.match(root.querySelector('#download-count-status').textContent, lang === 'en' ? /^Updated / : /^Atualizado em /);
    assert.equal(root.querySelector('#download-count-summary').attributes['aria-busy'], 'false');
  }
});

test('unpublished release never requests counts or invents a zero', async () => {
  const root = documentFixture();
  await loadDownloadCounts({ version: null, releaseUrl: null, platforms: {} }, {
    root, fetchImpl: () => assert.fail('No fetch before publication'),
  });
  assert.equal(root.querySelector('#download-count-total').textContent, '—');
  assert.match(root.querySelector('#download-count-status').textContent, /primeira release pública/);
  assert.equal(root.querySelector('[data-download-count="windows"]').hidden, true);
});

test('a real zero is displayed, and a disabled platform is excluded', async () => {
  const partial = structuredClone(manifest);
  partial.platforms.macos = { url: null, checksumUrl: null };
  const zero = { ...counts, total: 0, platforms: { windows: 0, linux: 0, macos: null } };
  const root = documentFixture();
  await loadDownloadCounts(partial, { root, fetchImpl: async () => Response.json(zero) });
  assert.equal(root.querySelector('#download-count-total').textContent, '0');
  assert.equal(root.querySelector('[data-download-count="windows"]').textContent, '0 downloads');
  assert.equal(root.querySelector('[data-download-count="macos"]').hidden, true);
});

test('rejects stale, incomplete, forged and inconsistent aggregate responses', () => {
  for (const change of [
    { schemaVersion: 2 }, { status: 'unavailable' }, { version: 'v0.9.0' },
    { releaseUrl: 'https://example.com/another-release' }, { total: -1 }, { total: 1235 },
    { total: '1234' }, { updatedAt: 'invalid' }, { updatedAt: 1 },
    { platforms: { windows: 1234, linux: null, macos: 0 } },
    { platforms: { windows: 1000, linux: 230, macos: 4.5 } },
  ]) assert.equal(verifiedCounts({ ...counts, ...change }, manifest), null);
});

test('API failures, offline and stale versions show unavailable without changing download links', async () => {
  for (const fetchImpl of [
    async () => new Response('rate limited', { status: 503 }),
    async () => { throw new Error('offline'); },
    async () => Response.json({ ...counts, version: 'v0.9.0' }),
    async () => new Response('<html>Static host 404</html>'),
  ]) {
    const root = documentFixture('en');
    await loadDownloadCounts(manifest, { root, fetchImpl });
    assert.equal(root.querySelector('#download-count-total').textContent, '—');
    assert.match(root.querySelector('#download-count-status').textContent, /temporarily unavailable/);
    assert.equal(root.querySelector('#download-count-summary').attributes['aria-busy'], 'false');
    assert.equal(root.querySelector('[data-download-count="windows"]').hidden, true);
  }
});
