# fimo (File-Mongo) CLI Toolkit

**fimo** is a modular, high-performance command-line toolkit written in Rust for importing, transforming, and synchronizing structured data with MongoDB. Built for developers, data engineers, and migration tasks, it enables robust data pipelines directly from the terminal.

---

## ✨ What is fimo?

At its core, `fimo` is a collection of CLI utilities for MongoDB workflows. Each binary targets a specific use case, but they all share a fast, extensible foundation with strong type safety, async performance, and developer-friendly configuration.

---

## 🚀 Available Tools

| Binary       | Description                                                                 |
|--------------|-----------------------------------------------------------------------------|
| `fimo-csv`   | Import CSV files into MongoDB using YAML-based field mappings and templates |
| `fimo-sync`  | Sync MongoDB documents across collections or environments incrementally     |

---

## 🧰 Use Cases

- 🔁 Keep environments in sync using `fimo-sync` (e.g., staging → production)
- 📥 Load flat files using `fimo-csv` for initial imports or ongoing feeds
- ⚙️ Define repeatable transformations using Jinja2-style templates
- 🧪 Perform safe, resumable batch operations using resume tokens or values

---

## 📦 Installation

Install from crates.io:

```bash
cargo install fimo
```

Or build from source:

```bash
git clone https://github.com/fimo-org/fimo.git
cd fimo
cargo build --release
```

---

## 📂 Project Layout

```text
fimo/
├── src/
│   └── bin/
│       ├── fimo-csv/       # fimo-csv binary: CSV to MongoDB importer
│       └── fimo-sync/      # fimo-sync binary: document sync engine
├── examples/               # Example input data and config
├── tests/                  # Test harnesses
├── Cargo.toml              # Multi-binary manifest
```

---

## 📄 Tool Docs

### [`fimo-csv`](./src/bin/fimo-csv)

- High-throughput CSV importer
- YAML mapping with full BSON type support
- Supports transformations via MiniJinja
- Ideal for ETL and data warehousing

📍 [View `fimo-csv` documentation →](https://github.com/fimo-org/fimo/tree/main/src/bin/fimo-csv/README.md)

---

### [`fimo-sync`](./src/bin/fimo-sync)

- Real-time or batch sync across MongoDB collections
- Change stream and field-based sync modes
- Resume-safe with `_id` disambiguation
- Optional health file support for watchdogs

📍 [View `fimo-sync` documentation →](https://github.com/fimo-org/fimo/tree/main/src/bin/fimo-sync/README.md)

---

## 🧩 Coming Soon

### `fimo-archive`

A new utility designed to move large volumes of archived MongoDB documents to external storage such as S3, Azure Blob, or GCS. It will support:

- BSON/Extended JSON export
- Filtered and range-based archival (e.g., by date)
- Compatibility with MongoDB Online Archive or cold storage solutions
- Future support for rehydration into live collections

---

## 📈 Why fimo?

- 🚀 **Rust-powered performance**
- 🔐 **Typed BSON support**
- 🧠 **Intelligent resume logic**
- 🛠️ **Modular CLI design**
- 🧪 **Tested on large-scale datasets**

Whether you're importing 10,000 rows or syncing millions of documents, `fimo` helps you do it cleanly, reliably, and efficiently.

---

## 📜 License

MIT © fimo.org — Built with ❤️ in Rust
