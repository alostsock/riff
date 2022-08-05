import { Link } from "wouter";

const links = [
	{
		path: "/",
		label: "Library",
	},
	{
		path: "/settings",
		label: "Settings",
	},
];

export default function Nav() {
	return (
		<div style={{ display: "flex", gap: "1rem" }}>
			{links.map((link) => (
				<Link key={link.path} href={link.path}>
					<a>
						<h1>{link.label}</h1>
					</a>
				</Link>
			))}
		</div>
	);
}
