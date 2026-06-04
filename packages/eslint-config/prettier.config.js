export default {
  importOrder: [
    "^react$",
    "^react-dom",
    "^@tanstack/",
    "^@tauri-apps/",
    "^@daydream/",
    "<THIRD_PARTY_MODULES>",
    "^@/",
    "^[./]",
    "\\.json$",
    "\\.css$",
  ],
  plugins: [
    "@trivago/prettier-plugin-sort-imports",
    "prettier-plugin-tailwindcss",
  ],
  singleQuote: true,
  trailingComma: "all",
};
