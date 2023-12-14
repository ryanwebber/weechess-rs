# Weechess

A rust port of Weechess: A wee-litle chess engine library and application.
    
    
## About

Weechess is a small UCI compatible chess engine. From a technical standpoint, the engine implementation features:
 * Compact bit-board format for storing game data
 * Magic-number tables for fast rook, bishop, and queen attack calculations
 * Minimax search with alpha-beta pruning and transposition tables with zobrist hashing
 * A simple, heuristic based position evaluator
 * A decent amount of tests

When architectural simplicity or code readability is in conflict with performance, this engine
has chosen the former in an attempt to provide a decently understandable reference for other
developers building their own chess engines.

The engine is also factored to produce a c++ library that can be used independently of the
bundled UCI server or terminal application binaries.


## Usage

```bash
$ cargo run --bin weechess

Usage: ./weechess [--help] {uci}

A wee-little chess engine

Optional arguments:
  --help

Subcommands:
  uci   Start the UCI server.
```
