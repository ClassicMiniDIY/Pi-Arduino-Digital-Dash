# Release Guide

This guide explains how to create releases for the Pi-Arduino Digital Dash project.

## Automated Release Process

Releases are automatically created when you push a version tag to GitHub.

### Creating a Release

1. **Ensure your code is ready**
   - All changes committed and pushed to `main` branch
   - Arduino sketches compile successfully
   - Documentation is up to date

2. **Create and push a version tag**
   ```bash
   git tag -a v1.0.0 -m "Release version 1.0.0"
   git push origin v1.0.0
   ```

3. **GitHub Actions will automatically:**
   - Compile both Arduino sketches for Mega 2560
   - Package all files (hex files, source code, INI, documentation)
   - Create SHA256 checksums
   - Generate release notes
   - Create a GitHub Release with all assets

### Version Tag Format

Use semantic versioning: `vMAJOR.MINOR.PATCH`

- `v1.0.0` - Initial stable release
- `v1.1.0` - Minor feature additions
- `v1.0.1` - Bug fixes and patches
- `v2.0.0` - Major changes or breaking changes

### Release Contents

Each release includes:
- **ZIP package** containing everything
- **Compiled .hex files** for direct upload to Arduino
- **Source .ino files** for customization
- **TunerStudio INI configuration**
- **All documentation** (README, CANBUS guide, CLAUDE.md)
- **SHA256 checksums** for verification

## Continuous Integration

### On Every Push/PR to Main

The following checks run automatically:

1. **Arduino Compilation** (`arduino-compile.yml`)
   - Compiles both sketches
   - Ensures code builds without errors
   - Artifacts available for 30 days

2. **Documentation Validation** (`validate-docs.yml`)
   - Verifies all required files exist
   - Checks signature consistency (`speeduino-travis`)
   - Validates baud rate settings (115200)

## Manual Release (Without Tags)

If you need to create a release manually without using tags:

1. Go to your repository on GitHub
2. Click "Releases" → "Draft a new release"
3. Create a new tag (e.g., `v1.0.0`)
4. The release workflow will trigger automatically

## Troubleshooting

### Workflow Fails on Compilation
- Check that your sketches compile locally in Arduino IDE
- Ensure all required libraries are specified in the workflow
- Review error logs in Actions tab

### Release Not Creating
- Verify tag format starts with `v` (e.g., `v1.0.0`, not `1.0.0`)
- Check repository permissions (workflows need `contents: write`)
- Ensure you're pushing tags: `git push origin --tags`

### Missing Files in Release
- Update `release.yml` to include additional files in the `Create release package` step
- Ensure files are committed to the repository before tagging

## Best Practices

1. **Test before tagging**: Ensure sketches compile and upload successfully
2. **Update version strings**: Update version info in `.ino` files if needed
3. **Document changes**: Update README or add notes about what's new
4. **Verify artifacts**: Download and test the release package after creation
5. **Delete failed tags**: If a release fails, delete the tag and fix issues before re-tagging
   ```bash
   git tag -d v1.0.0              # Delete locally
   git push origin :refs/tags/v1.0.0  # Delete remotely
   ```

## Example Release Workflow

```bash
# Make your changes
git add .
git commit -m "Add new sensor calibration for oil temp"
git push origin main

# Wait for CI checks to pass (check Actions tab)

# Create release
git tag -a v1.2.0 -m "Add improved oil temp calibration"
git push origin v1.2.0

# Watch the release workflow in Actions tab
# Release will appear in the Releases section when complete
```
