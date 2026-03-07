# js

A terminal-based interactive JSON viewer built with Rust and [ratatui](https://github.com/ratatui/ratatui).

## Usage

```sh
# Open a JSON file directly
js file.json

# Pipe JSON from stdin
cat file.json | js
curl https://api.example.com/data | js

# Launch with the interactive file picker
js
```

## Features

- **Tree view** — browse JSON as an expandable/collapsible tree
- **Vim-style navigation** — move through nodes with familiar keybindings
- **Search** — filter visible nodes with `/`
- **Clipboard** — copy a node's value or its JSON path
- **Depth expansion** — expand the tree to a specific depth with `1`–`9`
- **Mouse support** — scroll and click to navigate
- **Syntax coloring** — keys, strings, numbers, booleans, and nulls are each distinctly colored

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Collapse node / go to parent |
| `l` / `→` / `Enter` | Expand / toggle node |
| `g` | Go to top |
| `G` | Go to bottom |
| `f` / `PageDown` | Page down |
| `b` / `PageUp` | Page up |
| `e` | Expand all nodes |
| `c` | Collapse all nodes |
| `1`–`9` | Expand to depth N |
| `/` | Search |
| `n` / `N` | Next / previous search match |
| `y` | Copy value to clipboard |
| `Y` | Copy path to clipboard |
| `?` | Toggle help popup |
| `q` / `Esc` | Quit |

## Building

Requires Rust (edition 2024).

```sh
# Debug build
make build

# Release build
make release

# Run checks, linting, formatting, and release build
make
```

## Development

```sh
make check       # cargo check
make clippy      # run clippy lints
make fmt         # format code
make fmt-check   # check formatting without modifying
make test        # run tests
make clean       # clean build artifacts
```
