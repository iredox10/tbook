# tbook

A feature-rich **Terminal User Interface (TUI) E-book Reader** written in Rust. Read EPUB and PDF files with a premium experience, high-resolution image support, and keyboard-driven navigation.

## ✨ Features

- **Format Support**: EPUB and PDF.
- **High-Res Images**: Supports Kitty, Sixel, and iTerm2 graphics protocols.
- **Reading Progress**: Automatic saving and resuming.
- **Library Management**: SQLite-backed database with "Last Read" sorting.
- **Visual Statistics**: Track your daily reading habits with daily word count charts.
- **Interactive Zoom**: Adjust text width and margin with mouse or keyboard.
- **Annotations**: Highlight text and add notes.
- **Local AI**: (Coming Soon) Local research assistant via Ollama.
- **Knowledge Sync**: Export notes to Obsidian/Logseq with YAML frontmatter.

## 🚀 Installation

### Using Cargo (Recommended)
```bash
cargo install tbook
```

### Using NPM
```bash
npm install -g tbook-reader
```

### From Source
```bash
git clone https://github.com/iredox/tbook.git
cd tbook
make
```

## 🎮 Controls

### Global
- `?`: Toggle Help
- `q`: Back / Quit

### Library View
- `j`/`k`: Navigate Books
- `Enter`: Open Selected Book
- `i`: View Reading Statistics
- `n`: Scan filesystem for new books
- `S`: Global search across library

### Reader View
- `j`/`k`: Scroll text
- `h`/`l`: Previous / Next Chapter
- `+`/`-`: Adjust Text Size (Zoom)
- `a`: Toggle Auto-scroll
- `s`: Enter Select Mode
- `E`: Export notes to Markdown

### Select Mode
- `w`/`b`: Move by word
- `v`: Start visual selection
- `d`: Dictionary lookup

## 🛠️ Requirements
- **Rust/Cargo**: To build and run.
- **poppler-utils**: Required for fast PDF text extraction (`pdftotext`).
- **Modern Terminal**: Kitty, WezTerm, Ghostty, or iTerm2 for high-quality image support.

## 📄 License
MIT
