import "./index.css";

import { configure } from "mobx";
import React from "react";
import ReactDOM from "react-dom/client";

import App from "./components/App";

configure({
	enforceActions: "observed",
	computedRequiresReaction: true,
	observableRequiresReaction: true,
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<App />
	</React.StrictMode>
);
