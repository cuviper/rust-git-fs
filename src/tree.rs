// Copyright (C) 2014  Josh Stone
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use git2;
use libc;
use libc::consts::os::posix88;
use std::io;

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
        box Tree {
            oid: tree.id(),
            size: tree.len() as u64,
        }
    }

    fn tree<'a>(&self, repo: &'a git2::Repository) -> Result<git2::Tree<'a>, libc::c_int> {
        repo.find_tree(self.oid).map_err(|_| posix88::EINVAL)
    }
}

impl Inode for Tree {
    fn lookup(&mut self, repo: &git2::Repository, name: &PosixPath
              ) -> Result<Id, libc::c_int> {
        self.tree(repo).and_then(|tree| {
            match tree.get_path(name) {
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
            kind: io::FileType::Directory,
            perm: io::USER_DIR,
            ..attr
        })
    }

    fn readdir(&mut self, repo: &git2::Repository, offset: u64,
               add: |Id, io::FileType, &PosixPath| -> bool
              ) -> Result<(), libc::c_int> {
        let len = self.size;
        self.tree(repo).map(|tree| {
            for i in range(offset, len) {
                let e = match tree.get(i as uint) {
                    Some(e) => e,
                    None => continue,
                };
                let kind = match e.kind() {
                    Some(git2::ObjectType::Tree) => io::FileType::Directory,
                    Some(git2::ObjectType::Blob) => io::FileType::RegularFile,
                    _ => io::FileType::Unknown,
                };
                let path = PosixPath::new(e.name_bytes());
                if add(Id::Oid(e.id()), kind, &path) {
                    break;
                }
            }
        })
    }
}
