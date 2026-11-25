import { availablePackages } from './quickstart.js';

const platforms = ['windows', 'linux', 'macos'];
const validCount = (value) => Number.isSafeInteger(value) && value >= 0;

export function verifiedCounts(data, manifest) {
  if (data?.schemaVersion !== 1 || data.status !== 'available'
    || data.version !== manifest.version || data.releaseUrl !== manifest.releaseUrl
    || !validCount(data.total) || typeof data.updatedAt !== 'string'
    || !Number.isFinite(Date.parse(data.updatedAt))) return null;
  const enabled = availablePackages(manifest);
  if (!enabled.size) return null;
  let sum = 0;
  for (const platform of platforms) {
    const count = data.platforms?.[platform];
    if (enabled.has(platform)) {
      if (!validCount(count)) return null;
      sum += count;
    } else if (count !== null) return null;
  }
  return Number.isSafeInteger(sum) && sum === data.total ? data : null;
}

export async function loadDownloadCounts(manifest, {
  root = document, fetchImpl = globalThis.fetch,
} = {}) {
  const summary = root.querySelector('#download-count-summary');
  if (!summary) return;
  const english = root.documentElement.lang.startsWith('en');
  const locale = english ? 'en' : 'pt-BR';
  const status = root.querySelector('#download-count-status');
  const total = root.querySelector('#download-count-total');
  const version = root.querySelector('#download-count-version');
  const cards = platforms.map((platform) => root.querySelector(`[data-download-count="${platform}"]`));
  const unavailable = english
    ? 'Count temporarily unavailable. Downloads remain available.'
    : 'Contagem temporariamente indisponível. Os downloads continuam disponíveis.';
  total.textContent = '—';
  version.textContent = '';
  cards.forEach((card) => { if (card) card.hidden = true; });
  summary.setAttribute('aria-busy', 'false');
  if (!availablePackages(manifest).size) {
    status.textContent = english ? 'Counts will be available after the first public release.'
      : 'Contagem disponível após a primeira release pública.';
    return;
  }
  version.textContent = ` · ${manifest.version}`;
  status.textContent = english ? 'Loading download counts…' : 'Carregando contagem de downloads…';
  summary.setAttribute('aria-busy', 'true');
  try {
    const response = await fetchImpl(new URL('api/download-counts', import.meta.url), {
      credentials: 'omit', referrerPolicy: 'no-referrer', signal: AbortSignal.timeout(8000),
    });
    if (!response.ok) throw new Error('Counter unavailable');
    const counts = verifiedCounts(await response.json(), manifest);
    if (!counts) throw new Error('Counter does not match the displayed release');
    const number = new Intl.NumberFormat(locale);
    total.textContent = number.format(counts.total);
    for (let index = 0; index < platforms.length; index++) {
      const card = cards[index];
      const count = counts.platforms[platforms[index]];
      if (card && count !== null) {
        card.textContent = `${number.format(count)} downloads`;
        card.hidden = false;
      }
    }
    const date = new Intl.DateTimeFormat(locale, { dateStyle: 'short', timeStyle: 'short' })
      .format(new Date(counts.updatedAt));
    status.textContent = english ? `Updated ${date}. Counts are cached for about 15 minutes.`
      : `Atualizado em ${date}. A contagem usa cache de cerca de 15 minutos.`;
  } catch {
    status.textContent = unavailable;
  } finally {
    summary.setAttribute('aria-busy', 'false');
  }
}
