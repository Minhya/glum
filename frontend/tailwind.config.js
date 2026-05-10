/** @type {import('tailwindcss').Config} */
const config = {
    content: [
        "./app/**/*.{ts,tsx}",
        "./features/**/*.{ts,tsx}",
        "./shared/**/*.{ts,tsx}"
    ],
    presets: [require("nativewind/preset")],
    theme: {
        extend: {}
    }
};

module.exports = config;