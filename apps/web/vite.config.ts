import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// base: "/" for a custom domain; set PAGES_BASE="/engram/" for the project page.
export default defineConfig({
  base: process.env.PAGES_BASE ?? "/",
  plugins: [
    react({ babel: { plugins: ["babel-plugin-react-compiler"] } }),
    tailwindcss(),
  ],
});
