# Vercel Deployment

The server binary is the canonical HTTP runtime. For Vercel, package the HTTP API behind a function wrapper that forwards query parameters to the same core rendering path. Keep the renderer and aggregation logic in `crates/core`; do not fork behavior into platform-specific code.
