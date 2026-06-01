# CALDERA

## Releasing

To trigger a release build:

```bash
git tag v0.1-alpha
git push origin v0.1-alpha
```

GitHub Actions will build Linux + Windows packages and publish them automatically.

Pre-release flag is auto-set if the tag contains a `-` (e.g. `v0.1-alpha`, `v0.4-beta`).

To delete a tag if a build fails:

```bash
git tag -d v0.1-alpha
git push origin :refs/tags/v0.1-alpha
```
