use serde_json::{Value, json};

mod benchmark;
mod indexing;
mod maintenance;
mod navigation;
mod project;
mod quality;
mod search;

pub(super) fn tools_list() -> Value {
    let mut tools = Vec::new();
    tools.extend(project::tools());
    tools.extend(search::tools());
    tools.extend(indexing::tools());
    tools.extend(maintenance::tools());
    tools.extend(navigation::tools());
    tools.extend(quality::tools());
    tools.extend(benchmark::tools());
    json!({ "tools": tools })
}
