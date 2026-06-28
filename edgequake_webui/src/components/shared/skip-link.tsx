'use client';

import { useTranslation } from 'react-i18next';

/**
 * Skip navigation link for accessibility.
 * Allows keyboard users to skip directly to main content.
 * Only visible when focused (sr-only by default).
 *
 * KN-01 fix: uses programmatic focus to ensure the next Tab press
 * moves INTO the main content area, not back to the skip link.
 */
export function SkipLink() {
  const { t } = useTranslation();

  const handleClick = (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    const main = document.getElementById('main-content');
    if (main) {
      main.focus();
    }
  };

  return (
    <a
      href="#main-content"
      onClick={handleClick}
      className="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-[9999] focus:px-4 focus:py-2 focus:bg-primary focus:text-primary-foreground focus:rounded-md focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
    >
      {t('common.skipToContent', 'Skip to main content')}
    </a>
  );
}

export default SkipLink;
