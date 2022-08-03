import "./App.css";

import { Route, Switch } from "wouter";
import Settings from "./Settings";

function App() {
	return (
		<div className="App">
			<Switch>
				<Route path="/">
					<Settings />
				</Route>
				<Route>Invalid route!</Route>
			</Switch>
		</div>
	);
}

export default App;
