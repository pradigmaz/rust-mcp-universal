use crate::utils::hash_bytes;

pub(crate) const CURRENT_ARTIFACT_FINGERPRINT_VERSION: i64 = 1;

#[derive(Debug, Clone, Default)]
pub(crate) struct ArtifactFingerprintBuilder {
    bytes: Vec<u8>,
}

impl ArtifactFingerprintBuilder {
    pub(crate) fn add_chunk_manifest_entry(
        &mut self,
        chunk_hash: &str,
        chunk_idx: i64,
        start_line: Option<i64>,
        end_line: Option<i64>,
        excerpt: &str,
    ) {
        self.push_tag(b'C');
        self.push_text(chunk_hash);
        self.push_i64(chunk_idx);
        self.push_optional_i64(start_line);
        self.push_optional_i64(end_line);
        self.push_line(excerpt);
    }

    pub(crate) fn add_chunk_embedding_entry(
        &mut self,
        chunk_hash: &str,
        chunk_idx: i64,
        dim: i64,
        vector_json: &str,
    ) {
        self.push_tag(b'E');
        self.push_text(chunk_hash);
        self.push_i64(chunk_idx);
        self.push_i64(dim);
        self.push_line(vector_json);
    }

    pub(crate) fn add_semantic_vector(&mut self, dim: i64, vector_json: &str) {
        self.push_tag(b'V');
        self.push_i64(dim);
        self.push_line(vector_json);
    }

    pub(crate) fn add_ann_bucket(&mut self, bucket_family: i64, bucket_key: &str) {
        self.push_tag(b'A');
        self.push_i64(bucket_family);
        self.push_line(bucket_key);
    }

    pub(crate) fn finish(self) -> String {
        hash_bytes(&self.bytes)
    }

    fn push_tag(&mut self, tag: u8) {
        self.bytes.push(tag);
        self.bytes.push(0);
    }

    fn push_text(&mut self, value: &str) {
        self.bytes.extend_from_slice(value.as_bytes());
        self.bytes.push(0);
    }

    fn push_i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(value.to_string().as_bytes());
        self.bytes.push(0);
    }

    fn push_optional_i64(&mut self, value: Option<i64>) {
        if let Some(value) = value {
            self.bytes.extend_from_slice(value.to_string().as_bytes());
        }
        self.bytes.push(0);
    }

    fn push_line(&mut self, value: &str) {
        self.bytes.extend_from_slice(value.as_bytes());
        self.bytes.push(b'\n');
    }
}

pub(crate) fn sample_content_hash(sample: &str) -> String {
    hash_bytes(sample.as_bytes())
}

pub(crate) fn empty_artifact_content_hash() -> String {
    ArtifactFingerprintBuilder::default().finish()
}

pub(crate) fn semantic_vector_content_hash(dim: i64, vector_json: &str) -> String {
    let mut builder = ArtifactFingerprintBuilder::default();
    builder.add_semantic_vector(dim, vector_json);
    builder.finish()
}
