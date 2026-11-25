//! Bounded, portable ZIP snapshots. Nothing is extracted while inspecting an archive.
//! The preview accepts ordinary single-disk ZIPs, not ZIP64 or self-extracting files.
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

/// Recognize ZIP headers, including a truncated binary header, without treating
/// ordinary prose such as "PK is an abbreviation" as an archive.
pub(crate) fn has_zip_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK") && bytes.get(2).is_some_and(|byte| *byte < 0x20)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    pub max_archive_bytes: u64,
    pub max_entries: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_depth: usize,
    pub max_ratio: u64,
}

impl Limits {
    pub(crate) fn recipe() -> Self {
        Self {
            max_archive_bytes: 8 * 1024 * 1024,
            max_entries: 128,
            max_file_bytes: 256 * 1024,
            max_total_bytes: 1024 * 1024,
            max_depth: 12,
            max_ratio: 200,
        }
    }

    pub(crate) fn bundle() -> Self {
        Self {
            max_archive_bytes: 256 * 1024 * 1024,
            max_entries: 1024,
            max_file_bytes: 128 * 1024 * 1024,
            max_total_bytes: 256 * 1024 * 1024,
            max_depth: 16,
            max_ratio: 200,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ArchiveFile {
    pub bytes: Vec<u8>,
    pub executable: bool,
}

#[derive(Debug)]
pub(crate) struct Archive {
    pub files: BTreeMap<String, ArchiveFile>,
    pub raw_bytes: Vec<u8>,
    pub raw_sha256: String,
    pub content_sha256: String,
}

pub(crate) fn read(path: &Path, limits: &Limits) -> Result<Archive> {
    crate::paths::reject_links(path)?;
    let file = File::open(path).with_context(|| format!("Opening package {}", path.display()))?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > limits.max_archive_bytes {
        bail!(
            "Package must be a regular file of at most {} bytes",
            limits.max_archive_bytes
        );
    }
    let mut bytes = Vec::new();
    file.take(limits.max_archive_bytes + 1)
        .read_to_end(&mut bytes)?;
    from_bytes(bytes, limits)
}

pub(crate) fn from_bytes(bytes: Vec<u8>, limits: &Limits) -> Result<Archive> {
    if bytes.len() as u64 > limits.max_archive_bytes {
        bail!("ZIP exceeds the compressed size limit");
    }
    // zip 2 uses an IndexMap and can silently collapse identical central names.
    // Validate the original directory before handing it to the decompressor.
    let count = preflight(&bytes, limits)?;
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).context("Invalid ZIP archive")?;
    if zip.len() != count || zip.offset() != 0 {
        bail!("ZIP has duplicate entries or an unsupported prefix");
    }
    let mut files = BTreeMap::new();
    let mut total = 0u64;
    let mut validated_names = BTreeSet::new();
    let mut validated_paths = BTreeMap::new();
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .context("Cannot read ZIP entry (encryption is unsupported)")?;
        let name = std::str::from_utf8(entry.name_raw())
            .context("ZIP names must be UTF-8")?
            .to_owned();
        validate_name(&name, limits, &mut validated_names, &mut validated_paths)?;
        if entry.size() > limits.max_file_bytes
            || entry.size() > entry.compressed_size().saturating_mul(limits.max_ratio)
        {
            bail!("ZIP entry exceeds size or compression ratio limits: {name}");
        }
        let mode = entry.unix_mode().unwrap_or(0);
        let kind = mode & 0o170000;
        if entry.encrypted() || entry.is_symlink() || !matches!(kind, 0 | 0o040000 | 0o100000) {
            bail!("Encrypted entries, links and special files are unsupported: {name}");
        }
        if (kind == 0o040000 && !entry.is_dir()) || (kind == 0o100000 && entry.is_dir()) {
            bail!("Inconsistent ZIP entry type: {name}");
        }
        if entry.is_dir() {
            if entry.size() != 0 {
                bail!("ZIP directory contains data: {name}");
            }
            continue;
        }
        let mut contents = Vec::new();
        (&mut entry)
            .take(limits.max_file_bytes + 1)
            .read_to_end(&mut contents)
            .with_context(|| format!("Invalid ZIP data or checksum: {name}"))?;
        if contents.len() as u64 != entry.size() || contents.len() as u64 > limits.max_file_bytes {
            bail!("ZIP entry size mismatch or limit exceeded: {name}");
        }
        total = total
            .checked_add(contents.len() as u64)
            .context("ZIP size overflow")?;
        if total > limits.max_total_bytes {
            bail!("ZIP expanded size limit exceeded");
        }
        files.insert(
            name,
            ArchiveFile {
                bytes: contents,
                executable: mode & 0o111 != 0,
            },
        );
    }
    let mut digest = Sha256::new();
    digest.update(b"CREXE archive content v1\0");
    for (name, file) in &files {
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update([u8::from(file.executable)]);
        digest.update((file.bytes.len() as u64).to_le_bytes());
        digest.update(&file.bytes);
    }
    let content_sha256 = format!("{:x}", digest.finalize());
    let raw_sha256 = format!("{:x}", Sha256::digest(&bytes));
    Ok(Archive {
        files,
        raw_bytes: bytes,
        raw_sha256,
        content_sha256,
    })
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16> {
    let value = bytes
        .get(offset..offset + 2)
        .context("Truncated ZIP header")?;
    Ok(u16::from_le_bytes(value.try_into().unwrap()))
}
fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let value = bytes
        .get(offset..offset + 4)
        .context("Truncated ZIP header")?;
    Ok(u32::from_le_bytes(value.try_into().unwrap()))
}

fn preflight(bytes: &[u8], limits: &Limits) -> Result<usize> {
    if !bytes.starts_with(b"PK\x03\x04") && !bytes.starts_with(b"PK\x05\x06") {
        bail!("Expected an ordinary ZIP archive");
    }
    let eocd = (bytes.len().saturating_sub(65_557)..bytes.len().saturating_sub(21))
        .rev()
        .find(|&pos| {
            bytes.get(pos..pos + 4) == Some(b"PK\x05\x06")
                && u16_at(bytes, pos + 20).is_ok_and(|n| pos + 22 + n as usize == bytes.len())
        })
        .context("Missing ZIP end-of-directory record")?;
    let count = u16_at(bytes, eocd + 10)? as usize;
    let central_size = u32_at(bytes, eocd + 12)? as usize;
    let central_start = u32_at(bytes, eocd + 16)? as usize;
    if u16_at(bytes, eocd + 4)? != 0
        || u16_at(bytes, eocd + 6)? != 0
        || u16_at(bytes, eocd + 8)? as usize != count
        || count == 0xffff
        || central_size == u32::MAX as usize
        || central_start == u32::MAX as usize
        || central_start.checked_add(central_size) != Some(eocd)
    {
        bail!("ZIP64, multi-disk and prefixed ZIP archives are unsupported in this preview");
    }
    if count > limits.max_entries {
        bail!("ZIP entry count limit exceeded");
    }
    let mut cursor = central_start;
    let mut names = BTreeSet::new();
    let mut paths: BTreeMap<String, (String, bool)> = BTreeMap::new();
    let mut intervals = Vec::new();
    let mut total = 0u64;
    for _ in 0..count {
        if bytes.get(cursor..cursor + 4) != Some(b"PK\x01\x02") {
            bail!("Invalid ZIP central directory");
        }
        let flags = u16_at(bytes, cursor + 8)?;
        let method = u16_at(bytes, cursor + 10)?;
        let compressed = u32_at(bytes, cursor + 20)? as u64;
        let expanded = u32_at(bytes, cursor + 24)? as u64;
        let name_len = u16_at(bytes, cursor + 28)? as usize;
        let extra_len = u16_at(bytes, cursor + 30)? as usize;
        let comment_len = u16_at(bytes, cursor + 32)? as usize;
        let local = u32_at(bytes, cursor + 42)? as usize;
        if flags & (1 | 0x40 | 0x2000) != 0 || u16_at(bytes, cursor + 34)? != 0 {
            bail!("Encrypted or multi-disk ZIP entries are unsupported");
        }
        if !matches!(method, 0 | 8) {
            bail!("Only stored or deflated ZIP entries are supported");
        }
        if expanded > limits.max_file_bytes
            || expanded > compressed.saturating_mul(limits.max_ratio)
            || compressed > limits.max_archive_bytes
        {
            bail!("ZIP per-file size or compression ratio limit exceeded");
        }
        total = total.checked_add(expanded).context("ZIP size overflow")?;
        if total > limits.max_total_bytes {
            bail!("ZIP expanded size limit exceeded");
        }
        let end = cursor
            .checked_add(46 + name_len + extra_len + comment_len)
            .context("ZIP offset overflow")?;
        if end > eocd {
            bail!("Truncated ZIP directory entry");
        }
        let raw_name = bytes
            .get(cursor + 46..cursor + 46 + name_len)
            .context("Truncated ZIP name")?;
        let name = std::str::from_utf8(raw_name).context("ZIP names must be UTF-8")?;
        validate_name(name, limits, &mut names, &mut paths)?;
        validate_extra(&bytes[cursor + 46 + name_len..cursor + 46 + name_len + extra_len])?;
        if bytes.get(local..local + 4) != Some(b"PK\x03\x04")
            || u16_at(bytes, local + 6)? != flags
            || u16_at(bytes, local + 8)? != method
        {
            bail!("ZIP local and central headers disagree: {name}");
        }
        let local_name_len = u16_at(bytes, local + 26)? as usize;
        let local_extra_len = u16_at(bytes, local + 28)? as usize;
        if bytes.get(local + 30..local + 30 + local_name_len) != Some(raw_name) {
            bail!("ZIP local and central names disagree: {name}");
        }
        let local_extra = bytes
            .get(local + 30 + local_name_len..local + 30 + local_name_len + local_extra_len)
            .context("Truncated ZIP local extra fields")?;
        validate_extra(local_extra)?;
        if flags & 8 == 0
            && (u32_at(bytes, local + 14)? != u32_at(bytes, cursor + 16)?
                || u32_at(bytes, local + 18)? as u64 != compressed
                || u32_at(bytes, local + 22)? as u64 != expanded)
        {
            bail!("ZIP local and central sizes or checksums disagree: {name}");
        }
        let data_end = local
            .checked_add(30 + local_name_len + local_extra_len)
            .and_then(|start| start.checked_add(compressed as usize))
            .context("ZIP offset overflow")?;
        if data_end > central_start {
            bail!("ZIP entry overlaps the central directory: {name}");
        }
        intervals.push((local, data_end));
        cursor = end;
    }
    if cursor != eocd {
        bail!("Unexpected records in ZIP central directory");
    }
    intervals.sort_unstable();
    if intervals.first().is_some_and(|(start, _)| *start != 0)
        || intervals.windows(2).any(|pair| pair[0].1 > pair[1].0)
    {
        bail!("ZIP has overlapping entries or an unsupported prefix");
    }
    Ok(count)
}

fn validate_extra(bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;
    while cursor < bytes.len() {
        let kind = u16_at(bytes, cursor)?;
        let length = u16_at(bytes, cursor + 2)? as usize;
        // Unicode path fields can replace the already-validated central filename.
        // ZIP64 can replace size/offset fields and is unnecessary within preview limits.
        if matches!(kind, 0x0001 | 0x7075) {
            bail!("ZIP64 and alternate Unicode path fields are unsupported in this preview");
        }
        cursor += 4 + length;
        if cursor > bytes.len() {
            bail!("Truncated ZIP extra field");
        }
    }
    Ok(())
}

fn validate_name(
    name: &str,
    limits: &Limits,
    names: &mut BTreeSet<String>,
    paths: &mut BTreeMap<String, (String, bool)>,
) -> Result<()> {
    if !name.is_ascii() || name.contains('\\') || name.len() > 240 {
        bail!("ZIP internal paths must be portable ASCII, use '/', and fit in 240 bytes: {name}");
    }
    let directory = name.ends_with('/');
    let normalized = name.strip_suffix('/').unwrap_or(name);
    crate::paths::relative_file(normalized)?;
    let parts: Vec<_> = normalized.split('/').collect();
    if parts.len() > limits.max_depth {
        bail!("ZIP directory depth limit exceeded: {name}");
    }
    if !names.insert(normalized.to_ascii_lowercase()) {
        bail!("Duplicate ZIP path: {name}");
    }
    for index in 0..parts.len() {
        let canonical = parts[..=index].join("/");
        let key = canonical.to_ascii_lowercase();
        let is_directory = index + 1 != parts.len() || directory;
        if let Some((existing, existing_directory)) = paths.get(&key) {
            if existing != &canonical || !existing_directory || !is_directory {
                bail!("Case collision or file/directory conflict in ZIP path: {name}");
            }
        } else {
            paths.insert(key, (canonical, is_directory));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    pub(super) fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in entries {
            writer
                .start_file(
                    *name,
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn content_identity_ignores_entry_order_but_raw_snapshot_is_exact() {
        let bytes = zip(&[("spec.md", b"spec"), ("tests.md", b"tests")]);
        let a = from_bytes(bytes.clone(), &Limits::recipe()).unwrap();
        let b = from_bytes(
            zip(&[("tests.md", b"tests"), ("spec.md", b"spec")]),
            &Limits::recipe(),
        )
        .unwrap();
        assert_eq!(a.raw_bytes, bytes);
        assert_eq!(a.raw_sha256, format!("{:x}", Sha256::digest(&a.raw_bytes)));
        assert_eq!(a.content_sha256, b.content_sha256);
        assert_ne!(a.raw_sha256, b.raw_sha256);
    }

    #[test]
    fn rejects_traversal_portable_aliases_and_case_conflicts() {
        for name in [
            "../escape",
            "/absolute",
            "C:/drive",
            "a\\b",
            "a/NUL.md",
            "a:stream",
            "a/./b",
            "a//b",
            "a./b",
            "ação.md",
        ] {
            assert!(
                from_bytes(zip(&[(name, b"x")]), &Limits::recipe()).is_err(),
                "accepted {name}"
            );
        }
        for entries in [
            vec![("A.md", b"a".as_slice()), ("a.md", b"b".as_slice())],
            vec![("dir", b"a".as_slice()), ("dir/file.md", b"b".as_slice())],
            vec![("Dir/a.md", b"a".as_slice()), ("dir/b.md", b"b".as_slice())],
        ] {
            assert!(from_bytes(zip(&entries), &Limits::recipe()).is_err());
        }
    }

    #[test]
    fn rejects_duplicate_central_entries_before_zip_can_collapse_them() {
        let mut bytes = zip(&[("a.md", b"a"), ("b.md", b"b")]);
        for index in 0..bytes.len() - 4 {
            if &bytes[index..index + 4] == b"b.md" {
                bytes[index] = b'a';
            }
        }
        assert!(from_bytes(bytes, &Limits::recipe())
            .unwrap_err()
            .to_string()
            .contains("Duplicate"));
    }

    #[test]
    fn rejects_limits_and_checksum_corruption() {
        let mut limits = Limits::recipe();
        limits.max_entries = 0;
        assert!(from_bytes(zip(&[("a.md", b"hello")]), &limits).is_err());
        limits = Limits::recipe();
        limits.max_file_bytes = 4;
        assert!(from_bytes(zip(&[("a.md", b"hello")]), &limits).is_err());
        limits = Limits::recipe();
        limits.max_total_bytes = 6;
        assert!(from_bytes(zip(&[("a.md", b"hello"), ("b.md", b"hi")]), &limits).is_err());
        let mut bytes = zip(&[("a.md", b"hello")]);
        let index = bytes.windows(5).position(|b| b == b"hello").unwrap();
        bytes[index] ^= 1;
        assert!(from_bytes(bytes, &Limits::recipe()).is_err());
    }

    #[test]
    fn rejects_high_ratio_symlinks_and_encryption() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "spec.md",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(&vec![b'a'; 100_000]).unwrap();
        assert!(from_bytes(writer.finish().unwrap().into_inner(), &Limits::recipe()).is_err());
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .add_symlink("link", "../outside", SimpleFileOptions::default())
            .unwrap();
        assert!(from_bytes(writer.finish().unwrap().into_inner(), &Limits::recipe()).is_err());
        let mut bytes = zip(&[("spec.md", b"hello")]);
        bytes[6] |= 1;
        let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
        bytes[central + 8] |= 1;
        assert!(from_bytes(bytes, &Limits::recipe()).is_err());
    }

    #[test]
    fn rejects_extra_fields_that_replace_validated_paths_or_sizes() {
        for kind in [0x7075u16, 0x0001u16] {
            let mut bytes = zip(&[("spec.md", b"hello")]);
            let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
            let name_len = u16_at(&bytes, central + 28).unwrap() as usize;
            bytes[central + 30..central + 32].copy_from_slice(&4u16.to_le_bytes());
            let field = [kind as u8, (kind >> 8) as u8, 0, 0];
            bytes.splice(central + 46 + name_len..central + 46 + name_len, field);
            let eocd = bytes.len() - 22;
            let size = u32_at(&bytes, eocd + 12).unwrap() + 4;
            bytes[eocd + 12..eocd + 16].copy_from_slice(&size.to_le_bytes());
            assert!(from_bytes(bytes, &Limits::recipe())
                .unwrap_err()
                .to_string()
                .contains("unsupported"));
        }
    }
}
