import React, {useCallback, useEffect, useState} from 'react';
import clsx from 'clsx';

const STORAGE_KEY = 'oink-display-mode';
const DEFAULT_MODE = 'epaper';

const MODES = [
  {
    id: 'epaper',
    label: 'E-paper',
    description: 'E-paper display mode: grayscale, high contrast, dithered art',
  },
  {
    id: 'color',
    label: 'Color',
    description: 'Color display mode: warm paper with muted red and ochre accents',
  },
];

function readDisplayMode() {
  if (typeof document === 'undefined') {
    return DEFAULT_MODE;
  }
  return document.documentElement.getAttribute('data-display') === 'color'
    ? 'color'
    : DEFAULT_MODE;
}

function EpaperIcon() {
  return (
    <svg
      aria-hidden="true"
      className="oink-display-toggle__icon"
      viewBox="0 0 20 20"
      width="18"
      height="18"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4">
      <rect x="4.5" y="2.5" width="11" height="15" rx="1.2" />
      <path strokeWidth="1.1" strokeLinecap="round" d="M7.3 6.2h5.4M7.3 9h3.4M7.3 11.8h5.4" />
    </svg>
  );
}

function ColorIcon() {
  return (
    <svg
      aria-hidden="true"
      className="oink-display-toggle__icon"
      viewBox="0 0 20 20"
      width="18"
      height="18"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4">
      <path d="M10 2.6a7.4 7.4 0 1 0 0 14.8c1 0 1.6-.7 1.6-1.5 0-.5-.2-.8-.5-1.1-.4-.4-.6-.8-.6-1.3 0-.9.8-1.6 1.8-1.6h1.5c1.9 0 3.6-1.4 3.6-3.4C17.4 5.4 14.2 2.6 10 2.6Z" />
      <circle cx="6.6" cy="9.4" r="1" fill="currentColor" stroke="none" />
      <circle cx="10" cy="6.6" r="1" fill="currentColor" stroke="none" />
      <circle cx="13.4" cy="9.2" r="1" fill="currentColor" stroke="none" />
    </svg>
  );
}

/**
 * Switches the site between the e-paper and color display modes.
 *
 * The init script sets `data-display` on <html> before the page paints.
 * The clicked mode is saved in localStorage. The active button gets its
 * look from CSS selectors on `data-display`, so the state is correct even
 * before React hydrates. `aria-pressed` is set after mount.
 */
export default function DisplayModeToggle() {
  const [mode, setMode] = useState(DEFAULT_MODE);

  useEffect(() => {
    setMode(readDisplayMode());
  }, []);

  const selectMode = useCallback((nextMode) => {
    setMode(nextMode);
    document.documentElement.setAttribute('data-display', nextMode);
    try {
      window.localStorage.setItem(STORAGE_KEY, nextMode);
    } catch (error) {
      // Storage is unavailable (private browsing). The switch still works
      // for the current page.
    }
  }, []);

  return (
    <div
      className={clsx('navbar__item', 'oink-display-toggle')}
      role="group"
      aria-label="Display mode">
      {MODES.map((item) => (
        <button
          key={item.id}
          type="button"
          data-mode={item.id}
          className="clean-btn oink-display-toggle__button"
          title={item.description}
          aria-label={item.description}
          aria-pressed={mode === item.id}
          onClick={() => selectMode(item.id)}>
          {item.id === 'epaper' ? <EpaperIcon /> : <ColorIcon />}
          <span className="oink-display-toggle__label">{item.label}</span>
        </button>
      ))}
    </div>
  );
}
