# rust-git-fs

A FUSE implementation for Git objects.

With `git-fs` one can mount a Git tree as a filesystem, then browse any
branch/commit/etc. without needing to actually check them out.

## Usage

`git-fs [GIT_DIR [MOUNTPOINT]]`

- GIT_DIR: The directory of a git repository.  A bare git directory is fine,
or if given as a working directory, it will automatically use the .git/
directory within.  Defaults to the current directory.

- MOUNTPOINT: The target to mount the filesystem.  Defaults to GIT_DIR/fs.

## Building

Use `cargo build`, which will also handle dependencies on `git2-rs` and
`rust-fuse`.  The latter will also require `fuse-devel` or `libfuse-dev`
installed on your system.

## See also

The Git SCM Wiki has a whole page for external tools, including
[filesystem interfaces](https://git.wiki.kernel.org/index.php/Interfaces,_frontends,_and_tools#Filesystem_interfaces).

## License

`rust-git-fs` is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).  See LICENSE-APACHE, and LICENSE-MIT for details.
