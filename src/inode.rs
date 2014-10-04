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
use std::collections::hashmap;
use std::io;
use std::num;

use blob;
use tree;

pub use fuse::FileAttr;


/// Let Inodes use either existing inos or git2::Oid, whichever is convenient
// FIXME Using a 1:1 mapping between oid:ino breaks down when it comes to attributes.  For
// instance, Blobs and Trees have no concept of their own timestamps or permissions.  But a Tree
// does know its children's permissions in TreeEntry, and a Commit could propagate timestamps
// recursively down its Tree.  This will require Oids to be context sensitive, with 1:N inos.
#[deriving(Clone)]
pub enum Id {
    Ino(u64),
    Oid(git2::Oid),
}


/// A generic interface for different Git object types to implement.
pub trait Inode {
    /// Find a directory entry in this Inode by name.
    fn lookup(&self, _name: &PosixPath) -> Result<Id, libc::c_int> {
        Err(posix88::ENOTDIR)
    }

    /// Get the attributes of this Inode.
    fn getattr(&self, _attr: FileAttr) -> Result<FileAttr, libc::c_int> {
        Err(posix88::EINVAL)
    }

    /// Read data from this Inode.
    fn read (&self, _offset: u64, _size: uint) -> Result<&[u8], libc::c_int> {
        Err(posix88::EISDIR)
    }

    /// Read directory entries from this Inode.
    fn readdir (&self, _offset: u64, _add: |Id, io::FileType, &PosixPath| -> bool
               ) -> Result<(), libc::c_int> {
        Err(posix88::ENOTDIR)
    }
}


/// Assign new inode numbers, and map Oids to ino dynamically
// FIXME see the note on Id about 1:1 mapping trouble
#[deriving(Default)]
pub struct InodeMapper {
    max_ino: u64,
    oids: hashmap::HashMap<git2::Oid, u64>,
    inos: hashmap::HashMap<u64, git2::Oid>,
}

impl InodeMapper {
    /// Reserve a new inode number
    pub fn new_ino(&mut self) -> u64 {
        self.max_ino += 1;
        self.max_ino
    }

    /// Get the oid associated with this ino
    pub fn get_oid(&self, ino: u64) -> Option<git2::Oid> {
        self.inos.find_copy(&ino)
    }

    /// Map any Id to an inode number
    pub fn get_ino(&mut self, id: Id) -> u64 {
        match id {
            Ino(ino) => ino,
            Oid(oid) => {
                match self.oids.entry(oid) {
                    hashmap::Occupied(entry) => *entry.get(),
                    hashmap::Vacant(entry) => {
                        // NB can't call new_ino because entry holds mut
                        self.max_ino += 1;
                        let ino = self.max_ino;
                        self.inos.insert(ino, oid);
                        *entry.set(ino)
                    },
                }
            },
        }
    }
}


/// A separate container allows mut borrowing without blocking everything else
/// in the GitFS at the same time.
#[deriving(Default)]
pub struct InodeContainer<'a> {
    inodes: hashmap::HashMap<u64, Box<Inode+'a>>,
}

impl<'a> InodeContainer<'a> {
    pub fn insert(&mut self, ino: u64, inode: Box<Inode+'a>) -> bool {
        self.inodes.insert(ino, inode)
    }

    pub fn find(&'a mut self, ino: u64) -> Result<&Box<Inode>, libc::c_int> {
        self.inodes.find(&ino).ok_or(posix88::ENOENT)
    }

    pub fn prepare(&mut self, ino: u64, mapper: &mut InodeMapper, repo: &'a git2::Repository) {
        match self.inodes.entry(ino) {
            hashmap::Occupied(_) => (),
            hashmap::Vacant(entry) => {
                match mapper.get_oid(ino).and_then(|oid| new_inode(repo, oid)) {
                    Some(inode) => { entry.set(inode); },
                    None => (),
                }
            },
        }
    }
}


/// Creates an Inode from any Oid.
// FIXME see the note on Id about 1:1 mapping trouble
fn new_inode(repo: &git2::Repository, oid: git2::Oid) -> Option<Box<Inode>> {
    match repo.find_object(oid, None).ok().and_then(|o| o.kind()) {
        Some(git2::ObjectBlob) => {
            repo.find_blob(oid).ok().map(|blob| blob::Blob::new(blob))
        },
        Some(git2::ObjectTree) => {
            repo.find_tree(oid).ok().map(|tree| tree::Tree::new(tree))
        },
        Some(git2::ObjectCommit) => {
            // FIXME a first-class Commit might expose things like the message as xattrs,
            // but for now just redirect straight to the tree id.
            repo.find_commit(oid).ok()
                .and_then(|commit| new_inode(repo, commit.tree_id()))
        },
        _ => None,
    }
}


/// Compute the number of blocks needed to contain a given size.
pub fn st_blocks(size: u64) -> u64 {
    // NB FUSE apparently always uses 512-byte blocks.  Round up.
    let (blocks, extra) = num::div_rem(size, 512);
    blocks + if extra > 0 { 1 } else { 0 }
}
