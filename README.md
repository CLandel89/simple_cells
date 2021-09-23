simple_cells
============
simple_cells is a laboratory for some cellular automata like Conway's Game Of Life.

If you are looking for a more powerful tool, have a look at http://golly.sourceforge.net/. Golly can be used to try out patterns and seeds. Though being inspired, this project is not related to Golly in any other way.

Modify seed.json and seed.png to suit your needs.

Note that the size of seed.png defines the size of the automata and the allocated RAM. If you prefer very spacious automata, you may need another software that implements data compression (such as the aforementioned more powerful tool).

To run this, open a shell, `cd` into the `simple_cells` directory and fire up the Rust toolchain WITH optimizations:
```
cargo run --release
```

Workaround for `/usr/bin/ld: cannot find -lOpenCL`:
```
sudo ln -s /usr/lib/x86_64-linux-gnu/libOpenCL.so.1 /usr/local/lib/libOpenCL.so
```
C.f. https://askubuntu.com/questions/1007591/usr-bin-ld-cannot-find-lopencl