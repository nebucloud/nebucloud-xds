# Changesets

This directory is used to track changes that should be included in the next release.

## How it works

When you make a change that should be noted in the changelog:

1. Run `npx changeset` (or add a changeset file manually)
2. Select the packages that have changed
3. Choose the semver bump type (patch, minor, major)
4. Write a description of the change

The changeset will be stored as a markdown file in this directory until the next release.

## For Rust Projects

Since this is a Rust project, we also use `git-cliff` for changelog generation.
See `cliff.toml` in the project root for configuration.

### Creating a changeset manually

Create a file like `.changeset/descriptive-name.md`:

```markdown
---
"xds-core": patch
"xds-cache": patch
---

Brief description of the change.
```

### Version types

- `patch` - Bug fixes, no API changes
- `minor` - New features, backwards compatible
- `major` - Breaking changes

## Release Process

1. Changesets accumulate in `.changeset/`
2. On release, run the release workflow
3. Versions are bumped and CHANGELOGs updated
4. Packages are published to crates.io
