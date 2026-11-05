/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter", "Segoe UI", "system-ui", "-apple-system", "Noto Sans", "Noto Sans Devanagari", "sans-serif"],
      },
      colors: {
        brand: {
          50: "#eefbf5", 100: "#d6f5e6", 200: "#b0ead0", 300: "#7cd9b3", 400: "#46c191",
          500: "#22a677", 600: "#158660", 700: "#126b4f", 800: "#125541", 900: "#104637",
        },
      },
      keyframes: {
        pop: { "0%": { transform: "scale(.6)", opacity: "0" }, "100%": { transform: "scale(1)", opacity: "1" } },
      },
      animation: { pop: "pop .35s cubic-bezier(.2,1.4,.4,1) both" },
    },
  },
  plugins: [],
};
