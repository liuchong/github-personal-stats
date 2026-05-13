# Playbook: Change Renderer

1. Identify whether the change affects dashboard, individual cards, text output, JSON, or PNG.
2. Keep layout calculations inside renderer code.
3. Preserve fixed dimensions and `viewBox` for SVG output.
4. Add or update deterministic rendering tests.
5. Review snapshots manually.
6. Record durable rendering rules in `.agents/knowledge/rendering.md` when changed.
7. Run verification and forbidden-reference scan.
