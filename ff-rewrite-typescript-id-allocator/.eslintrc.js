module.exports = {
	extends: [require.resolve("@fluidframework/eslint-config-fluid/minimal"), "prettier"],
	parserOptions: {
		project: "./tsconfig.eslint.json",
		tsconfigRootDir: __dirname,
	},
	rules: {
		"@typescript-eslint/no-shadow": "off",
		"space-before-function-paren": "off", // Off because it conflicts with typescript-formatter
		"import/no-nodejs-modules": ["error", { allow: ["v8", "perf_hooks", "child_process"] }],
		"import/no-internal-modules": "off", // Off because we import assert from /copied-utils
	},
	overrides: [
		{
			files: ["*.ts", "*.tsx"],
			rules: {
				"import/no-nodejs-modules": "off",
			},
		},
	],
};
