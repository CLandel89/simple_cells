simple_cells
============
simple_cells is a laboratory for some cellular automata like Conway's Game Of Life. It uses the GPU via OpenCL in order to achieve a high calculation speed.

simple_cells is well-suited to process chaotic patterns, which, e.g., result from the accompanied seeds in conjunction with my favorite ruleset (B3/S1256).

`seed.json` and `seed.png` contain all the data that define the game.

`prefs.json` contains all preferences for live monitoring, regular snapshots of the playfield, and benchmarking.

This tool slows down with greater playfields. This is so because simple_cells handles empty space and repetitive patterns the same way as it does with chaotic patterns; while VRAM would usually suffice for vast playfields, the algorithm is too simple to speed up in such a use-case.

A powerful editor for cellular automata in general is http://golly.sourceforge.net/.
Golly can be used to try out patterns and rulestrings very efficiently, as well as vast playfields that do not exhibit as much chaotic behavior.
Though being inspired, this project is not related to Golly in any other way.

To run this, open a shell, `cd` into the `simple_cells` directory and fire up the Rust toolchain WITH optimizations:
```
cargo run --release
```

A proof of concept can be found here: [proof-of-concept](doc/proof-of-concept.md).
