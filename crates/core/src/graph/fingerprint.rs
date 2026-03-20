use crate::utils::hash_bytes;

use super::{GraphExtraction, GraphRef, GraphSymbol};

#[derive(Debug, Clone, Default)]
pub(crate) struct GraphFingerprintBuilder {
    bytes: Vec<u8>,
}

impl GraphFingerprintBuilder {
    pub(crate) fn add_symbol(
        &mut self,
        name: &str,
        kind: &str,
        line: Option<i64>,
        column: Option<i64>,
        language: &str,
    ) {
        self.push_tag(b'S');
        self.push_text(name);
        self.push_text(kind);
        self.push_optional_i64(line);
        self.push_optional_i64(column);
        self.push_line(language);
    }

    pub(crate) fn add_dep(&mut self, dep: &str, language: &str) {
        self.push_tag(b'D');
        self.push_text(dep);
        self.push_line(language);
    }

    pub(crate) fn add_ref(
        &mut self,
        symbol: &str,
        line: Option<i64>,
        column: Option<i64>,
        language: &str,
    ) {
        self.push_tag(b'R');
        self.push_text(symbol);
        self.push_optional_i64(line);
        self.push_optional_i64(column);
        self.push_line(language);
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

pub(crate) fn empty_graph_content_hash() -> String {
    GraphFingerprintBuilder::default().finish()
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GraphEdgeFingerprintBuilder {
    bytes: Vec<u8>,
}

impl GraphEdgeFingerprintBuilder {
    pub(crate) fn add_outgoing(
        &mut self,
        src_path: &str,
        dst_path: &str,
        edge_kind: &str,
        raw_count: i64,
        weight: f32,
    ) {
        self.push_tag(b'O');
        self.push_text(src_path);
        self.push_text(dst_path);
        self.push_text(edge_kind);
        self.push_i64(raw_count);
        self.push_f32(weight);
    }

    pub(crate) fn add_incoming(
        &mut self,
        src_path: &str,
        dst_path: &str,
        edge_kind: &str,
        raw_count: i64,
        weight: f32,
    ) {
        self.push_tag(b'I');
        self.push_text(src_path);
        self.push_text(dst_path);
        self.push_text(edge_kind);
        self.push_i64(raw_count);
        self.push_f32(weight);
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

    fn push_f32(&mut self, value: f32) {
        self.bytes
            .extend_from_slice(value.to_bits().to_string().as_bytes());
        self.bytes.push(0);
    }
}

pub(crate) fn empty_graph_edge_content_hash() -> String {
    GraphEdgeFingerprintBuilder::default().finish()
}

pub(crate) fn graph_content_hash(language: &str, graph: &GraphExtraction) -> String {
    let mut builder = GraphFingerprintBuilder::default();

    let mut symbols = graph.symbols.clone();
    symbols.sort_by(|left, right| {
        (&left.name, &left.kind, left.line, left.column).cmp(&(
            &right.name,
            &right.kind,
            right.line,
            right.column,
        ))
    });
    for symbol in &symbols {
        add_symbol(&mut builder, symbol, language);
    }

    let mut deps = graph.deps.clone();
    deps.sort();
    for dep in &deps {
        builder.add_dep(dep, language);
    }

    let mut refs = graph.refs.clone();
    refs.sort_by(|left, right| {
        (&left.symbol, left.line, left.column).cmp(&(&right.symbol, right.line, right.column))
    });
    for graph_ref in &refs {
        add_ref(&mut builder, graph_ref, language);
    }

    builder.finish()
}

fn add_symbol(builder: &mut GraphFingerprintBuilder, symbol: &GraphSymbol, language: &str) {
    builder.add_symbol(
        &symbol.name,
        &symbol.kind,
        symbol
            .line
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        symbol
            .column
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        language,
    );
}

fn add_ref(builder: &mut GraphFingerprintBuilder, graph_ref: &GraphRef, language: &str) {
    builder.add_ref(
        &graph_ref.symbol,
        graph_ref
            .line
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        graph_ref
            .column
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        language,
    );
}
