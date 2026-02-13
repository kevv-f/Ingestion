# EHL v13 Data Mapping & Cross-System References

## Overview

Content in Clace lives across two separate storage systems that never share a database file. This document maps every table, file, and reference that connects them.

## Storage Systems

### System A: `ehl_v13_rs` (Vector Engine)

Each index manifold (e.g., `ehl_index/directives/`, `ehl_index/episodic/`) contains:

| File                   | Format                            | Contents                                      |
| ---------------------- | --------------------------------- | --------------------------------------------- |
| `vectors.bin`          | Raw binary (packed f32 or binary) | Embedding vectors, appended sequentially      |
| `metadata.db`          | SQLite                            | Chunk text + JSON metadata                    |
| `inverted_indices.bin` | Binary                            | Sparse inverted index for candidate filtering |
| `inverted_indptr.bin`  | Binary                            | Index pointer array for inverted index        |
| `inverted_meta.bin`    | Binary                            | Inverted index metadata                       |
| `doc_norms.bin`        | Binary                            | Document norms for scoring                    |

#### `metadata.db` Schema

```sql
chunks (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    vector_index   INTEGER NOT NULL,   -- 0-based position in vectors.bin
    text           TEXT NOT NULL,       -- actual chunk content
    meta           TEXT NOT NULL,       -- JSON blob (see below)
    is_deleted     INTEGER DEFAULT 0   -- soft delete flag
)

-- Indexes
idx_chunks_id      ON chunks(id)
idx_chunks_deleted ON chunks(is_deleted)
```

#### `meta` JSON Structure (Directives)

```json
{
  "id": "directive-uuid",
  "title": "Directive Title",
  "chunk_index": 0,
  "total_chunks": 3,
  "source_type": "directive"
}
```

The `id` field in meta is the `directive_id` — the key used to group chunks back into a full document.

### System B: `conversation_store` (Relational)

Single SQLite database managed by rusqlite. Owns conversations, turns, rules, and all metadata _about_ retrieved context.

---

## Cross-System Reference Map

```
conversation_store                          ehl_v13_rs
==================                          ==========

┌─────────────────────┐
│   rule_libraries    │
│─────────────────────│
│ id                  │
│ name, version, ...  │
│ content_source_id ──┼──┐
│ ehl_doc_id ─────────┼──┼──────────────────► chunks.meta.id
│ ingestion_status    │  │                    (directive_id in JSON)
└─────────────────────┘  │
                         │
┌─────────────────────┐  │
│  content_sources    │◄─┘
│─────────────────────│
│ id (PK)             │
│ source_type         │  file|directive|url|clipboard|conversation|attachment
│ source_path         │
│ source_ref_id       │  → rule_libraries.id or file_attachments.id
│ content_hash        │  SHA-256 for staleness detection
│ ehl_doc_id ─────────┼─────────────────────► chunks.meta.id
│ chunk_count         │                       (count of chunks with this directive_id)
│ ingestion_status    │
│ embedding_model_ver │
└────────┬────────────┘
         │
         │ source_id (FK)
         ▼
┌─────────────────────┐
│  ingestion_queue    │
│─────────────────────│
│ id (PK)             │
│ source_id ──────────┼──► content_sources.id
│ job_type            │  ingest|reingest|delete
│ priority            │
│ status              │  queued|processing|completed|failed|cancelled
│ attempts            │
│ max_attempts        │
└─────────────────────┘

┌─────────────────────┐
│ turn_context_refs   │
│─────────────────────│
│ id (PK)             │
│ turn_id ────────────┼──► conversation_turns.id
│ chunk_id ───────────┼─────────────────────► chunks.vector_index (or chunk identifier)
│ source_id ──────────┼──► content_sources.id
│ relevance_score     │
│ retrieval_query     │
│ position            │  position in context window
│ token_count         │
│ source_system       │  ehl_v13|web_search|mcp_tool|user_provided
│ chunk_status        │  valid|orphaned|expired
│ content_snapshot    │  cached text if chunk orphaned
│ UNIQUE(turn_id,     │
│        chunk_id)    │
└─────────────────────┘

┌──────────────────────────┐
│ conversation_context_    │
│ window                   │
│──────────────────────────│
│ conversation_id ─────────┼──► conversations.id
│ chunk_id ────────────────┼──► same chunk_id as turn_context_refs
│ source_id ───────────────┼──► content_sources.id
│ first_included_turn      │
│ last_included_turn       │
│ consecutive_inclusions   │
│ total_inclusions         │
│ is_sticky                │
│ sticky_until_turn        │
│ user_approved            │
│ initial_score            │
│ current_score            │  decays via: score *= (1 - decay_rate)
│ relevance_decay_rate     │
│ PK(conversation_id,      │
│    chunk_id)             │
└──────────────────────────┘

┌─────────────────────┐
│ retrieval_sessions  │
│─────────────────────│
│ id (PK)             │
│ conversation_id ────┼──► conversations.id
│ turn_id ────────────┼──► conversation_turns.id
│ query_text          │
│ query_hash          │
│ top_k, threshold    │
│ chunks_returned     │
│ chunks_used         │
│ total_tokens        │
│ embedding_latency   │
│ search_latency      │
│ total_latency       │
│ systems_queried     │
└─────────────────────┘

┌─────────────────────┐
│query_embedding_cache│
│─────────────────────│
│ query_hash (PK)     │
│ query_text          │
│ embedding_ref       │
│ embedding_inline    │  BLOB — cached embedding vector
│ model_version       │
│ embedding_dim       │
│ use_count           │
│ last_used_at        │  for LRU eviction
└─────────────────────┘

┌─────────────────────┐
│ file_attachments    │
│─────────────────────│
│ id (PK)             │
│ owner_type          │  turn|rule_library
│ owner_id            │
│ storage_path        │
│ content_source_id ──┼──► content_sources.id  (added via ALTER)
│ auto_ingest         │
│ ingestion_status    │
└─────────────────────┘

┌─────────────────────┐
│   conversations     │
│─────────────────────│
│ id (PK)             │
│ active_context_count│  (added via ALTER)
│ sticky_context_count│  (added via ALTER)
│ last_retrieval_at   │  (added via ALTER)
└─────────────────────┘
```

