/**
 * Injects the display mode init script.
 *
 * The script runs before the page paints. It reads the saved display mode
 * (e-paper or color) and sets `data-display` on the <html> element, so the
 * correct theme shows up without a color flash. E-paper is the default.
 *
 * Without JavaScript the attribute stays unset and the CSS defaults to
 * e-paper.
 */

const STORAGE_KEY = 'oink-display-mode';
const DEFAULT_MODE = 'epaper';

export default function displayModeInitPlugin() {
  return {
    name: 'oink-display-mode-init',
    injectHtmlTags() {
      const script = `
(function () {
  var mode = '${DEFAULT_MODE}';
  try {
    var saved = window.localStorage.getItem('${STORAGE_KEY}');
    if (saved === 'color' || saved === 'epaper') {
      mode = saved;
    }
  } catch (error) {
    // Storage is unavailable. Keep the default.
  }
  document.documentElement.setAttribute('data-display', mode);
})();
`;
      return {
        headTags: [{tagName: 'script', innerHTML: script}],
      };
    },
  };
}
