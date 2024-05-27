# Weechess

A rust port of [Weechess](https://github.com/ryanwebber/weechess/tree/main): A wee-little chess engine library and application.
    
    
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

The engine is also factored to produce a library that can be used independently of the
bundled UCI server or terminal application binaries.


## Usage

```bash
$ cargo run --release 

Usage: weechess [COMMAND]

Commands:
  display   Print out the board in a human-readable format
  evaluate  Evaluate a position
  perft     Walk the move generation tree of strictly legal moves to count all the leaf nodes of a certain depth
  repl      Start an interactive REPL session with the engine
  uci       Start a UCI client
  version   Print out the version of the engine
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
