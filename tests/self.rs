//! Test that this very testfile is accessible in our own mount.

#![feature(path)]
#![feature(path_ext)]

extern crate gitfs;

use std::fs::PathExt;
use std::path::Path;

#[test]
fn mounted_test_exists() {
    let git_dir = Path::new(".git");
    let mount = git_dir.join("fs");
    let file = mount.join("HEAD").join(file!());

    // NB: If this isn't a git checkout, we'll fail here, sorry!
    let fs = gitfs::GitFS::new(&git_dir).unwrap();

    assert!(!file.exists(), "{:?} shouldn't exist before mounting!", file);

    let session = fs.spawn_mount(&mount).unwrap();

    assert!(file.exists(), "{:?} should exist in the mount!", file);

    drop(session);

    assert!(!file.exists(), "{:?} shouldn't exist after unmounting!", file);
}
