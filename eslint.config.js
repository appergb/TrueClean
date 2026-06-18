import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactPlugin from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import simpleImportSort from "eslint-plugin-simple-import-sort";
import prettierConfig from "eslint-config-prettier";
import globals from "globals";

export default tseslint.config(
  // ── Global ignores ───────────────────────────────────────────────
  {
    ignores: [
      "dist/**",
      "src-tauri/**",
      "node_modules/**",
      "coverage/**",
      ".claude/**",
      "proposal/**",
      "tests/**",
      "*.config.{ts,js,mjs,cjs}",
    ],
  },

  // ── Base recommended rules (JS + TS) ─────────────────────────────
  js.configs.recommended,
  ...tseslint.configs.recommended,

  // ── Config files (vite.config.ts, eslint.config.js, …) ───────────
  {
    files: ["*.config.{ts,js,mjs,cjs}"],
    languageOptions: {
      globals: { ...globals.node },
    },
  },

  // ── Frontend source ──────────────────────────────────────────────
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      globals: { ...globals.browser },
      parserOptions: {
        ecmaFeatures: { jsx: true },
      },
    },
    settings: {
      react: { version: "detect" },
    },
    plugins: {
      react: reactPlugin,
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
      "simple-import-sort": simpleImportSort,
    },
    rules: {
      // React (jsx-runtime — React import not required in scope)
      "react/react-in-jsx-scope": "off",
      "react/prop-types": "off",
      "react/jsx-key": "warn",
      // Hooks
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      // Fast Refresh — only export components from component files
      "react-refresh/only-export-components": [
        "warn",
        { allowConstantExport: true },
      ],
      // No console.log (warn/error allowed for runtime diagnostics)
      "no-console": ["error", { allow: ["warn", "error"] }],
      // No any — explicit types required
      "@typescript-eslint/no-explicit-any": "error",
      // Unused vars (warn; allow _-prefixed)
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      // Import sorting
      "simple-import-sort/imports": "warn",
      "simple-import-sort/exports": "warn",
    },
  },

  // ── Disable formatting rules that conflict with Prettier ─────────
  prettierConfig,
);
