# IJVRust
This is an [IJVM](https://en.wikipedia.org/wiki/IJVM) emulator written in Rust.  

## Usage
Example usage: `cargo run -r files/mandelbread.ijvm`.  
There are two example IJVM files provided in the files/ directory, along with their more human-readable JAS assembly files.  

Omit `-r` and set `debug_assertions = true` in the Cargo file to get debug output.  
Warning: This generates a lot of output on stderr.  
