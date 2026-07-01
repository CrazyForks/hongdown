// Data for the landing page's install section, mirroring README.md.

export interface InstallMethod {
  name: string;
  command: string;
}

export const INSTALL_METHODS: InstallMethod[] = [
  { name: "winget", command: "winget install HongMinhee.Hongdown" },
  { name: "Scoop", command: "scoop bucket add extras\nscoop install hongdown" },
  { name: "npm", command: "npm install -g hongdown" },
  { name: "mise", command: "mise use -g aqua:dahlia/hongdown" },
  { name: "Nix", command: "nix run github:dahlia/hongdown" },
  { name: "Cargo", command: "cargo install hongdown" },
];

export const RELEASES_URL = "https://github.com/dahlia/hongdown/releases";
