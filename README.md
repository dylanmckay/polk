# polk

Dotfile manager.

# Installation

## With Cargo

```bash
cargo install polk
```

# Examples

## General usage

```bash
# Grab and symlink dotfiles from my GitHub account.
# (assumes repository named 'dotfiles')
polk setup github:dylanmckay

# Grab and symlink dotfiles from another repository.
polk setup github:dylanmckay/otherdotfiles
```

## Multiple users/dotfile repositories

```bash
# Set up dotfiles for the default user (with what your computer username is).
# Also symlink them to ~/
polk setup github:dylanmckay

# Download dotfiles to a local cache folder but don't create symlinks
polk grab --user bob github:bob67

# Open a shell to a custom home folder with dotfiles symlinked.
polk shell --user bob

# Create symlinks to the currently grabbed dotfiles
# Replace symlinks in ~/ with what bob has
polk link --user bob

# Update the dotfiles (via git)
polk update

# Remove all symlinks created by polk.
polk unlink
```

## Utilities

```bash
# Remove all symlinks and cached dotfiles/repositories (~/.polk)
polk forget

# Print a bunch of information
polk info
```

# Your dotfiles repository

A repository would generally look something like this

```
.
..
.bashrc
.rspec
.tmux.conf
.tmux.linux.conf
.vim
.config/awesome/config.lua
README.md
```

# How symlinking works works

Here is a table of how dotfiles within a repository map to symlinks in `$HOME`.

| File                          | Symlink                                                    |
| ----------------------------- | ---------------------------------------------------------- |
|  `.bashrc`                    |  `~/.bashrc -> ~/<dotfiles repository path>/.bashrc`       |
| `.tmux.conf`                  |  `~/.tmux.conf -> ~/<dotfiles repository path>/.tmux.conf` |
| `.config/awesome/config.lua`  |  `~/.config/awesome/config.lua -> ~/<dotfiles repository path>/.config/awesome/config.lua` |

#### Handling of config files in subdirectories

As you can see in the above table, if a dotfile resides in a subdirectory(s), those directories
will get created in `$HOME` and then a symlink to the dotfile will be created within the subdirectories.

It is not possible with this tool to symlink an entire directory within a dotfiles repository to `$HOME`.
If this were possible, applications would/could write new files into the repository, which isn't good.

# Feature flags

Dotfiles can mention required features in their filenames. These dotfiles will be conditionally symlinked
depending on the current system.

When a dotfile is linked, all feature flags are substituted with the feature name. For example,
`linux` will become `os`, `x86` will become `arch`, and `unix` will become `family`.
Because of this, it is possible to source OS or arch specific dotfiles the same way across all
architectures.

Examples

| File                          | Symlink                | Note                                    |
| ----------------------------- | ---------------------- | --------------------------------------- |
| `.tmux.conf`                  | `~/.tmux.conf`         | No feature flags, will always be linked |
| `.tmux.linux.conf`            | `~/.tmux.os.conf`      | Will only be linked on Linux            |
| `.tmux.linux.x86.conf`        | `~/.tmux.os.arch.conf` | Will only be linked on x86 Linux        |

