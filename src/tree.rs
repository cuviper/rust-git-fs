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

/// Git trees are represented as directories
// FIXME needs context, e.g. permissions from TreeEntry and timestamps from Commit
pub struct Tree<'a> {
    tree: git2::Tree<'a>,
}

impl<'a> Tree<'a> {
    pub fn new(tree: git2::Tree<'a>) -> Box<inode::Inode> {
        box Tree {
            tree: tree,
        }
    }
}

impl<'a> inode::Inode for Tree<'a> {
    fn lookup(&self, name: &PosixPath) -> Result<inode::Id, libc::c_int> {
        self.tree.get_path(name).map(|e| inode::Oid(e.id()))
            .map_err(|_| posix88::ENOENT)
    }

    fn getattr(&self, attr: inode::FileAttr) -> Result<inode::FileAttr, libc::c_int> {
        let size = self.tree.len() as u64;
        Ok(inode::FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: io::TypeDirectory,
            perm: io::UserDir,
            ..attr
        })
    }

    fn readdir (&self, offset: u64, add: |inode::Id, io::FileType, &PosixPath| -> bool
               ) -> Result<(), libc::c_int> {
        let len = self.tree.len() as u64;
        for i in range(offset, len) {
            let e = match self.tree.get(i as uint) {
                Some(e) => e,
                None => continue,
            };
            let kind = match e.kind() {
                Some(git2::ObjectTree) => io::TypeDirectory,
                Some(git2::ObjectBlob) => io::TypeFile,
                _ => io::TypeUnknown,
            };
            let path = PosixPath::new(e.name_bytes());
            if add(inode::Oid(e.id()), kind, &path) {
                break;
            }
        }
        Ok(())
    }
}
