# Fimo (File Mongo)

**Fimo** is a fast and flexible CLI tool written in Rust that transforms CSV data into MongoDB documents using YAML-based field mappings and Jinja2-style templating. It's ideal for bulk inserts, updates, and upserts with full control over document structure.

---

## 🚀 Features

- ✅ RFC 4180-compliant CSV parsing (including headers, quoting, escaped quotes)
- 🛠️ Field mapping via YAML configuration
- 🧠 Custom transformation logic using [MiniJinja](https://docs.rs/minijinja/)
- 📦 MongoDB insert, update, and upsert support
- 🧪 Validate-only and dry-run modes
- 🔄 Batch processing support for large files
- 🔐 Supports Extended JSON and BSON types
- 🔣 Configurable CSV delimiter and quote characters
- 📊 Debug and verbose output for development and testing

---

## 📦 Installation

```bash
cargo install fimo


