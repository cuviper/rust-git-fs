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
pub struct Tree<'a> {
    tree: git2::Tree<'a>,
}

impl<'a> Tree<'a> {
    pub fn new(tree: git2::Tree<'a>) -> Box<Inode> {
        box Tree {
            tree: tree,
        }
    }
}

impl<'a> Inode for Tree<'a> {
    fn lookup(&mut self, _repo: &git2::Repository, name: &PosixPath
              ) -> Result<Id, libc::c_int> {
        self.tree.get_path(name).map(|e| Id::Oid(e.id()))
            .map_err(|_| posix88::ENOENT)
    }

    fn getattr(&mut self, _repo: &git2::Repository, attr: FileAttr
               ) -> Result<FileAttr, libc::c_int> {
        let size = self.tree.len() as u64;
        Ok(FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: io::FileType::Directory,
            perm: io::USER_DIR,
            ..attr
        })
    }

    fn readdir(&mut self, _repo: &git2::Repository, offset: u64,
               add: |Id, io::FileType, &PosixPath| -> bool
              ) -> Result<(), libc::c_int> {
        let len = self.tree.len() as u64;
        for i in range(offset, len) {
            let e = match self.tree.get(i as uint) {
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
        Ok(())
    }
}
