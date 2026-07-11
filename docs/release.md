# Release Process

This document describes how version tags are created, packages built, and pages documentation updated.

## Versioning Policy

We follow [Semantic Versioning](https://semver.org):
- **Major**: Non-backward compatible API/logic changes.
- **Minor**: New features (e.g. new export formats or crop ratios).
- **Patch**: Security fixes, bug fixes, and performance updates.

## Step-by-Step Release Flow

1. **Verify Local Codebase**:
   Ensure all checks and tests pass:
   ```bash
   make ci
   ```
2. **Update Changelog**:
   Edit [CHANGELOG.md](https://gitlab.com/demsking/pixie/-/blob/main/CHANGELOG.md) to record changes under a new version heading.
3. **Commit & Tag**:
   Create a signed Git tag:
   ```bash
   git commit -am "Prepare release v0.1.0"
   git tag -s v0.1.0 -m "Release v0.1.0"
   git push origin main --tags
   ```
4. **GitLab Release Pipeline**:
   The GitLab runner detects the new version tag and starts the release pipeline:
   - Compiles release binaries.
   - Publishes build artifacts.
   - Deploys static mdBook documentation to [https://demsking.gitlab.io/pixie](https://demsking.gitlab.io/pixie).
   - Validates the Flatpak manifest.

## Rollback Plan

If a release tag fails verification or introduces a critical bug:
1. Revert the offending commit on `main`.
2. Delete the remote/local tag:
   ```bash
   git tag -d v0.1.0
   git push origin :refs/tags/v0.1.0
   ```
3. Create a patch version tag (e.g. `v0.1.1`) to override the faulty package.
