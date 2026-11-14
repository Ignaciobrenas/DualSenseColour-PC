## Descripción
<!-- Describe brevemente los cambios incluidos en este Pull Request -->

Closes #<numero-de-issue>

## Tipo de cambio
- [ ] Nueva funcionalidad (`feat`)
- [ ] Corrección de error (`fix`)
- [ ] Diseño / UI (`design`)
- [ ] Documentación (`docs`)
- [ ] CI / Empaquetado (`ci` / `build`)

## Checklist de verificación
- [ ] `cargo fmt --check` pasa sin errores
- [ ] `cargo clippy --all-targets -- -D warnings` pasa sin advertencias
- [ ] `cargo test` pasa al 100% (incluidos tests HID/Mock)
- [ ] `npm run lint && npm run typecheck && npm test` pasa sin errores
- [ ] La rama destino es `staging` (o `main` solo para releases desde `staging`)
