// Copyright (C) 2014  Josh Stone
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # GitFS: a FUSE filesystem for Git objects

#![deny(missing_doc)]

#![feature(if_let)]

#![feature(phase)]
#[phase(plugin)]
extern crate probe;

extern crate fuse;
extern crate git2;
extern crate libc;
extern crate time;

use std::collections::hashmap;
use std::default::Default;
use std::io;
use std::u64;


mod inode;
mod blob;
mod tree;
mod reference;
mod root;


static TTY: time::Timespec = time::Timespec { sec: 1, nsec: 0 };


/// The main object implementing a FUSE filesystem.
pub struct GitFS<'a> {
    repo: git2::Repository,
    epoch: time::Timespec,
    uid: u32,
    gid: u32,
    mapper: inode::InodeMapper,
    inodes: inode::InodeContainer<'a>,
    mountdir: Option<DirHandle>,
}

impl<'a> GitFS<'a> {
    /// Create a GitFS referencing the given GIT_DIR.
    pub fn new(git_dir: &Path) -> Result<GitFS<'a>, git2::Error> {
        Ok(GitFS {
            repo: try!(git2::Repository::open(git_dir)),
            epoch: time::get_time(),
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            mapper: Default::default(),
            inodes: Default::default(),
            mountdir: None,
        })
    }

    /// Get the resolved GIT_DIR.
    pub fn git_dir(&self) -> Path {
        self.repo.path()
    }

    fn mount_options(&self) -> Vec<u8> {
        let mut options = b"-oro,default_permissions,fsname=".to_vec();
        options.push_all(self.repo.path().as_vec()); // FIXME escape commas?
        options
    }

    /// Mount the filesystem and wait until the path is unmounted, e.g. with the command
    /// `fusermount -u PATH`.
    pub fn mount(mut self, mountpoint: &Path) {
        // Create/remove the mount point if it doesn't exist
        self.mountdir = DirHandle::new(mountpoint);

        let options = self.mount_options();
        fuse::mount(self, mountpoint, &[options.as_slice()])
    }

    /// Mount the filesystem in the background.  It will remain mounted until the returned session
    /// object is dropped, or an external umount is issued.
    pub fn spawn_mount(mut self, mountpoint: &Path) -> fuse::BackgroundSession {
        // Create/remove the mount point if it doesn't exist
        self.mountdir = DirHandle::new(mountpoint);

        let options = self.mount_options();
        fuse::spawn_mount(self, mountpoint, &[options.as_slice()])
    }

    fn defattr(&self, ino: u64) -> fuse::FileAttr {
        fuse::FileAttr {
            ino: ino,
            size: 0,
            blocks: 0,
            atime: self.epoch,
            mtime: self.epoch,
            ctime: self.epoch,
            crtime: self.epoch,
            kind: io::TypeUnknown,
            perm: Default::default(),
            nlink: 1,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            flags: 0,
        }
    }
}

impl<'a> fuse::Filesystem for GitFS<'a> {
    fn init (&mut self, _req: &fuse::Request) -> Result<(), libc::c_int> {
        let root_ino = self.mapper.new_ino();
        let refs_ino = self.mapper.new_ino();
        assert_eq!(fuse::FUSE_ROOT_ID, root_ino);

        let root = root::Root::new(&self.repo, inode::Ino(refs_ino));
        self.inodes.insert(root_ino, root);

        let refs = reference::RefDir::new();
        self.inodes.insert(refs_ino, refs);

        Ok(())
    }

    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &PosixPath,
              reply: fuse::ReplyEntry) {
        probe!(gitfs, lookup, parent, name.to_c_str().as_ptr());

        let id = {
            let inode = self.inodes.find(parent);
            match inode.and_then(|inode| inode.lookup(name)) {
                Ok(id) => id,
                Err(rc) => return reply.error(rc),
            }
        };
        let ino = self.mapper.get_ino(id);

        if let hashmap::Vacant(entry) = self.inodes.entry(ino) {
            if let Some(oid) = self.mapper.get_oid(ino) {
                if let Some(inode) = inode::new_inode(&self.repo, oid) {
                    entry.set(inode);
                }
            }
        }

        let attr = self.defattr(ino);
        let inode = self.inodes.find(ino);
        match inode.and_then(|inode| inode.getattr(attr)) {
            Ok(attr) => reply.entry(&TTY, &attr, 1),
            Err(rc) => reply.error(rc),
        }
    }

    fn forget (&mut self, _req: &fuse::Request, _ino: u64, _nlookup: uint) {
        probe!(gitfs, forget, _ino, _nlookup);

        // TODO could probably drop Oid inodes, since they're easily recreated
    }

    fn getattr (&mut self, _req: &fuse::Request, ino: u64,
                reply: fuse::ReplyAttr) {
        probe!(gitfs, getattr, ino);

        let attr = self.defattr(ino);
        let inode = self.inodes.find(ino);
        match inode.and_then(|inode| inode.getattr(attr)) {
            Ok(attr) => reply.attr(&TTY, &attr),
            Err(rc) => reply.error(rc),
        }
    }

    fn read (&mut self, _req: &fuse::Request, ino: u64, _fh: u64, offset: u64, size: uint,
             reply: fuse::ReplyData) {
        probe!(gitfs, read, ino, offset, size);

        let inode = self.inodes.find(ino);
        match inode.and_then(|inode| inode.read(offset, size)) {
            Ok(data) => reply.data(data),
            Err(rc) => reply.error(rc),
        }
    }

    fn readdir (&mut self, _req: &fuse::Request, ino: u64, _fh: u64, mut offset: u64,
                mut reply: fuse::ReplyDirectory) {
        probe!(gitfs, readdir, ino, offset);

        let mapper = &mut self.mapper;
        let inode = self.inodes.find(ino);
        match inode.and_then(|inode| {
            if offset == 0 {
                offset += 1;
                reply.add(u64::MAX, offset, io::TypeDirectory, &PosixPath::new("."));
            }
            if offset == 1 {
                offset += 1;
                reply.add(u64::MAX, offset, io::TypeDirectory, &PosixPath::new(".."));
            }
            inode.readdir(offset - 2, |id, kind, path| {
                offset += 1;
                reply.add(mapper.get_ino(id), offset, kind, path)
            })
        }) {
            Ok(()) => reply.ok(),
            Err(rc) => reply.error(rc),
        }
    }
}


/// Helper for mkdir, ensuring rmdir when dropped
struct DirHandle {
    path: Path,
}

impl DirHandle {
    fn new(path: &Path) -> Option<DirHandle> {
        io::fs::mkdir(path, io::UserDir).ok()
            .map(|()| DirHandle { path: path.clone() })
    }
}

impl Drop for DirHandle {
    fn drop(&mut self) {
        io::fs::rmdir(&self.path).ok();
    }
}
