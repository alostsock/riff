import { open } from "@tauri-apps/api/dialog";
import { observer } from "mobx-react-lite";

import mediaState from "../lib/media-state";

export default observer(function Settings() {
	return (
		<div className="Settings">
			<DirectorySelectionButton />

			<h2>directory: {mediaState.directory ?? "no directory selected"}</h2>

			{mediaState.directory && <FileList />}
		</div>
	);
});

function DirectorySelectionButton() {
	const directoryPrompt = "Pick a directory";

	const pickDirectory = async () => {
		const path = await open({
			directory: true,
			title: directoryPrompt,
		});
		if (Array.isArray(path)) {
			throw new Error("expected path to be a single string");
		}
		if (path) {
			mediaState.setDirectory(path);
		}
	};

	return <button onClick={pickDirectory}>{directoryPrompt}</button>;
}

const FileList = observer(function FileList() {
	if (mediaState.isLoading) return <div>loading...</div>;
	if (!mediaState.media) return <div>no media</div>;

	const { tracks, images } = mediaState.media;

	return (
		<div>
			<h3>images ({images.length}):</h3>
			{images.map((s) => (
				<div key={s}>{s}</div>
			))}

			<h3>tracks ({tracks.length}):</h3>
			{tracks.map((s) => (
				<div key={s}>{s}</div>
			))}
		</div>
	);
});
