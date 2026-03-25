CREATE TABLE members (
    id INTEGER PRIMARY KEY,
    member_key TEXT NOT NULL
);
CREATE UNIQUE INDEX uq_members_member_key ON members(member_key);
