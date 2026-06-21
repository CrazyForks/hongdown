Hongdown for Visual Studio Code
===============================

[![Visual Studio Marketplace badge][]][Visual Studio Marketplace]
[![Open VSX Registry badge][]][Open VSX Registry]

This extension registers [Hongdown] as a formatter for Markdown and MDX files.
It uses the bundled WebAssembly formatter by default, so no separate Hongdown
installation is required.

[Visual Studio Marketplace badge]: https://vsmarketplacebadges.dev/version/hongminhee.hongdown.svg
[Visual Studio Marketplace]: https://marketplace.visualstudio.com/items?itemName=hongminhee.hongdown
[Open VSX Registry badge]: https://img.shields.io/open-vsx/v/hongminhee/hongdown
[Open VSX Registry]: https://open-vsx.org/extension/hongminhee/hongdown
[Hongdown]: https://github.com/dahlia/hongdown


Usage
-----

Install the extension, then select *Format Document* in a Markdown file.  To
make Hongdown the default Markdown formatter:

~~~~ json
{
  "[markdown]": {
    "editor.defaultFormatter": "hongminhee.hongdown"
  },
  "[mdx]": {
    "editor.defaultFormatter": "hongminhee.hongdown"
  }
}
~~~~


Settings
--------

 -  `hongdown.backend`: `wasm` by default.  Set to `cli` to run the system
    Hongdown CLI.
 -  `hongdown.cli.path`: CLI executable path for the `cli` backend.  The
    default `hongdown` resolves from `PATH`.
 -  `hongdown.config.path`: explicit config path.  Relative paths are resolved
    from the workspace folder, or from the file's containing directory when no
    workspace is open.  When unset, the extension uses *.hongdown.toml* in the
    workspace root when present.

The WASM backend cannot execute external code block formatters configured in
`code_block.formatters`.  If you need those, use the `cli` backend.


License
-------

Distributed under the [GPL-3.0-or-later].

[GPL-3.0-or-later]: https://www.gnu.org/licenses/gpl-3.0.html
