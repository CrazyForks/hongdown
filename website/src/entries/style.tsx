import { render } from "solid-js/web";

import "../shared";
import { StylePage } from "../pages/StylePage";

const root = document.getElementById("root");

if (root) {
  render(() => <StylePage />, root);
  // The document renders after load, so the browser's native
  // scroll-to-anchor finds nothing; repeat it once the content exists.
  if (location.hash) {
    const target = document.getElementById(
      decodeURIComponent(location.hash.slice(1)),
    );
    target?.scrollIntoView();
  }
}
