import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/__tests__/setup.ts"],
    css: true,
    coverage: {
      provider: "v8",
      reporter: ["lcov", "text", "clover"],
      include: ["src/**/*.ts", "src/**/*.tsx"],
      exclude: [
        "src/__tests__/**",
        "src/types-gen/**",
        "src/**/*.d.ts",
        "src/main.tsx",
      ],
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
