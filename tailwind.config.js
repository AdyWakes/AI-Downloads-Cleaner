/** @type {import('tailwindcss').Config} */
const token = (name) => `rgb(var(--${name}) / <alpha-value>)`;

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    // Replace (not extend) the default palette: only design tokens exist.
    colors: {
      transparent: "transparent",
      current: "currentColor",
      bg: token("bg"),
      surface: token("surface"),
      line: token("line"),
      ink: { DEFAULT: token("ink"), 2: token("ink-2"), 3: token("ink-3") },
      brand: token("brand"),
      action: { DEFAULT: token("action"), hover: token("action-hover") },
      accent: token("accent"),
      tint: { DEFAULT: token("tint"), strong: token("tint-strong") },
      "on-action": token("on-action"),
      success: token("success"),
      error: token("error"),
      chart: { 1: token("chart-1"), 2: token("chart-2"), 3: token("chart-3"), 4: token("chart-4"), 5: token("chart-5") },
    },
    fontFamily: {
      sans: ["IBM Plex Sans", "IBM Plex Sans Devanagari", "Segoe UI", "system-ui", "sans-serif"],
      mono: ["IBM Plex Mono", "IBM Plex Sans Devanagari", "ui-monospace", "Consolas", "monospace"],
    },
    extend: {
      borderRadius: { xl: "14px", "2xl": "20px" },
      boxShadow: {
        soft: "0 1px 2px rgb(var(--ink) / 0.04), 0 4px 16px rgb(var(--ink) / 0.04)",
        lift: "0 2px 6px rgb(var(--ink) / 0.08), 0 12px 32px rgb(var(--ink) / 0.10)",
      },
      keyframes: {
        pop: { "0%": { transform: "scale(.8)", opacity: "0" }, "100%": { transform: "scale(1)", opacity: "1" } },
        gulp: { "0%,100%": { transform: "scale(1)" }, "40%": { transform: "scale(1.12) translateY(-2px)" }, "70%": { transform: "scale(.96)" } },
        rise: { "0%": { transform: "translateY(8px)", opacity: "0" }, "100%": { transform: "translateY(0)", opacity: "1" } },
        drop: {
          "0%": { transform: "translateY(-28px) rotate(-8deg)", opacity: "0" },
          "30%": { opacity: "1" },
          "70%": { transform: "translateY(6px) rotate(0deg)", opacity: "1" },
          "100%": { transform: "translateY(14px) rotate(0deg)", opacity: "0" },
        },
        shimmer: { "0%": { transform: "translateX(-100%)" }, "100%": { transform: "translateX(250%)" } },
      },
      animation: {
        pop: "pop .3s cubic-bezier(.2,1.3,.4,1) both",
        gulp: "gulp .5s ease-out both",
        rise: "rise .4s ease-out both",
        drop: "drop 1.4s ease-in infinite",
        shimmer: "shimmer 1.4s ease-in-out infinite",
      },
    },
  },
  plugins: [],
};
