# dsplit
Directory splitting utility that lets you separate files by their name/extension using regex. Previous solution was to use GNU core utilities such as `find`, `grep`, `xargs`, and others to find certain files with regex and move them to the desired location. This utility aims to automate that task and combine it into one.

# Command line options
| Option  | Description | Required |
| ------------- | ------------- | ------------- |
| `-s` or `--source` | Source folder path where files reside. | True |
| `-e` or `--expr` | Regular expression/-s against which file names will be matched using FIFO order. | True  |
| `-f` or `--flat` | Flatten the directory structure. | False |
| `-p` or `--prefix` | Prefix of the output folder (Default: 1, 2, 3, ...). | False |
| `-o` or `--output` | Output folder where subdirectories will be created. (Default: Current working directory) | False |
| `-m` or `--move` | Move files instead of copying them. | False |
| `-t` or `--tree` | Print each group and their matched files without doing anything. | False |

# Still work in progress

# TODO
- [ ] Add description
- [x] Clean up the main function
- [ ] Write unit tests
- [ ] Refactor
