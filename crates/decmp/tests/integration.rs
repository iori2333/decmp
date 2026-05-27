use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn decmp_bin() -> Command {
  Command::new(env!("CARGO_BIN_EXE_decmp"))
}

fn setup_test_dir() -> tempfile::TempDir {
  let tmp = tempfile::tempdir().unwrap();
  let root = tmp.path().join("input");
  fs::create_dir_all(root.join("subdir")).unwrap();
  fs::write(root.join("root.txt"), b"Hello from root file\n").unwrap();
  fs::write(root.join("subdir/nested.txt"), b"Nested file content\n").unwrap();
  fs::write(root.join("subdir/another.md"), b"# Another file\n").unwrap();
  fs::write(
    root.join("subdir/binary.bin"),
    &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05],
  )
  .unwrap();
  tmp
}

fn create_with_decmp(dest: &Path, sources: &[&Path], fmt: &str, password: Option<&str>) {
  let mut cmd = decmp_bin();
  cmd.args(["create", "-f", &dest.to_string_lossy(), "-F", fmt, "-v"]);
  for s in sources {
    cmd.args(["-s", &s.to_string_lossy()]);
  }
  if let Some(p) = password {
    cmd.args(["-p", p]);
  }
  let out = cmd.output().unwrap();
  assert!(
    out.status.success(),
    "create failed: {}",
    String::from_utf8_lossy(&out.stderr)
  );
}

fn extract_with_decmp(archive: &Path, dest: &Path, password: Option<&str>) {
  let mut cmd = decmp_bin();
  cmd.args([
    "extract",
    "-f",
    &archive.to_string_lossy(),
    "-o",
    &dest.to_string_lossy(),
    "-v",
  ]);
  if let Some(p) = password {
    cmd.args(["-p", p]);
  }
  let out = cmd.output().unwrap();
  assert!(
    out.status.success(),
    "extract failed: {}",
    String::from_utf8_lossy(&out.stderr)
  );
}

fn list_with_decmp(archive: &Path, encoding: Option<&str>) -> String {
  let mut cmd = decmp_bin();
  cmd.args(["list", "-f", &archive.to_string_lossy(), "-v"]);
  if let Some(e) = encoding {
    cmd.args(["-e", e]);
  }
  let out = cmd.output().unwrap();
  assert!(
    out.status.success(),
    "list failed: {}",
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stdout).to_string()
}

fn assert_dirs_equal(expected: &Path, actual: &Path) {
  let mut expected_entries: Vec<_> = walkdir::WalkDir::new(expected)
    .sort_by_file_name()
    .into_iter()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_type().is_file())
    .map(|e| e.path().strip_prefix(expected).unwrap().to_path_buf())
    .collect();

  let mut actual_entries: Vec<_> = walkdir::WalkDir::new(actual)
    .sort_by_file_name()
    .into_iter()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_type().is_file())
    .map(|e| e.path().strip_prefix(actual).unwrap().to_path_buf())
    .collect();

  expected_entries.sort();
  actual_entries.sort();

  assert_eq!(
    expected_entries, actual_entries,
    "directory structure differs"
  );

  for entry in &expected_entries {
    let expected_content = fs::read(expected.join(entry)).unwrap();
    let actual_content = fs::read(actual.join(entry)).unwrap();
    assert_eq!(
      expected_content,
      actual_content,
      "content differs for {}",
      entry.display()
    );
  }
}

// ── TAR family ──────────────────────────────────────────────

