# fimo (file-mongo) CLI Toolkit

**fimo** is a fast, modular, and extensible command-line toolkit written in Rust. It includes multiple tools for working with MongoDB and structured data — starting with fimo-csv, a high-performance CSV-to-MongoDB importer using YAML mappings and Jinja2-style templates.

---

## 🚀 What's Included

| Binary      | Description                                                         |
| ----------- | ------------------------------------------------------------------- |
| `fimo-csv`  | Import CSV data into MongoDB using field mappings and templates     |
| `fimo-sync` | (Coming soon) Sync and transform JSON documents across environments |


---

## 📦 Installation

```bash
cargo install fimo

```

Or clone and build:

```bash
git clone https://github.com/fimo-org/fimo.git
cd fimo
cargo build --release

````

## 📂 Project Layout

```txt
fimo/
├── src/
│   └── bin/
│       ├── fimo-csv/       # Source for fimo-csv binary
│       └── fimo-sync/      # Source for fimo-sync binary (future)
├── Cargo.toml              # Defines multiple [[bin]] targets

```

### 📄 Tool Documentation

* fimo-csv: A robust CLI for CSV-to-MongoDB transformation with YAML-based schema mapping and Jinja templates
📍 [View fimo-csv README →](https://github.com/fimo-org/fimo/tree/main/src/bin/fimo-csv/README.md)

* fimo-sync: syncing JSON documents between environments

## 📜 License

MIT © 
