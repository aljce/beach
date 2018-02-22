# Beach & Umbrella
An assortment of rust for Reed CS 393
## Beach 
A simple shell written in rust
### Installation
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
Umbrella is a library that has functions and types related to file system operations.
Beach is merely a frontend to umbrella. This being said the command line interface can
be accessed by `cd beach/beach && cargo run`.
### Feature List
- newfs
- mount
- blockmap
