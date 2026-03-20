pub(super) const INIT_DB_SCHEMA_SQL: &str = r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;

            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                sha256 TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                language TEXT NOT NULL,
                sample TEXT NOT NULL,
                indexed_at_utc TEXT NOT NULL,
                source_mtime_unix_ms INTEGER,
                artifact_fingerprint_version INTEGER,
                fts_sample_hash TEXT,
                chunk_manifest_count INTEGER,
                chunk_manifest_hash TEXT,
                chunk_embedding_count INTEGER,
                chunk_embedding_hash TEXT,
                semantic_vector_hash TEXT,
                ann_bucket_count INTEGER,
                ann_bucket_hash TEXT,
                graph_symbol_count INTEGER,
                graph_ref_count INTEGER,
                graph_module_dep_count INTEGER,
                graph_content_hash TEXT,
                graph_fingerprint_version INTEGER,
                graph_edge_out_count INTEGER,
                graph_edge_in_count INTEGER,
                graph_edge_hash TEXT,
                graph_edge_fingerprint_version INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_files_language ON files(language);

            CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                path,
                sample
            );

            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                language TEXT NOT NULL,
                line INTEGER,
                column INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_path ON symbols(path);

            CREATE TABLE IF NOT EXISTS module_deps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                dep TEXT NOT NULL,
                language TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_module_deps_dep ON module_deps(dep);
            CREATE INDEX IF NOT EXISTS idx_module_deps_path ON module_deps(path);

            CREATE TABLE IF NOT EXISTS refs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                symbol TEXT NOT NULL,
                language TEXT NOT NULL,
                line INTEGER,
                column INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_refs_symbol ON refs(symbol);
            CREATE INDEX IF NOT EXISTS idx_refs_path ON refs(path);

            CREATE TABLE IF NOT EXISTS file_graph_edges (
                src_path TEXT NOT NULL,
                dst_path TEXT NOT NULL,
                edge_kind TEXT NOT NULL,
                raw_count INTEGER NOT NULL,
                weight REAL NOT NULL,
                PRIMARY KEY(src_path, dst_path, edge_kind)
            );
            CREATE INDEX IF NOT EXISTS idx_file_graph_edges_src ON file_graph_edges(src_path);
            CREATE INDEX IF NOT EXISTS idx_file_graph_edges_dst ON file_graph_edges(dst_path);

            CREATE TABLE IF NOT EXISTS semantic_vectors (
                path TEXT PRIMARY KEY,
                model TEXT NOT NULL,
                dim INTEGER NOT NULL,
                vector_json TEXT NOT NULL,
                indexed_at_utc TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_semantic_vectors_model ON semantic_vectors(model);

            CREATE TABLE IF NOT EXISTS file_chunks (
                path TEXT NOT NULL,
                chunk_hash TEXT NOT NULL,
                chunk_idx INTEGER NOT NULL,
                start_line INTEGER,
                end_line INTEGER,
                PRIMARY KEY(path, chunk_idx)
            );
            CREATE INDEX IF NOT EXISTS idx_file_chunks_chunk_hash ON file_chunks(chunk_hash);
            CREATE INDEX IF NOT EXISTS idx_file_chunks_path ON file_chunks(path);

            CREATE TABLE IF NOT EXISTS chunk_embeddings (
                chunk_hash TEXT NOT NULL,
                model TEXT NOT NULL,
                dim INTEGER NOT NULL,
                vector_json TEXT NOT NULL,
                created_at_utc TEXT NOT NULL,
                PRIMARY KEY(chunk_hash, model)
            );
            CREATE INDEX IF NOT EXISTS idx_chunk_embeddings_model ON chunk_embeddings(model);

            CREATE TABLE IF NOT EXISTS model_metadata (
                model TEXT PRIMARY KEY,
                dim INTEGER NOT NULL,
                updated_at_utc TEXT NOT NULL
            );
            "#;

pub(super) const OPEN_DB_PRAGMAS_SQL: &str = r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            "#;

pub(super) const REQUIRED_SCHEMA_TABLES: [&str; 11] = [
    "meta",
    "files",
    "files_fts",
    "symbols",
    "module_deps",
    "refs",
    "file_graph_edges",
    "semantic_vectors",
    "file_chunks",
    "chunk_embeddings",
    "model_metadata",
];
