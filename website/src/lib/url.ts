type PagePath = "" | "demo/" | "style/";

// Builds a page URL that respects Vite's base path (`/hongdown/` on
// GitHub Pages, `/` in development).
export function page(path: PagePath, hash?: string): string {
  return import.meta.env.BASE_URL + path + (hash === undefined ? "" : `#${hash}`);
}
