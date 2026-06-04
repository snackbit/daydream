import js from "@eslint/js";
import prettierConfig from "eslint-config-prettier/flat";
import prettierPlugin from "eslint-plugin-prettier";
import reactPlugin from "eslint-plugin-react";
import reactHooksPlugin from "eslint-plugin-react-hooks";
import unusedImportsPlugin from "eslint-plugin-unused-imports";
import globals from "globals";
import tseslint from "typescript-eslint";

const ignoredPaths = [
  "**/dist/**",
  "**/build/**",
  "**/coverage/**",
  "**/node_modules/**",
  "**/target/**",
  "**/.vite/**",
  "**/*.gen.ts",
  "**/generated/**",
  "apps/desktop/src-tauri/**",
];

export function createDaydreamConfig({ react = false } = {}) {
  return tseslint.config(
    {
      ignores: ignoredPaths,
    },
    js.configs.recommended,
    ...tseslint.configs.recommended,
    prettierConfig,
    {
      files: ["**/*.{js,mjs,cjs,ts,tsx,mts,cts}"],
      languageOptions: {
        ecmaVersion: "latest",
        globals: {
          ...globals.browser,
          ...globals.node,
        },
      },
    },
    {
      files: ["**/*.{ts,tsx,mts,cts}"],
      rules: {
        "@typescript-eslint/consistent-type-imports": [
          "error",
          {
            prefer: "type-imports",
          },
        ],
        "@typescript-eslint/no-explicit-any": [
          "warn",
          {
            ignoreRestArgs: true,
          },
        ],
        "@typescript-eslint/no-unused-vars": [
          "error",
          {
            args: "all",
            argsIgnorePattern: "^_",
            caughtErrors: "all",
            caughtErrorsIgnorePattern: "^_",
            destructuredArrayIgnorePattern: "^_",
            ignoreRestSiblings: true,
            varsIgnorePattern: "^_",
          },
        ],
      },
    },
    {
      files: ["**/*.{js,mjs,cjs,ts,tsx,mts,cts}"],
      plugins: {
        prettier: prettierPlugin,
        "unused-imports": unusedImportsPlugin,
      },
      rules: {
        "max-lines": [
          "warn",
          {
            max: 600,
            skipBlankLines: true,
            skipComments: true,
          },
        ],
        "no-console": [
          "warn",
          {
            allow: ["warn", "error"],
          },
        ],
        "no-multiple-empty-lines": [
          "error",
          {
            max: 1,
            maxEOF: 1,
          },
        ],
        "no-warning-comments": [
          "warn",
          {
            location: "anywhere",
            terms: ["todo", "fixme", "hack", "xxx"],
          },
        ],
        "prettier/prettier": "error",
        "unused-imports/no-unused-imports": "error",
      },
    },
    react
      ? {
          files: ["**/*.{tsx,jsx}"],
          languageOptions: {
            globals: {
              ...globals.browser,
            },
          },
          plugins: {
            react: reactPlugin,
            "react-hooks": reactHooksPlugin,
          },
          rules: {
            ...reactPlugin.configs.flat.recommended.rules,
            ...reactPlugin.configs.flat["jsx-runtime"].rules,
            ...reactHooksPlugin.configs.recommended.rules,
          },
          settings: {
            react: {
              version: "detect",
            },
          },
        }
      : {},
    {
      files: [
        "**/*.spec.ts",
        "**/*.spec.tsx",
        "**/*.test.ts",
        "**/*.test.tsx",
        "**/test/**/*.ts",
        "**/test/**/*.tsx",
      ],
      rules: {
        "@typescript-eslint/no-explicit-any": "off",
        "max-lines": "off",
        "no-warning-comments": "off",
      },
    },
  );
}
