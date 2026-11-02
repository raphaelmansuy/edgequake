# SPEC-096 Screenshot Run Notes

Generated: 2026-11-02T05:26:34.791Z
Source: `edgequake_webui/e2e/spec096-extraction-language.spec.ts`

### S01
- Card visible beside entity types
- View mode shows Server default
- Future-only hint present

### S02
- Reconfigure wizard extraction step
- Chinese selected

### S03
- After Apply, view shows Chinese

### S04
- Future-only hint remains visible after save

### S06
- French selected → chips show PERSONNE / ORGANISATION
- English PERSON chip absent (preset remapped)

### S07
- English selected → General preset English tokens restored
- French PERSONNE chip absent

### S05
- Create dialog trigger not found in mocked shell — language card on workspace page verified instead
- Create form field is wired in create-workspace-wizard / TenantGuard / HeaderTenantSelector
