# Architecture Knowledge

## Boundaries

- Interface layers parse user input and write output.
- Data clients fetch remote data and classify errors.
- Aggregators transform fetched data into stable card data.
- Renderers transform card data into SVG, text, JSON, or PNG output.
- Deployment layers package and expose the same CLI/core behavior.

## Default Shape

The default output is a single dashboard SVG so layout is controlled inside the renderer instead of relying on README HTML behavior.

## Durability Rule

When a boundary changes, update this file and record the decision in `.agents/decisions.md`.