## Join Keys Between Systems

| conversation_store column    | ehl_v13 equivalent           | Notes                                                     |
| ---------------------------- | ---------------------------- | --------------------------------------------------------- |
| `content_sources.ehl_doc_id` | `chunks.meta` → `"id"` field | Groups all chunks belonging to one ingested document      |
| `turn_context_refs.chunk_id` | `chunks.vector_index`        | Identifies a specific chunk returned from search          |
| `context_window.chunk_id`    | `chunks.vector_index`        | Same chunk_id used in context window tracking             |
| `rule_libraries.ehl_doc_id`  | `chunks.meta` → `"id"` field | Direct shortcut, same value as content_sources.ehl_doc_id |

## Data Flow: Ingestion

```
1. Content registered → content_sources row (status: pending)
2. Job queued        → ingestion_queue row (status: queued)
3. Sidecar called    → ehl_v13 chunks text into chunks table + vectors into vectors.bin
4. Success written   → content_sources.ehl_doc_id, chunk_count updated (status: ingested)
5. Job completed     → ingestion_queue (status: completed)
```

## Data Flow: Retrieval

```
1. Query embedded    → vector created (cached in query_embedding_cache)
2. Similarity search → vectors.bin scanned, top-k vector_indexes returned
3. Metadata fetched  → chunks.text + chunks.meta for those indexes
4. Refs stored       → turn_context_refs rows created (chunk_id = vector_index)
5. Window updated    → conversation_context_window upserted with scores
6. Session logged    → retrieval_sessions row with latency metrics
```

## Data Flow: Orphan Reconciliation

```
1. reconcile_chunks() iterates turn_context_refs with status = 'valid'
2. For each chunk_id, calls sidecar to verify chunk exists
3. If chunk deleted from ehl_v13:
   a. content_snapshot populated with last-known text (if available)
   b. chunk_status set to 'orphaned'
4. Orphaned chunks still usable via snapshot for graceful degradation
```

## Separate Manifolds

ehl_v13 maintains separate index directories for different content types:

| Manifold   | Path                    | Contents                           |
| ---------- | ----------------------- | ---------------------------------- |
| Directives | `ehl_index/directives/` | Rule libraries, system directives  |
| Episodic   | `ehl_index/episodic/`   | Conversation history, user content |

Each has its own `vectors.bin` + `metadata.db` pair. Vector indexes are local to each manifold — chunk_id `0` in directives is unrelated to chunk_id `0` in episodic.
