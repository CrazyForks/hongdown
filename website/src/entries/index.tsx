import { render } from "solid-js/web";

import "../shared";
import { LandingPage } from "../pages/LandingPage";

const root = document.getElementById("root");

if (root) {
  render(() => <LandingPage />, root);
}
