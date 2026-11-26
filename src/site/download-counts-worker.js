// Cloudflare Pages advanced-mode Worker. Only public GitHub release metadata is
// queried; visitor headers, identifiers and credentials are never forwarded.
const REPOSITORY = 'omarceloribeiro/crexe';
const RELEASE_ROOT = `https://github.com/${REPOSITORY}/releases/`;
const API_ROOT = `https://api.github.com/repos/${REPOSITORY}/releases`;
const PLATFORMS = ['windows', 'linux', 'macos'];
const ENDPOINT = '/api/download-counts';
const SUCCESS_TTL = 900;
const ERROR_TTL = 60;

function emptyCounts(status, manifest = null) {
  return {
    schemaVersion: 1, status, version: manifest?.version ?? null,
    releaseUrl: manifest?.releaseUrl ?? null, total: null,
    platforms: Object.fromEntries(PLATFORMS.map((platform) => [platform, null])),
    updatedAt: null,
  };
}

function jsonResponse(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      'Content-Type': 'application/json; charset=utf-8',
      'X-Content-Type-Options': 'nosniff',
      'Cache-Control': 'public, max-age=60',
    },
  });
}

function exactGithubUrl(value, expectedPrefix, asset = false) {
  if (typeof value !== 'string') throw new Error('Missing release URL.');
  const url = new URL(value);
  if (url.protocol !== 'https:' || url.hostname !== 'github.com' || url.port ||
      url.username || url.password || url.search || url.hash || url.href !== value) {
    throw new Error('Only canonical GitHub release URLs are supported.');
  }
  if (asset) {
    if (!value.startsWith(expectedPrefix)) throw new Error('Wrong release asset.');
    const name = decodeURIComponent(value.slice(expectedPrefix.length));
    if (!name || /[/\\\x00-\x1f]/.test(name) || name === '.' || name === '..') {
      throw new Error('Invalid release asset name.');
    }
  } else if (value !== expectedPrefix) throw new Error('Wrong release tag.');
  return value;
}

// Runtime validation is intentionally stricter than the static download catalog:
// this endpoint can count only assets from this project's public GitHub release.
function readManifest(input) {
  if (!input || typeof input !== 'object' || !input.platforms) throw new Error('Invalid catalog.');
  const disabled = input.version === null && input.releaseUrl === null;
  if (!disabled && (typeof input.version !== 'string' || !input.version.trim() ||
      input.version !== input.version.trim() || /[\x00-\x1f]/.test(input.version))) {
    throw new Error('Invalid release version.');
  }
  const version = disabled ? null : input.version;
  const tag = encodeURIComponent(version);
  const releaseUrl = disabled ? null : exactGithubUrl(input.releaseUrl, `${RELEASE_ROOT}tag/${tag}`);
  const platforms = {};
  const seen = new Set();
  for (const platform of PLATFORMS) {
    const entry = input.platforms[platform];
    if (!entry || typeof entry !== 'object') throw new Error('Missing platform.');
    if (entry.url === null && entry.checksumUrl === null) {
      platforms[platform] = null;
      continue;
    }
    if (disabled) throw new Error('Asset requires a release.');
    const prefix = `${RELEASE_ROOT}download/${tag}/`;
    const url = exactGithubUrl(entry.url, prefix, true);
    const checksumUrl = exactGithubUrl(entry.checksumUrl, prefix, true);
    if (checksumUrl !== `${url}.sha256` || /\.(?:sha256|md|txt)$/i.test(url) || seen.has(url)) {
      throw new Error('Invalid or duplicate engine package.');
    }
    if (!/\.(?:zip|tar\.gz)$/.test(url)) throw new Error('Unsupported engine archive.');
    seen.add(url);
    platforms[platform] = { url, checksumUrl };
  }
  return { version, releaseUrl, platforms };
}

async function cacheRequest(origin, manifest) {
  const bytes = new TextEncoder().encode(JSON.stringify(manifest));
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  const hash = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
  // Neither the incoming query string nor a user-controlled header affects the
  // cache key. A catalog update always gets its own cached result.
  return new Request(`${origin}${ENDPOINT}?catalog=${hash}`);
}

async function githubJson(fetcher, url, signal) {
  const response = await fetcher(url, {
    // Workers supports manual/follow, not error. Reject 3xx below without
    // following Location, so the fixed GitHub API remains the only upstream.
    method: 'GET', redirect: 'manual', signal,
    headers: {
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      'User-Agent': 'CREXE-public-download-counter',
    },
  });
  if (!response.ok) throw new Error('Public release information is unavailable.');
  return response.json();
}

