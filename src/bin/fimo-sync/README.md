# fimo-sync

**fimo-sync** is a flexible and high-performance CLI tool written in Rust for synchronizing documents between two MongoDB collections. It supports both real-time sync via change streams and incremental sync using a chosen field (e.g., `_id`, date).

---

## 🚀 Features

- ✅ Sync at the collection level
- 🔁 Supports MongoDB **Change Streams** with resume token support
- ⏱️ Field-based incremental sync (e.g., date, ObjectId, numeric, string) with resume value support
- 🧠 Resume file and manual resume value support
- 📦 Batched writes with upsert logic
- 🔐 Full BSON type support (ObjectId, DateTime, Int64, etc.)
- 📁 Multi-mode resume handling with file or CLI-provided values
- 📝 Optional resume type declaration for correct BSON parsing
- 🔄 Compatible with MongoDB 4.0+
- 💡 Exponential backoff to avoid polling overload
- ❤️ Health check file support for monitoring
- ⚙️ Lexicographic filtering with `_id` safety

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
```

---

## 📝 Usage

### 🔄 Change Stream Mode

```bash
fimo-sync \
  --source-uri mongodb://localhost:27017 \
  --source-db staging \
  --source-collection orders \
  --target-uri mongodb://localhost:27017 \
  --target-db production \
  --target-collection orders_mirror \
  --use-change-stream \
  --resume-file resume.json \
  --limit 500
```

### 🕑 Field-Based Sync (e.g., _id or timestamp)

```bash
fimo-sync \
  --sync-field _id \
  --resume-value "64fc40e8e1234567890abcde" \
  --resume-type objectid \
  --resume-file resume.json \
```

---

## 🔧 CLI Options

| Option               | Description                                              |
|----------------------|----------------------------------------------------------|
| `--source-uri`       | MongoDB URI for source cluster                           |
| `--source-db`        | Source database name                                     |
| `--source-collection`| Source collection name                                   |
| `--target-uri`       | MongoDB URI for target cluster                           |
| `--target-db`        | Target database name                                     |
| `--target-collection`| Target collection name                                   |
| `--use-change-stream`| Use MongoDB change stream for real-time sync            |
| `--sync-field`       | Field to use for incremental sync (e.g. `_id`, `date`)   |
| `--resume-file`      | File path to persist or read resume token/value          |
| `--resume-value`     | Resume value to override file or initialize sync         |
| `--resume-type`      | Type of resume value: `objectid`, `date`, `int`, `string`|
| `--limit`            | Maximum number of documents per sync batch               |
| `--health-file`      | Optional path to write sync heartbeat timestamp          |

---

## 📁 Project Structure

```plaintext
.
├── src/
│   ├── main.rs             # CLI entry point
│   ├── cli.rs              # CLI argument parser
│   ├── sync.rs             # Sync engine
├── examples/               # Sample resume files and use cases
├── tests/                  # Sync test harness
└── Cargo.toml              # Package manifest
```

---

## 🧩 MongoDB Version Compatibility

- **Source database**: MongoDB 4.0 and newer
- **Target database**: MongoDB 5.0 and newer (minimum requirement)

### Why MongoDB 8+ is Recommended
The Rust MongoDB driver supports `bulk_write` only from MongoDB 8+. When targeting MongoDB < 8, Fimo-Sync falls back to slower per-document `replace_one` operations.

To maximize performance, MongoDB 8+ is highly recommended as the target.

---

## 💡 Health File Support

By providing `--health-file /tmp/sync.health`, Fimo-Sync writes a heartbeat timestamp (in ms since epoch) after each successful batch.

This makes it easy to:
- Monitor whether sync is active
- Detect stalled syncs
- Integrate with cron or external watchdogs

---

## 🧠 Why `_id` Is Used in Field-Based Sync

Fimo-Sync stores a resume checkpoint as:
```json
{
  "value": <lastFieldValue>,
  "_id": <ObjectId>
}
```

This enables precise, lexicographic filtering to ensure no skipped or duplicate documents:
```js
{
  $or: [
    { syncField: { $gt: resumeValue } },
    {
      $and: [
        { syncField: resumeValue },
        { _id: { $gt: lastId } }
      ]
    }
  ]
}
```

This approach works best with ObjectIds because:
- ObjectIds are ordered chronologically
- They are unique, avoiding false positives
- They support stable filtering and sorting

---

## 🔁 Exponential Backoff in Field-Based Mode

If no new documents are found, Fimo-Sync backs off incrementally:
- Waits 10s, then 20s, then 40s… up to 60s
- Once new data is found, the backoff resets to 10s

This approach avoids wasteful polling when the system is idle.

---

## 🧩 Node.js Version Recommendation

For environments where the target MongoDB is version < 8, consider using the official **Node.js implementation** of Fimo-Sync:

- Supports bulk writes down to MongoDB 5.x
- Matches feature parity with the Rust version
- Easier to deploy in Node.js-native environments

Link: [https://github.com/fimo-org/fimo-node](https://github.com/fimo-org/fimo-node)

---

## ⚠️ Important Notes on MongoDB < 8 as Target

Targeting MongoDB 5, 6, or 7 will work — but with limitations:
- Bulk write is not available
- Per-document upserts are slower
- Concurrency is limited by available threads

Use MongoDB 8+ for best performance.

---

## 📜 License

MIT © fimo.org
