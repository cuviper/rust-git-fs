// Copyright (C) 2014  Josh Stone
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use fuse::FileType;
use git2;
use libc;
use libc::consts::os::posix88;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{AsPath, Path};

use inode;
use inode::{FileAttr, Id, Inode};

/// Git trees are represented as directories
// FIXME needs context, e.g. permissions from TreeEntry and timestamps from Commit
pub struct Tree {
    oid: git2::Oid,
    size: u64,
}

impl Tree {
    pub fn new(tree: git2::Tree) -> Box<Inode+'static> {
        Box::new(Tree {
            oid: tree.id(),
            size: tree.len() as u64,
        })
    }

    fn tree<'a>(&self, repo: &'a git2::Repository) -> Result<git2::Tree<'a>, libc::c_int> {
        repo.find_tree(self.oid).map_err(|_| posix88::EINVAL)
    }
}

impl Inode for Tree {
    fn lookup(&mut self, repo: &git2::Repository, name: &Path
              ) -> Result<Id, libc::c_int> {
        self.tree(repo).and_then(|tree| {
            match tree.get_path(name.as_path()) {
                Ok(e) => Ok(Id::Oid(e.id())),
                Err(_) => Err(posix88::ENOENT),
            }
        })
    }

    fn getattr(&mut self, _repo: &git2::Repository, attr: FileAttr
               ) -> Result<FileAttr, libc::c_int> {
        Ok(FileAttr {
            size: self.size,
            blocks: inode::st_blocks(self.size),
            kind: FileType::Directory,
            perm: 0o755,
            ..attr
        })
    }

    fn readdir<'a>(&'a mut self, repo: &git2::Repository, offset: u64,
               mut add: Box<FnMut(Id, FileType, &Path) -> bool + 'a>
              ) -> Result<(), libc::c_int> {
        let len = self.size;
        self.tree(repo).map(|tree| {
            for i in (offset..len) {
                let e = match tree.get(i as usize) {
                    Some(e) => e,
                    None => continue,
                };
                let kind = match e.kind() {
                    Some(git2::ObjectType::Tree) => FileType::Directory,
                    Some(git2::ObjectType::Blob) => FileType::RegularFile,
                    _ => FileType::CharDevice, /* something weird?!? unknown... */
                };
                let os_path = <OsStr as OsStrExt>::from_bytes(e.name_bytes());
                if add(Id::Oid(e.id()), kind, Path::new(os_path)) {
                    break;
                }
            }
        })
    }
}
