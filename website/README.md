# oxipix documentation site

Docusaurus site for the oxipix documentation. It is a private package (`"private": true` in `package.json`) and is
never published to npm or crates.io; the published crate excludes this directory.

The site is being rebuilt to follow the structure of docs.opencv.org: tutorials per module, an API reference per
function with its OpenCV equivalent, a migration guide from the `opencv` crate, and performance comparisons. The plan
is in [`docs/plans/OPENCV_STYLE_DOCS_PLAN.md`](../docs/plans/OPENCV_STYLE_DOCS_PLAN.md). Until it lands, the pages
under `docs/` still describe the earlier seven-crate design and are superseded by the repository `docs/` folder.

```sh
npm install
npm run start   # http://localhost:3000
npm run build
```

See [`CHANGELOG.md`](CHANGELOG.md) for changes to the site.
