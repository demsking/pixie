# Security Model

Pixie treats all image files and filesystem paths as untrusted input.

## threat Model

- **Decompression Bombs**: Small files (e.g. SVG or compressed PNGs) that expand into gigabytes of memory, causing out-of-memory panics.
- **Integer Overflows**: Oversized dimensions resulting in wrapped pixel-count math during buffer allocation.
- **Unsafe Overwrite**: Unintended destructions of user files due to failed exports.
- **Metadata Leaks**: Camera metadata (EXIF/GPS) leaking private geographical locations when converting/uploading.

## Safeguards

### 1. Decoding Protections & Image Size Limits
Before decoding pixel buffers, the `pixie-core` reader parses *only* the headers to extract dimensions. It validates dimensions against strict bounds:
- **Maximum Dimension**: 16,384px (width or height).
- **Maximum Pixel Count**: 268,435,456 pixels.
- **Maximum File Size**: 100 MB.

### 2. Allocation Validation
Memory requirements are verified using checked multiplication:
$$\text{width} \times \text{height} \times \text{channels} \times \text{bytes\_per\_channel}$$
If the total byte requirement exceeds the **1 GB memory limit**, the allocation request is rejected before any buffer is allocated.

### 3. Atomic Export Strategy
To prevent corruption, file exports utilize a two-stage save:
1. Create a secure temporary file in the target directory using `tempfile`.
2. Write/encode the image, flush buffers, and call `sync_all` on the descriptor.
3. Atomically rename/replace the temporary file to the final destination path using the OS `rename` system call.

### 4. Metadata Privacy
By default, Pixie strips location details (GPS) and camera descriptors during saving.
