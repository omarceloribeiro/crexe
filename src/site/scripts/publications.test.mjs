import assert from 'node:assert/strict';
import test from 'node:test';
import { escapeHtml, renderPublication, validateCatalog } from './publications.mjs';

const current = {
  slug: 'exemplo', source: 'docs/publicacoes/exemplo.md', title: 'Texto de exemplo',
  description: 'Uma descrição.', category: 'article', status: 'Artigo', date: '2026-11-24', notice: 'Visão editorial.',
};
const reference = { ...current, slug: 'perfis', source: 'docs/PERFIS_V1.md', title: 'Perfis' };
const catalog = [current, reference];

test('raw HTML and executable URL schemes cannot become executable markup', () => {
  const result = renderPublication([
    '# Título',
    '<script>alert("script")</script>',
    '<img src=x onerror="alert(1)">',
    '[perigoso](javascript:alert%281%29)',
    '[entidade](javascript&#x3a;alert%281%29)',
    '![imagem](data:image/svg+xml;base64,PHN2Zz4=)',
    '```html\n<img src=x onerror=alert(2)>\n```',
    '[referência](https://example.org/?a=1&b=2)',
  ].join('\n\n'), current, catalog);
  assert.doesNotMatch(result.html, /<script|<img[^>]*onerror|href=["']javascript:|src=["']data:/i);
  assert.match(result.html, /&lt;script&gt;/);
  assert.match(result.html, /&lt;img src=x onerror=alert\(2\)&gt;/);
  assert.match(result.html, /href="https:\/\/example\.org\/\?a=1&amp;b=2"/);
  assert.equal(escapeHtml('<img src=x onerror="x"> &'), '&lt;img src=x onerror=&quot;x&quot;&gt; &amp;');
});

test('links keep fragments and resolve from the original Markdown source', () => {
  const result = renderPublication([
    '[aqui](#secao)', '[catálogo](../PERFIS_V1.md#verificacao-automatizada)',
    '[arquivo](../../README.md)', '[esta página](exemplo.md?mode=read#secao)',
    '![captura](../validation/example%20image.png)',
  ].join('\n\n'), current, catalog);
  assert.match(result.html, /href="#secao"/);
  assert.match(result.html, /href="perfis\.html#verificacao-automatizada"/);
  assert.match(result.html, /href="\.\.\/README\.md"/);
  assert.match(result.html, /href="exemplo\.html\?mode=read#secao"/);
  assert.match(result.html, /src="\.\.\/docs\/validation\/example%20image\.png"/);
  assert.equal(result.localLinks.length, 4);
  assert.throws(() => renderPublication('[fora](../../../private.md)', current, catalog), /escapes repository/);
  assert.throws(() => renderPublication('[fora](..%2f..%2f..%2fprivate.md)', current, catalog), /escapes repository/);
});

test('the first source H1 is replaced by page metadata and heading IDs remain unique', () => {
  const result = renderPublication('# Título original\n\n## Ação e criação\n\nTexto.\n\n### Ação e criação\n\n## Ação e criação\n\n## Artigo', current, catalog);
  assert.doesNotMatch(result.html, /<h1|Título original/);
  assert.deepEqual(result.toc.map(({ id, level }) => ({ id, level })), [
    { id: 'acao-e-criacao', level: 2 }, { id: 'acao-e-criacao-2', level: 3 },
    { id: 'acao-e-criacao-3', level: 2 }, { id: 'artigo-2', level: 2 },
  ]);
  assert(result.wordCount > 0);
  assert.equal(result.readTime, 1);
});

test('tables retain semantics inside a keyboard-accessible scroll region', () => {
  const result = renderPublication('| Nome | Valor |\n| --- | --- |\n| Exemplo | 1 |', current, catalog);
  assert.match(result.html, /role="region" aria-label="Tabela com rolagem horizontal" tabindex="0"><table>/);
  assert.match(result.html, /<th>Nome<\/th>/);
  assert.match(result.html, /<td>Exemplo<\/td>/);
});

test('only a recognized opening status quote is replaced by page metadata', () => {
  const result = renderPublication('# Título\n\n> Artigo • visão editorial • 24/11/2026\n\nTexto.\n\n> Uma citação importante.', current, catalog);
  assert.doesNotMatch(result.html, /visão editorial/);
  assert.match(result.html, /<blockquote>\n<p>Uma citação importante\.<\/p>/);
  const substantive = renderPublication('# Título\n\n> Em 24/11/2026, uma afirmação substantiva.', current, catalog);
  assert.match(substantive.html, /<blockquote>/);
});

test('catalog rejects unsafe filenames, duplicate destinations and invalid dates', () => {
  assert.equal(validateCatalog(catalog).length, 2);
  assert.throws(() => validateCatalog([{ ...current, slug: '../escape' }]));
  assert.throws(() => validateCatalog([{ ...current, source: '../../private.md' }]));
  assert.throws(() => validateCatalog([{ ...current, date: '2026-02-30' }]));
  assert.throws(() => validateCatalog([current, { ...reference, slug: current.slug }]));
  assert.throws(() => validateCatalog([current, { ...reference, source: current.source }]));
});
