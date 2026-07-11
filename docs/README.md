# Introduction to Pixie

Welcome to the documentation for **Pixie — Modern Image Processor**.

Pixie is a lightweight, secure, and modern desktop application designed primarily for Linux and GNOME environments. It is built from scratch using **Rust**, **GTK4**, and **Libadwaita**.

## Purpose

Unlike complex graphics packages or fully featured image editors, Pixie focuses on providing fast, pixel-accurate, and non-destructive core operations:
- **Interactive Cropping**: Free aspect ratio, standard presets, and circular masks.
- **Checked Resizing**: Enlargement prevention and Lanczos3 high-quality filtering.
- **Accurate Rotations**: Fast lossless 90/180-degree paths and bilinear arbitrary rotations.
- **Metadata Protection**: Privacy controls that strip metadata by default.
- **Atomic Operations**: Safe background task queuing and atomic file exports.

## Table of Contents

- [Architecture](./architecture.md): Codebase layout, workspaces, domain pipeline model.
- [Security Model](./security.md): Threat vectors, decompression bomb safeguards, atomic writes.
- [Memory Management](./memory-management.md): Dimension constraints, buffer consumption, allocations.
- [Development Guide](./development.md): Environment dependencies, Makefile task lists, Flatpaks.
- [Release Process](./release.md): Version tags, packaging pipelines, changelog policies.
