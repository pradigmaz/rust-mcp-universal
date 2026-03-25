CREATE TABLE member_archive (
    id INTEGER PRIMARY KEY,
    external_key TEXT NOT NULL
);
CREATE UNIQUE INDEX uq_member_archive_external_key ON member_archive(external_key);
