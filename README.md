# Beach & Umbrella
An assortment of rust for Reed CS 393
## Beach 
A simple shell
### Compilation
Install [rust](https://www.rust-lang.org/en-US/install.html).
```bash
git clone https://github.com/mckeankylej/beach
cd beach/beach
cargo run
```
### Feature List
- Process Creation
- Process Sequencing (|, &&, ||)
- Stdout Redirection (>)
- Input Escaping (\\)
- Full tty support (try running vim)
- Tab completion of files
- History completion (up/down keys)
## Umbrella
A simple file system
### Compilation
```bash
git clone https://github.com/mckeankylej/beach
cd beach/umbrella
cargo test
```
This being said if you want the command line interface follow the compilation
instructions for *Beach*
### Feature List
- newfs
- mount
- blockmap
