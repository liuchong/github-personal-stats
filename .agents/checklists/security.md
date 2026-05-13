# Security Checklist

- No secrets in tracked files.
- No raw private API responses in fixtures.
- Tokens are accepted only through explicit environment variables or platform secret stores.
- Logs redact credentials and sensitive headers.
- Errors avoid leaking token values or private resource names.
- External URLs are configurable only where needed and validated before use.
- Archive downloads verify checksums before execution.
