<!-- fa7cbfe9-3a6b-4273-af5b-d0b8adb2a865 d4a458d8-67d9-4b40-bd72-bf09493441e5 -->
# TailwindCSS Integration Plan for Leptos Frontend

## Overview

Integrate TailwindCSS into the Leptos frontend (`front/` folder) following the [Leptos Tailwind example](https://github.com/leptos-rs/leptos/blob/4f3a26ce88eb1b429d382a871c47e086be96d559/examples/tailwind_actix/README.md). The project uses `cargo-leptos` which processes SCSS files through dart-sass and Lightning CSS.

## Current State

- Leptos project with Actix Web backend
- SCSS file configured: `style/main.scss` (currently has basic body styles)
- `cargo-leptos` build system processes SCSS files
- No TailwindCSS configuration exists in the `front/` folder

## Implementation Steps

### 1. Create package.json in front folder

Create `front/package.json` with TailwindCSS dependencies:

- `tailwindcss` (dev dependency)
- `postcss` (dev dependency) 
- `autoprefixer` (dev dependency)

### 2. Initialize TailwindCSS configuration

Create `front/tailwind.config.js` with content paths pointing to:

- `./src/**/*.rs` (Rust source files where Tailwind classes are used)
- `./style/**/*.scss` (SCSS files)

### 3. Create PostCSS configuration

Create `front/postcss.config.js` to process TailwindCSS and Autoprefixer:

- Configure TailwindCSS plugin
- Configure Autoprefixer plugin

### 4. Update SCSS entry point

Modify `front/style/main.scss` to include Tailwind directives:

- `@tailwind base;`
- `@tailwind components;`
- `@tailwind utilities;`
- Remove or keep existing body styles (optional)

### 5. Install dependencies

Run `npm install` in the `front/` directory to install TailwindCSS and related packages.

## Files to Create/Modify

### New Files

- `front/package.json` - npm dependencies for TailwindCSS
- `front/tailwind.config.js` - TailwindCSS configuration
- `front/postcss.config.js` - PostCSS configuration

### Modified Files

- `front/style/main.scss` - Add Tailwind directives

## Technical Notes

- `cargo-leptos` will process the SCSS file through dart-sass, which will handle the Tailwind directives
- The compiled CSS will be optimized by Lightning CSS before being written to `target/site/pkg/front.css`
- Tailwind classes can be used directly in Leptos `view!` macros as string attributes
- The configuration should scan `.rs` files for Tailwind class usage in string literals

## Optional Enhancements

- Add VS Code settings for TailwindCSS IntelliSense (recommended but not required)
- Consider adding `tailwindcss-animate` plugin if animations are needed (matches root project setup)