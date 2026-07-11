# Memory Management

Pixie maintains a low memory footprint by avoiding unnecessary copies and validating allocations.

## Checked Allocations

All buffer sizing is protected against integer overflow using checked arithmetic:

```rust
let area = width.checked_mul(height)?;
let total_bytes = area.checked_mul(channels * bytes_per_channel)?;
```

No vector allocations occur until `total_bytes` is verified to be under **1 GB**.

## Buffer Ownership & Preview Scaling

- **Lazy Rendering**: The full pipeline is only computed at full resolution when saving/exporting.
- **On-Screen Canvas**: The GTK DrawingArea paints the current image to a Cairo `ImageSurface` using `Argb32` format. This surface is updated only when the pipeline state changes.
- **Buffer Recycling**: Ownership of the pixel buffer is consumed and replaced by operations (`DynamicImage::crop`, `resize_exact`) where possible, reducing heap churn.
- **Cache Eviction**: Undo/redo histories store only compact operation parameters (`ImageOperation` enums) instead of full-size image frames. Redundant temporary buffers are immediately dropped when the background threads return.
