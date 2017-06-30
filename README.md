# dotty

Dotfile manager.

# Installation

## With Cargo

```bash
cargo install dotty
```

# Examples

```bash
# Grab and symlink dotfiles from my GitHub account.
# (assumes repository named 'dotfiles')
dotty setup github:dylanmckay

# Grab and symlink dotfiles from another repository.
dotty setup github:dylanmckay/otherdotfiles

# Remove all symlinks created by dotty.
dotty clean

# Print all symlink information
dotty info
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

|| File                         || Symlink ||
|  `.bashrc`                    |  `~/.bashrc -> ~/<dotfiles repository path>/.bashrc`       |
| `.tmux.conf`                  |  `~/.tmux.conf -> ~/<dotfiles repository path>/.tmux.conf` |
| `.config/awesome/config.lua`  |  `~/.config/awesome/config.lua -> ~/<dotfiles repository path>/.config/awesome/config.lua` |

#### Handling of config files in subdirectories

As you can see in the above table, if a dotfile resides in a subdirectory(s), those directories
will get created in `$HOME` and then a symlink to the dotfile will be created within the subdirectories.

It is not possible with this tool to symlink an entire directory within a dotfiles repository to `$HOME`.
If this were possible, applications would/could write new files into the repository, which isn't good.

