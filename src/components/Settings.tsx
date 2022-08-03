import { useEffect, useState } from "react";
import { open } from "@tauri-apps/api/dialog";
import { list_media_files, Media } from "../commands";

export default function Settings() {
	const [path, setPath] = useState<string | null>(null);

	return (
		<div className="Settings">
			<h1>settings</h1>

			<DirectorySelectionButton onSelect={setPath} />

			<h2>directory: {path ? path : "no directory selected"}</h2>

			{path && <FileList path={path} />}
		</div>
	);
}

function DirectorySelectionButton({ onSelect }: { onSelect: (path: string | null) => void }) {
	const directoryPrompt = "Pick a directory";

	const pickDirectory = async () => {
		const path = await open({
			directory: true,
			title: directoryPrompt,
		});
		if (Array.isArray(path)) {
			throw new Error("expected path to be a single string");
		}
		onSelect(path);
	};

	return <button onClick={pickDirectory}>{directoryPrompt}</button>;
}

function FileList({ path }: { path: string }) {
	const [loading, setLoading] = useState(false);
	const [media, setMedia] = useState<Media | null>(null);

	useEffect(() => {
		if (!path) return;
		setLoading(true);
		list_media_files(path).then((media) => {
			setMedia(media);
			setLoading(false);
		});
	}, [path]);

	if (loading) return <div>loading...</div>;
	if (!media) return <div>no media</div>;

	const { tracks, images } = media;

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
}
