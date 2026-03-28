use super::normalize_tokens;

#[test]
fn normalize_tokens_collapse_rust_attributes_and_macros() {
    let tokens = normalize_tokens(
        "rust",
        "#[derive(Clone)]\npub fn alpha() { tracing::info!(\"hi\"); }\n",
    );
    let values = tokens
        .into_iter()
        .map(|token| token.value)
        .collect::<Vec<_>>();
    assert!(values.contains(&"$attr".to_string()));
    assert!(values.contains(&"$macro".to_string()));
}

#[test]
fn normalize_tokens_collapse_python_and_java_annotations() {
    let python = normalize_tokens("python", "@dataclass\nclass User:\n    pass\n");
    let java = normalize_tokens("java", "@Entity\npublic class User {}\n");
    assert_eq!(python[0].value, "$attr");
    assert_eq!(java[0].value, "$attr");
}

#[test]
fn normalize_tokens_handle_rust_crate_attributes_and_lifetimes() {
    let tokens = normalize_tokens(
        "rust",
        "#![allow(dead_code)]\nfn alpha<'a>(value: &'a str) -> &'a str { value }\n",
    );
    let values = tokens
        .into_iter()
        .map(|token| token.value)
        .collect::<Vec<_>>();
    assert!(values.contains(&"$attr".to_string()));
    assert!(values.contains(&"$lifetime".to_string()));
}

#[test]
fn normalize_tokens_emit_jsx_closing_tokens_and_framework_markers() {
    let tokens = normalize_tokens(
        "tsx",
        "export function CardShell({ children }: Props) { return <Card><Body>{children}</Body></Card> }\n",
    );
    let values = tokens
        .into_iter()
        .map(|token| token.value)
        .collect::<Vec<_>>();
    assert!(values.contains(&"</".to_string()));
    assert!(values.contains(&"children".to_string()));
}
