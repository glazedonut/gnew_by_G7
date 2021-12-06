# Gnew

## Downloading and installing

To download gnew:

    git clone https://github.com/glazedonut/gnew_by_G7

To install:

    cd gnew_by_G7
    cargo install --path .

Make sure that you have `.cargo/bin` in your PATH to be able to run the installed program:

    export PATH="~/.cargo/bin:$PATH"

## Testing

To run tests:

    make -C tests

or in verbose mode:

    VERBOSE=1 make -C tests

## List of supported commands

```
add <FILES>                             Add files to tracking list

cat <COMMIT> <PATH>                     Output a file at a commit

cat-object <blob|tree|commit> <HASH>    Show the content of an object

checkout <BRANCH|COMMIT>                Update the working directory
         -b                             Create a new branch
         --force, -f                    Ignore currently untracked files (Warning: they will be lost!)

clone <PATH>                            Copy an existing repository

commit <MESSAGE>                        Commit changes to the repository

diff [<COMMIT1> [<COMMIT2>]]            Show changes between commits or a commit and the working directory

hash-file <PATH>                        Write a blob object from a file

heads                                   List the heads

help                                    Prints this message or the help of the given subcommand(s)

init                                    Create an empty repository

log [AMOUNT]                            Show the commit log of the current branch

merge <COMMIT>                          Merge two commits

pull <PATH>                             Pull changes from another repository for the current branch
     --all, -a                          Pull changes for all branches

push <PATH>                             Push changes to another repository for the current branch
     --all, -a                          Push changes to all branches

remove <FILES>                          Remove files from tracking list

status                                  Show the repository status

write-tree                              Write a tree object from the working directory
```
