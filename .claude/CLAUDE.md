# Project Instructions

## Package Manager Policy (RSR)

- **REQUIRED**: Deno for JavaScript/TypeScript
- **FORBIDDEN**: npm, npx, node_modules, package-lock.json
- **FORBIDDEN**: bun (unless Deno is technically impossible)

When asked to add npm packages, use Deno alternatives:
- `npm install X` → Add to import_map.json or use npm: specifier
- `npm run X` → `deno task X`
