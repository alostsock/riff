export default {
	"*.{ts,tsx}": [() => "pnpm tsc", "eslint --fix", "prettier --write"],
	"*.{html,css,scss,json,md}": ["prettier --write"],
};
