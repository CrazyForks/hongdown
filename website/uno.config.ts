import {
  defineConfig,
  presetIcons,
  presetTypography,
  presetUno,
} from "unocss";

// Design tokens are documented in DESIGN.md.  Change them there first,
// then here.
export default defineConfig({
  presets: [
    presetUno({
      dark: "media",
    }),
    presetTypography(),
    presetIcons(),
  ],
  theme: {
    colors: {
      // Light mode surfaces (paper) and text (ink).
      paper: {
        DEFAULT: "#FCFCFB",
        raised: "#F4F3F0",
        shade: "#ECEAE5",
      },
      ink: {
        DEFAULT: "#221E1B",
        mute: "#6E6861",
        faint: "#A39C92",
      },
      // Dark mode surfaces (night) and text (snow).
      night: {
        DEFAULT: "#141210",
        raised: "#1E1B18",
        shade: "#282420",
      },
      snow: {
        DEFAULT: "#E9E6E1",
        mute: "#A8A198",
        faint: "#6E6861",
      },
      // The accent: 홍/紅 (hong) is crimson, and a dahlia is a crimson
      // flower.  Used for links, markers, and the hero underline only.
      hong: {
        DEFAULT: "#BE3450",
        deep: "#9C2843",
        bright: "#E7768C",
        soft: "#F8E9EC",
        dusk: "#38222A",
      },
    },
    fontFamily: {
      mono: '"IBM Plex Mono", ui-monospace, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
      serif:
        '"Source Serif 4 Variable", "Source Serif 4", Georgia, "Times New Roman", serif',
    },
  },
  shortcuts: {
    // The content column: 48rem = 768px = 80 columns of 16px IBM Plex Mono.
    "col-80": "max-w-3xl mx-auto px-5 sm:px-8",
    "col-wide": "max-w-6xl mx-auto px-5 sm:px-8",
    "text-body": "font-serif text-ink dark:text-snow",
    "text-quiet": "text-ink-mute dark:text-snow-mute",
    "btn":
      "inline-flex items-center gap-2 font-mono text-sm px-4 py-2 rounded cursor-pointer transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-50 no-underline",
    "btn-primary":
      "btn bg-hong text-white hover:bg-hong-deep dark:bg-hong dark:hover:bg-hong-deep",
    "btn-quiet":
      "btn bg-paper-raised text-ink hover:bg-paper-shade dark:bg-night-raised dark:text-snow dark:hover:bg-night-shade",
    "input-base":
      "bg-paper-raised dark:bg-night-raised text-ink dark:text-snow rounded px-3 py-1.5 focus:outline-none focus:ring-2 focus:ring-hong/60 dark:focus:ring-hong-bright/60",
    "surface": "bg-paper dark:bg-night",
    "surface-raised": "bg-paper-raised dark:bg-night-raised",
  },
});
