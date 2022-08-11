import { open } from "@tauri-apps/api/dialog";
import { observer } from "mobx-react-lite";

import mediaState from "../lib/media-state";

export default observer(function Settings() {
	return (
		<div className="Settings">
			<DirectorySelectionButton />

			<h2>directory: {mediaState.directory ?? "no directory selected"}</h2>

			<FileList />
		</div>
	);
});

function DirectorySelectionButton() {
	const directoryPrompt = "Pick a directory";

	const pickDirectory = async () => {
		const path = await open({ directory: true, title: directoryPrompt });
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
			<h3>images ({Object.keys(images).length}):</h3>
			{Object.values(images).map((img) => (
				<pre key={img.id}>
					<code>{JSON.stringify(img, null, 2)}</code>
				</pre>
			))}

			<h3>tracks ({Object.keys(tracks).length}):</h3>
			{Object.values(tracks).map((track) => (
				<pre key={track.id}>
					<code>{JSON.stringify(track, null, 2)}</code>
				</pre>
			))}
		</div>
	);
});
