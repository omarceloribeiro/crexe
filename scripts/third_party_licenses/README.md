# Reviewed dependency notice supplements

These files supplement license and notice texts missing from particular crates.io
archives used by CREXE. They retain their upstream terms; CREXE's MIT license and
the scoped MIT-0 output permission do not relicense them.

`index.json` pins each supplement to a package name, version and declared Cargo
license expression. Every snapshot records its original relative path, a raw
GitHub URL with a full commit identifier, and its SHA-256. The package helper checks
these hashes and declarations and uses these local files without downloading them.
Identical upstream files share one local snapshot, while each entry retains the
appropriate source URL. Preserve the original bytes, including line endings.

The sources were checked on 2026-11-25 against the versions resolved for Windows,
Linux and macOS. This is a conservative notice inventory, not an SBOM or a legal
certification. Build and development dependencies can also appear in a package's
inventory. A dependency update requires a fresh review of its notice coverage;
the reviewed version entries are not wildcard permissions.

## Cases requiring more than a filename scan

- The egui family omits the repository's MIT and Apache-2.0 texts from its crate
  archives. `epaint_default_fonts` needs these code-license texts **in addition**
  to its bundled font notices. The collector separately includes the Hack font's
  `fonts/Hack-Regular.txt`, the emoji notice, OFL and Ubuntu Font Licence.
- AccessKit's repository texts include the separate Chromium notice. The collector
  also preserves the copyright and license header of `accesskit` 0.24.1, whose
  upstream MIT text has no copyright line. `accesskit_winit` declares Apache-2.0
  only; its entry does not substitute the dual license used by other family crates.
- `gl_generator` and `khronos_api` need the upstream Apache-2.0 text. The collector
  preserves audited source and registry headers for `khronos_api` 3.1.0, including
  the distinct EGL and ANGLE notices, instead of treating all vendored data as
  Apache-2.0. The ANGLE license omitted from the crate archive is copied from the
  exact submodule commit pinned by gl-rs and mapped to `api_angle/LICENSE`.
  Its 36 included WebGL extension XML files have no separate license or
  copyright notices to extract.
- Older objc2 releases provide a complete upstream MIT text. Newer releases
  provide `LICENSE.md`, a declaration and discussion with links rather than the
  full license terms. Its Apple SDK discussion is preserved without resolving that
  upstream question. The declared alternatives are supplemented with the matching
  standard SPDX texts and an explanatory note in the distributed inventory.
- `dispatch` 0.2.0 declares MIT in its versioned upstream `Cargo.toml`, but its
  archive and corresponding Git tree have no standalone license file or specific
  copyright notice. Its entry therefore links the declaration and supplies the
  unchanged SPDX MIT reference text, explicitly identified as a template.

SPDX files retain their original placeholders. They are standard reference texts,
not new grants attributed to invented copyright holders. Copyright ownership is
never inferred from Cargo's `authors` field. Explanatory notes in `index.json` are
included in the package's `THIRD-PARTY-NOTICES.txt` so these limitations remain
visible outside this source repository.

Other available notices are collected directly from each dependency, preserving
relative paths and exact bytes, including nested ring, Unicode and Wayland texts.
The helper fails when a reachable dependency has neither a local notice nor a
reviewed supplement. That check is a useful packaging guard; the presence of a
file alone does not prove comprehensive legal coverage of a future release.

## Updating a supplement

1. Inspect the dependency archive and its `.cargo_vcs_info.json`, then inspect the
   exact upstream commit and relevant bundled materials.
2. Preserve all applicable notices and alternatives. Record exceptional filenames
   or embedded headers explicitly in the collector rather than copying arbitrary
   source trees.
3. Obtain the upstream text at an immutable URL, retain its exact bytes, and record
   its hash and package declaration in `index.json`. Document incomplete upstream
   notices honestly; do not manufacture an attribution.
4. Run the collector tests and inspect the filtered Cargo graphs for all supported
   native targets. The resulting package manifest must hash every included notice.
