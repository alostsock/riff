export default {
	// We need to run tsc with this function syntax so that tsconfig settings are picked up.
	// See: https://github.com/okonet/lint-staged/issues/825#issuecomment-620018284
	"*.{ts,tsx}": [() => "tsc", "eslint --fix", "prettier --write"],
	"*.{html,css,scss,json,md}": ["prettier --write"],
};
