export function getInitialTheme() {
  if (typeof window === 'undefined') return 'light';

  try {
    const storedTheme = window.localStorage.getItem('glum-theme');
    if (storedTheme === 'light' || storedTheme === 'dark') {
      return storedTheme;
    }
  } catch (err) {
    // localStorage can be blocked in private contexts.
  }

  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function syncTheme(nextTheme) {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.theme = nextTheme;
  document.documentElement.style.colorScheme = nextTheme;
}

export function persistTheme(nextTheme) {
  try {
    window.localStorage.setItem('glum-theme', nextTheme);
  } catch (err) {
    // The visual theme should still change even when storage is unavailable.
  }
}
