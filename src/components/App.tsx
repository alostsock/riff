import "./App.css";

import { Route, Switch } from "wouter";

import Library from "./Library";
import Nav from "./Nav";
import Settings from "./Settings";

function App() {
	return (
		<div className="App">
			<Nav />
			<Switch>
				<Route path="/">
					<Library />
				</Route>
				<Route path="/settings">
					<Settings />
				</Route>
				<Route>Invalid route! aaa</Route>
			</Switch>
		</div>
	);
}

export default App;
