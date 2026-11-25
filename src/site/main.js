import { availablePackages, httpsUrl, quickstartFor } from './quickstart.js';

document.documentElement.classList.add('js');

const english = document.documentElement.lang.startsWith('en');
const messages = english ? {
  copied: 'Command copied.',
  copyFallback: 'Select and copy the command with Ctrl+C or Cmd+C.',
  error: 'Error', invalidOperation: 'Invalid operation', decimal: '.',
  release: 'Versioned packages with SHA-256 checksums. Read the release notes for supported platforms and the limits of this version.',
  source: (revision) => `Source at commit ${revision}. Requires Rust/Cargo and native build tools.`,
} : {
  copied: 'Comando copiado.',
  copyFallback: 'Selecione e copie o comando com Ctrl+C ou Cmd+C.',
  error: 'Erro', invalidOperation: 'Operação inválida', decimal: ',',
  release: 'Pacotes versionados com checksum SHA-256. Consulte as notas da release para conhecer o suporte e os limites desta versão.',
  source: (revision) => `Fontes no commit ${revision}. Requer Rust/Cargo e ferramentas nativas de compilação.`,
};
const sharedUrl = (relative) => new URL(relative, import.meta.url);
const formatValue = (value) => String(value).replace('.', messages.decimal);

const menu = document.querySelector('.menu-toggle');
const navigation = document.querySelector('#navigation');
menu.addEventListener('click', () => {
  const expanded = menu.getAttribute('aria-expanded') !== 'true';
  menu.setAttribute('aria-expanded', String(expanded));
  navigation.classList.toggle('is-open', expanded);
});
navigation.addEventListener('click', (event) => {
  if (event.target.closest('a')) closeMenu();
});
function closeMenu() {
  menu.setAttribute('aria-expanded', 'false');
  navigation.classList.remove('is-open');
}
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && menu.getAttribute('aria-expanded') === 'true') {
    closeMenu();
    menu.focus();
  }
});

let selectedOS = 'windows';
let packagesAvailable = new Set();
function chooseOs(os) {
  if (!['windows', 'macos', 'linux'].includes(os)) return;
  selectedOS = os;
  const instructions = quickstartFor(os, packagesAvailable.has(os), english);
  document.querySelectorAll('[data-os]').forEach((button) => {
    button.setAttribute('aria-pressed', String(button.dataset.os === os));
  });
  document.querySelector('#os-requirements').textContent = instructions.requirements;
  document.querySelector('#first-step-title').textContent = instructions.firstTitle;
  document.querySelector('#first-step-description').textContent = instructions.firstDescription;
  document.querySelector('#build-command').textContent = instructions.firstCommand;
  document.querySelector('[data-copy="build-command"]').setAttribute('aria-label', instructions.firstCopyLabel);
  document.querySelector('#configure-command').textContent = instructions.configureCommand;
  document.querySelector('#run-description').textContent = instructions.runDescription;
  document.querySelector('#run-command').textContent = instructions.runCommand;
}
document.querySelectorAll('[data-os], [data-choose-os]').forEach((button) => {
  button.addEventListener('click', () => chooseOs(button.dataset.os || button.dataset.chooseOs));
});

let copyTimer;
document.querySelectorAll('[data-copy]').forEach((button) => {
  button.addEventListener('click', async () => {
    const content = document.getElementById(button.dataset.copy);
    const status = document.querySelector('#copy-status');
    clearTimeout(copyTimer);
    try {
      await navigator.clipboard.writeText(content.textContent);
      status.textContent = messages.copied;
    } catch {
      // Selection remains a useful fallback in file previews or denied clipboard contexts.
      const range = document.createRange();
      const visibleContent = content.hidden ? button.parentElement.querySelector('code') : content;
      range.selectNodeContents(visibleContent);
      const selection = window.getSelection();
      selection.removeAllRanges();
      selection.addRange(range);
      status.textContent = messages.copyFallback;
    }
    copyTimer = setTimeout(() => { status.textContent = ''; }, 6000);
  });
});

