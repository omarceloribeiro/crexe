// Reading, links and the table of contents work without JavaScript.
for (const button of document.querySelectorAll('[data-print]')) {
  button.hidden = false;
  button.addEventListener('click', () => window.print());
}
