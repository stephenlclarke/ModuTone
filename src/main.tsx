// Phase: 8
// React entry point

import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles/tokens/base.css";
import "./styles/tokens/motion.css";
import "./styles/tokens/elevation.css";
import "./styles/themes/light.css";
import "./styles/themes/dark.css";
import "./styles/styles/quiet-precision.css";
import "./styles/styles/luminous-professional.css";
import "./styles/styles/editorial-precision.css";
import "./styles/styles/glass-slate.css";
import "./styles/global.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
