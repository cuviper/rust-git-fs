//! Test that this very testfile is accessible in our own mount.

extern crate gitfs;

use std::fs;
use std::path::Path;

// FIXME: use PathExt::exists() once stable
fn exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
}

#[test]
fn mounted_test_exists() {
    let git_dir = Path::new(".git");
    let mount = git_dir.join("fs");
    let file = mount.join("HEAD").join(file!());

    // NB: If this isn't a git checkout, we'll fail here, sorry!
    let fs = gitfs::GitFS::new(&git_dir).unwrap();

    assert!(!exists(&file), "{:?} shouldn't exist before mounting!", file);

    let session = unsafe { fs.spawn_mount(&mount) }.unwrap();

    assert!(exists(&file), "{:?} should exist in the mount!", file);

    drop(session);

    assert!(!exists(&file), "{:?} shouldn't exist after unmounting!", file);
}