function collectCounts(assets, expected, found) {
  if (!Array.isArray(assets)) throw new Error('Invalid asset list.');
  for (const asset of assets) {
    if (!asset || !expected.has(asset.browser_download_url)) continue;
    if (asset.state !== 'uploaded' || !Number.isSafeInteger(asset.download_count) || asset.download_count < 0) {
      throw new Error('Invalid download count.');
    }
    if (found.has(asset.browser_download_url)) throw new Error('Duplicate release asset.');
    found.set(asset.browser_download_url, asset.download_count);
  }
}

async function queryCounts(manifest, fetcher, now, timeoutMs) {
  // One deadline covers the release request and all optional asset pages.
  const controller = new AbortController();
  let timer;
  const deadline = new Promise((_, reject) => {
    timer = setTimeout(() => {
      controller.abort();
      reject(new Error('GitHub request timed out.'));
    }, timeoutMs);
  });
  const query = async () => {
    const release = await githubJson(fetcher, `${API_ROOT}/tags/${encodeURIComponent(manifest.version)}`, controller.signal);
    if (!release || release.draft !== false || release.private === true || release.repository?.private === true ||
        release.tag_name !== manifest.version || release.html_url !== manifest.releaseUrl ||
        typeof release.published_at !== 'string' || !Number.isFinite(Date.parse(release.published_at)) ||
        !Number.isSafeInteger(release.id) || release.id <= 0) {
      throw new Error('The public release did not match the catalog.');
    }
    const expected = new Set(Object.values(manifest.platforms).filter(Boolean).map((entry) => entry.url));
    let found = new Map();
    collectCounts(release.assets, expected, found);
    if (found.size !== expected.size) {
      // GitHub may truncate the embedded list. Re-read complete pages using a
      // locally constructed URL; never follow response-provided URLs or links.
      found = new Map();
      for (let page = 1; page <= 10; page++) {
        const assets = await githubJson(fetcher, `${API_ROOT}/${release.id}/assets?per_page=100&page=${page}`, controller.signal);
        collectCounts(assets, expected, found);
        if (found.size === expected.size || assets.length < 100) break;
      }
    }
    if (found.size !== expected.size) throw new Error('A configured package is missing from the release.');
    const data = emptyCounts('available', manifest);
    for (const platform of PLATFORMS) {
      const entry = manifest.platforms[platform];
      if (entry) data.platforms[platform] = found.get(entry.url);
    }
    data.total = [...found.values()].reduce((sum, count) => sum + count, 0);
    if (!Number.isSafeInteger(data.total)) throw new Error('Download total exceeds integer precision.');
    data.updatedAt = new Date(now()).toISOString();
    return data;
  };
  try { return await Promise.race([query(), deadline]); }
  finally { clearTimeout(timer); }
}

export function createDownloadCountsWorker(options = {}) {
  return {
    async fetch(request, env, ctx) {
      const incoming = new URL(request.url);
      if (incoming.pathname !== ENDPOINT) return env.ASSETS.fetch(request);
      if (request.method !== 'GET' && request.method !== 'HEAD') {
        return new Response(null, { status: 405, headers: { Allow: 'GET, HEAD', 'Cache-Control': 'no-store' } });
      }
      const finish = (response) => request.method === 'HEAD'
        ? new Response(null, { status: response.status, headers: response.headers }) : response;
      let manifest;
      try {
        // Deliberately construct a fresh request with no visitor headers.
        const catalog = await env.ASSETS.fetch(new Request(`${incoming.origin}/downloads.json`));
        if (!catalog.ok) throw new Error('Catalog unavailable.');
        manifest = readManifest(await catalog.json());
      } catch { return finish(jsonResponse(emptyCounts('unavailable'), 503)); }
      if (!manifest.version || !Object.values(manifest.platforms).some(Boolean)) {
        return finish(jsonResponse(emptyCounts('unpublished', manifest)));
      }
      const cache = options.cache === undefined ? globalThis.caches?.default : options.cache;
      let key;
      try {
        key = await cacheRequest(incoming.origin, manifest);
        const cached = await cache?.match(key);
        if (cached) {
          const response = new Response(cached.body, cached);
          response.headers.set('Cache-Control', 'public, max-age=60');
          return finish(response);
        }
      } catch { /* Cache availability must not determine download availability. */ }
      let response;
      try {
        const data = await queryCounts(manifest, options.fetch ?? globalThis.fetch,
          options.now ?? Date.now, Math.min(5000, Math.max(1, options.timeoutMs ?? 5000)));
        response = jsonResponse(data);
      } catch { response = jsonResponse(emptyCounts('unavailable', manifest), 503); }
      if (cache && key) {
        const cached = response.clone();
        cached.headers.set('Cache-Control', `public, max-age=${response.ok ? SUCCESS_TTL : ERROR_TTL}`);
        const pending = Promise.resolve().then(() => cache.put(key, cached)).catch(() => {});
        if (ctx?.waitUntil) ctx.waitUntil(pending);
        else await pending;
      }
      return finish(response);
    },
  };
}

export default createDownloadCountsWorker();
