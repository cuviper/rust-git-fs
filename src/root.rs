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
use std::path::Path;

use inode;
use inode::{FileAttr, Id, Inode};

/// The root of the filesystem, currently just revealing HEAD and refs/
pub struct Root {
    head: Id,
    refs: Id,
}

impl Root {
    pub fn new(head: Id, refs: Id) -> Box<Inode+'static> {
        Box::new(Root {
            head: head,
            refs: refs,
        })
    }
}

impl Inode for Root {
    fn lookup(&mut self, repo: &git2::Repository, name: &Path
             ) -> Result<Id, libc::c_int> {
        if name == Path::new("HEAD") {
            repo.head().ok()
                .and_then(|head| head.target())
                .map(|oid| Id::Oid(oid))
        }
        else if name == Path::new("refs") {
            Some(self.refs)
        }
        else { None }.ok_or(libc::ENOENT)
    }

    fn getattr(&mut self, _repo: &git2::Repository, attr: FileAttr
              ) -> Result<FileAttr, libc::c_int> {
        let size = 2; // just HEAD and refs/
        Ok(FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: FileType::Directory,
            perm: 0o755,
            ..attr
        })
    }

    fn readdir<'a>(&mut self, _repo: &git2::Repository, offset: u64,
               mut add: Box<FnMut(Id, FileType, &Path) -> bool + 'a>
              ) -> Result<(), libc::c_int> {
        if offset == 0 {
            add(self.head, FileType::Directory, &Path::new("HEAD"));
        }
        if offset <= 1 {
            add(self.refs, FileType::Directory, &Path::new("refs"));
        }
        Ok(())
    }
}