#[test]
fn test_tar_create_list_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("out.tar");

  create_with_decmp(&archive, &[&input], "tar", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("input/root.txt"));
  assert!(listing.contains("input/subdir/nested.txt"));
  assert!(listing.contains("input/subdir/binary.bin"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_tar_gz_create_list_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("out.tar.gz");

  create_with_decmp(&archive, &[&input], "tar.gz", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("input/root.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_tar_bz2_create_list_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("out.tar.bz2");

  create_with_decmp(&archive, &[&input], "tar.bz2", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("input/root.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_tar_xz_create_list_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("out.tar.xz");

  create_with_decmp(&archive, &[&input], "tar.xz", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("input/root.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_tar_zst_create_list_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("out.tar.zst");

  create_with_decmp(&archive, &[&input], "tar.zst", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("input/root.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

// ── ZIP ─────────────────────────────────────────────────────

#[test]
fn test_zip_create_list_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("out.zip");

  create_with_decmp(&archive, &[&input], "zip", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("input/root.txt"));
  assert!(listing.contains("input/subdir/nested.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_zip_encrypted_create_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("enc.zip");

  create_with_decmp(&archive, &[&input], "zip", Some("mypassword"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, Some("mypassword"));
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_zip_encrypted_wrong_password_fails() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("enc.zip");

  create_with_decmp(&archive, &[&input], "zip", Some("correct"));

  let mut cmd = decmp_bin();
  cmd.args([
    "extract",
    "-f",
    &archive.to_string_lossy(),
    "-o",
    &tmp.path().join("fail_out").to_string_lossy(),
    "-p",
    "wrong",
  ]);
  let out = cmd.output().unwrap();
  assert!(!out.status.success());
}

// ── 7z ──────────────────────────────────────────────────────

#[test]
fn test_7z_create_list_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("out.7z");

  create_with_decmp(&archive, &[&input], "7z", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("input/root.txt"));
  assert!(listing.contains("input/subdir/nested.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_7z_encrypted_create_extract() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("enc.7z");

  create_with_decmp(&archive, &[&input], "7z", Some("s3cret"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, Some("s3cret"));
  assert_dirs_equal(&input, &out_dir.join("input"));
}

// ── Single-file compressed ──────────────────────────────────

#[test]
fn test_gz_single_file_roundtrip() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("data.txt");
  fs::write(&src, b"Hello, gzip!").unwrap();
  let archive = tmp.path().join("data.txt.gz");

  create_with_decmp(&archive, &[&src], "gz", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("data.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  let extracted = fs::read(out_dir.join("data.txt")).unwrap();
  assert_eq!(extracted, b"Hello, gzip!");
}

#[test]
fn test_zst_single_file_roundtrip() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("data.txt");
  fs::write(&src, b"Hello, zstd!").unwrap();
  let archive = tmp.path().join("data.txt.zst");

  create_with_decmp(&archive, &[&src], "zst", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("data.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  let extracted = fs::read(out_dir.join("data.txt")).unwrap();
  assert_eq!(extracted, b"Hello, zstd!");
}

#[test]
fn test_xz_single_file_roundtrip() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("data.txt");
  fs::write(&src, b"Hello, xz!").unwrap();
  let archive = tmp.path().join("data.txt.xz");

  create_with_decmp(&archive, &[&src], "xz", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("data.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  let extracted = fs::read(out_dir.join("data.txt")).unwrap();
  assert_eq!(extracted, b"Hello, xz!");
}

#[test]
fn test_bz2_single_file_roundtrip() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("data.txt");
  fs::write(&src, b"Hello, bzip2!").unwrap();
  let archive = tmp.path().join("data.txt.bz2");

  create_with_decmp(&archive, &[&src], "bz2", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("data.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  let extracted = fs::read(out_dir.join("data.txt")).unwrap();
  assert_eq!(extracted, b"Hello, bzip2!");
}

// ── Shell-created archives ──────────────────────────────────

fn create_with_shell(dest: &Path, input: &Path, fmt: &str) {
  match fmt {
    "zip" => {
      let name = input.file_name().unwrap().to_string_lossy();
      Command::new("zip")
        .args(["-r", &dest.to_string_lossy(), &name])
        .current_dir(input.parent().unwrap())
        .output()
        .unwrap();
    }
    "tar" | "tar.gz" | "tar.bz2" | "tar.xz" => {
      let name = input.file_name().unwrap().to_string_lossy();
      let flag = match fmt {
        "tar" => "cf",
        "tar.gz" => "czf",
        "tar.bz2" => "cjf",
        "tar.xz" => "cJf",
        _ => unreachable!(),
      };
      Command::new("tar")
        .args([flag, &dest.to_string_lossy(), &name])
        .current_dir(input.parent().unwrap())
        .output()
        .unwrap();
    }
    "gz" => {
      Command::new("gzip")
        .args(["-k", "-9", &input.to_string_lossy()])
        .output()
        .unwrap();
      let generated = PathBuf::from(format!("{}.gz", input.to_string_lossy()));
      if generated != dest {
        fs::rename(&generated, dest).unwrap();
      }
    }
    "bz2" => {
      Command::new("bzip2")
        .args(["-k", "-9", &input.to_string_lossy()])
        .output()
        .unwrap();
      let generated = PathBuf::from(format!("{}.bz2", input.to_string_lossy()));
      if generated != dest {
        fs::rename(&generated, dest).unwrap();
      }
    }
    "xz" => {
      Command::new("xz")
        .args(["-k", "-9", &input.to_string_lossy()])
        .output()
        .unwrap();
      let generated = PathBuf::from(format!("{}.xz", input.to_string_lossy()));
      if generated != dest {
        fs::rename(&generated, dest).unwrap();
      }
    }
    "zst" => {
      Command::new("zstd")
        .args([
          &input.to_string_lossy(),
          "-o",
          &dest.to_string_lossy(),
          "-19",
          "--force",
        ])
        .output()
        .unwrap();
    }
    _ => panic!("unknown format: {fmt}"),
  }
  assert!(dest.exists(), "shell tool failed to create {dest:?}");
}

#[test]
fn test_extract_shell_zip() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("shell.zip");
  create_with_shell(&archive, &input, "zip");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_extract_shell_tar() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("shell.tar");
  create_with_shell(&archive, &input, "tar");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_extract_shell_tar_gz() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("shell.tar.gz");
  create_with_shell(&archive, &input, "tar.gz");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_extract_shell_tar_bz2() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("shell.tar.bz2");
  create_with_shell(&archive, &input, "tar.bz2");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_extract_shell_tar_xz() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("shell.tar.xz");
  create_with_shell(&archive, &input, "tar.xz");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_dirs_equal(&input, &out_dir.join("input"));
}

#[test]
fn test_extract_shell_gz() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("data.txt");
  fs::write(&src, b"Hello from gzip!").unwrap();
  let archive = tmp.path().join("data.txt.gz");
  create_with_shell(&archive, &src, "gz");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  let extracted = fs::read(out_dir.join("data.txt")).unwrap();
  assert_eq!(extracted, b"Hello from gzip!");
}

#[test]
fn test_extract_shell_bz2() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("data.txt");
  fs::write(&src, b"Hello from bzip2!").unwrap();
  let archive = tmp.path().join("data.txt.bz2");
  create_with_shell(&archive, &src, "bz2");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  let extracted = fs::read(out_dir.join("data.txt")).unwrap();
  assert_eq!(extracted, b"Hello from bzip2!");
}

#[test]
fn test_extract_shell_xz() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("data.txt");
  fs::write(&src, b"Hello from xz!").unwrap();
  let archive = tmp.path().join("data.txt.xz");
  create_with_shell(&archive, &src, "xz");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  let extracted = fs::read(out_dir.join("data.txt")).unwrap();
  assert_eq!(extracted, b"Hello from xz!");
}

#[test]
fn test_extract_shell_zst() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("data.txt");
  fs::write(&src, b"Hello from zstd!").unwrap();
  let archive = tmp.path().join("data.txt.zst");
  create_with_shell(&archive, &src, "zst");

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  let extracted = fs::read(out_dir.join("data.txt")).unwrap();
  assert_eq!(extracted, b"Hello from zstd!");
}

// ── Edge cases ──────────────────────────────────────────────

#[test]
fn test_create_rejects_no_sources() {
  let tmp = tempfile::tempdir().unwrap();
  let archive = tmp.path().join("empty.zip");

  let mut cmd = decmp_bin();
  cmd.args(["create", "-f", &archive.to_string_lossy()]);
  let out = cmd.output().unwrap();
  assert!(!out.status.success());
}

#[test]
fn test_create_rejects_nonexistent_source() {
  let tmp = tempfile::tempdir().unwrap();
  let archive = tmp.path().join("bad.zip");

  let mut cmd = decmp_bin();
  cmd.args([
    "create",
    "-f",
    &archive.to_string_lossy(),
    "-s",
    "/nonexistent/path",
  ]);
  let out = cmd.output().unwrap();
  assert!(!out.status.success());
}

#[test]
fn test_extract_rejects_nonexistent_archive() {
  let tmp = tempfile::tempdir().unwrap();

  let mut cmd = decmp_bin();
  cmd.args([
    "extract",
    "-f",
    "/nonexistent/archive.zip",
    "-o",
    &tmp.path().join("out").to_string_lossy(),
  ]);
  let out = cmd.output().unwrap();
  assert!(!out.status.success());
}

#[test]
fn test_list_rejects_unsupported_format() {
  let tmp = tempfile::tempdir().unwrap();
  let bad = tmp.path().join("file.xyz");
  fs::write(&bad, b"data").unwrap();

  let mut cmd = decmp_bin();
  cmd.args(["list", "-f", &bad.to_string_lossy()]);
  let out = cmd.output().unwrap();
  assert!(!out.status.success());
}

#[test]
fn test_empty_file_archive() {
  let tmp = tempfile::tempdir().unwrap();
  let src = tmp.path().join("empty.txt");
  fs::write(&src, b"").unwrap();
  let archive = tmp.path().join("empty.zip");

  create_with_decmp(&archive, &[&src], "zip", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("empty.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_eq!(fs::metadata(out_dir.join("empty.txt")).unwrap().len(), 0);
}

#[test]
fn test_verbose_flag_shows_sizes() {
  let tmp = setup_test_dir();
  let input = tmp.path().join("input");
  let archive = tmp.path().join("out.zip");
  create_with_decmp(&archive, &[&input], "zip", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("Compressed"));
  assert!(listing.contains("entries"));
}

#[test]
fn test_multiple_source_files() {
  let tmp = tempfile::tempdir().unwrap();
  let a = tmp.path().join("a.txt");
  let b = tmp.path().join("b.txt");
  fs::write(&a, b"file A").unwrap();
  fs::write(&b, b"file B").unwrap();
  let archive = tmp.path().join("multi.zip");

  create_with_decmp(&archive, &[&a, &b], "zip", None);

  let listing = list_with_decmp(&archive, None);
  assert!(listing.contains("a.txt"));
  assert!(listing.contains("b.txt"));

  let out_dir = tmp.path().join("out");
  extract_with_decmp(&archive, &out_dir, None);
  assert_eq!(fs::read(out_dir.join("a.txt")).unwrap(), b"file A");
  assert_eq!(fs::read(out_dir.join("b.txt")).unwrap(), b"file B");
}
