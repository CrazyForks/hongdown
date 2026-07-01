import { render } from "solid-js/web";

import "../shared";
import { DemoPage } from "../pages/DemoPage";

const root = document.getElementById("root");

if (root) {
  render(() => <DemoPage />, root);
}