// This is a browser-only illustration. It never runs a recipe or calls a model.
const display = document.querySelector('#calculator-display');
const equation = document.querySelector('#calculator-equation');
let value = '48';
let previous = null;
let operator = null;
let replaceValue = true;
const operatorLabels = { '+': '+', '-': '−', '*': '×', '/': '÷' };
function paint() {
  const number = Number(value);
  const rendered = value !== messages.error && value.length > 11 && Number.isFinite(number)
    ? number.toExponential(5)
    : value;
  display.textContent = formatValue(rendered);
  display.title = formatValue(value);
  display.classList.toggle('compact', rendered.length > 9);
}
function calculate(left, right, operation) {
  if (operation === '/' && right === 0) return null;
  const result = operation === '+' ? left + right : operation === '-' ? left - right : operation === '*' ? left * right : left / right;
  return Number.isFinite(result) ? Number(result.toPrecision(10)) : null;
}
function input(key) {
  if (key === 'clear') {
    value = '0'; previous = null; operator = null; replaceValue = true; equation.textContent = '\u00a0';
  } else if (/^\d$/.test(key) || key === '.') {
    if (replaceValue) { value = key === '.' ? '0.' : key; replaceValue = false; if (!operator) equation.textContent = '\u00a0'; }
    else if (key === '.' && !value.includes('.')) value += '.';
    else if (key !== '.' && value.replace(/[-.]/g, '').length < 10) value = value === '0' ? key : value + key;
  } else if (key === 'sign' && value !== messages.error) {
    value = String(-Number(value));
  } else if (key === 'percent' && value !== messages.error) {
    value = String(Number((Number(value) / 100).toPrecision(10)));
  } else if (Object.hasOwn(operatorLabels, key) && value !== messages.error) {
    if (operator && !replaceValue) {
      const result = calculate(previous, Number(value), operator);
      if (result === null) { showError(); return; }
      value = String(result);
    }
    previous = Number(value); operator = key; replaceValue = true;
    equation.textContent = `${formatValue(value)} ${operatorLabels[key]}`;
  } else if (key === '=' && operator && value !== messages.error) {
    const right = Number(value);
    const result = calculate(previous, right, operator);
    equation.textContent = `${formatValue(previous)} ${operatorLabels[operator]} ${formatValue(value)} =`;
    if (result === null) { showError(); return; }
    value = String(result); previous = null; operator = null; replaceValue = true;
  }
  paint();
}
function showError() {
  value = messages.error; previous = null; operator = null; replaceValue = true;
  equation.textContent = messages.invalidOperation; paint();
}
document.querySelectorAll('[data-key]').forEach((button) => button.addEventListener('click', () => input(button.dataset.key)));
document.querySelector('.calculator-keys').addEventListener('keydown', (event) => {
  const key = event.key === ',' ? '.' : event.key === 'Escape' ? 'clear' : event.key;
  // Enter/space retain native button activation and do not calculate a second time.
  if (/^[0-9.+*/=\-]$/.test(key) || key === 'clear') {
    event.preventDefault(); input(key);
  }
});

async function loadDownloads() {
  try {
    const response = await fetch(sharedUrl('downloads.json'));
    if (!response.ok) return;
    const manifest = await response.json();
    const releaseUrl = httpsUrl(manifest.releaseUrl);
    if (typeof manifest.version !== 'string' || !manifest.version.trim() || !releaseUrl) return;
    packagesAvailable = availablePackages(manifest);
    let available = 0;
    for (const card of document.querySelectorAll('[data-platform]')) {
      const entry = manifest.platforms?.[card.dataset.platform];
      const url = httpsUrl(entry?.url);
      const checksum = httpsUrl(entry?.checksumUrl);
      if (!packagesAvailable.has(card.dataset.platform)) continue;
      const link = card.querySelector('[data-package-link]');
      const checksumLink = card.querySelector('[data-checksum-link]');
      link.href = url; link.hidden = false;
      checksumLink.href = checksum; checksumLink.hidden = false;
      card.querySelector('.download-state').hidden = true;
      available++;
    }
    if (available) {
      document.querySelector('#release-label').textContent = manifest.version;
      document.querySelector('#release-summary').textContent = messages.release;
      const notes = document.querySelector('#release-notes');
      notes.href = releaseUrl; notes.hidden = false;
    }
  } catch {
    // Keep useful, honest static content when offline or when the manifest is invalid.
  } finally {
    // The selected tab may have changed while the catalog was downloading.
    chooseOs(selectedOS);
  }
}
async function loadSource() {
  try {
    const response = await fetch(sharedUrl('source.json'));
    if (!response.ok) return;
    const source = await response.json();
    if (!/^[a-f0-9]{40,64}$/.test(source.revision)) return;
    const sourcePath = /^downloads\/crexe-source-[a-f0-9]+\.zip$/;
    if (!sourcePath.test(source.url) || source.checksumUrl !== `${source.url}.sha256`) return;
    document.querySelector('#source-download').href = sharedUrl(source.url).href;
    document.querySelector('#source-checksum').href = sharedUrl(source.checksumUrl).href;
    document.querySelector('#source-revision').textContent = messages.source(source.revision.slice(0, 7));
  } catch { /* Build-time links remain usable without this enhancement. */ }
}
loadDownloads();
loadSource();
