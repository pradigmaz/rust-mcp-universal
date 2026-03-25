CREATE TABLE profiles (
    profile_id TEXT PRIMARY KEY,
    email TEXT NOT NULL
);
CREATE UNIQUE INDEX uq_profiles_profile_id ON profiles(profile_id);
