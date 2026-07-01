/// <reference types="vite/client" />

declare module "*.md?raw" {
  const content: string;
  export default content;
}

declare module "virtual:style-doc" {
  export const html: string;
  export const toc: { level: number; text: string; slug: string }[];
}

declare module "virtual:style-toc" {
  export const toc: { level: number; text: string; slug: string }[];
}
