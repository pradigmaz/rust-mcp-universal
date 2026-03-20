use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

const ENV_BACKEND: &str = "RMU_EMBED_BACKEND";
const ENV_TRANSFORMER_CMD: &str = "RMU_TRANSFORMER_EMBED_CMD";
const ENV_TRANSFORMER_ARGS: &str = "RMU_TRANSFORMER_EMBED_ARGS";
const ENV_TRANSFORMER_ARGS_JSON: &str = "RMU_TRANSFORMER_EMBED_ARGS_JSON";
const ENV_TRANSFORMER_MODEL: &str = "RMU_TRANSFORMER_MODEL";

const BACKEND_LOCAL_DENSE: &str = "local_dense";
const BACKEND_TRANSFORMER: &str = "transformer";
const DEFAULT_LOCAL_MODEL: &str = "rmu-local-dense-v1";
const DEFAULT_TRANSFORMER_MODEL: &str = "rmu-transformer-hybrid-v1";

#[derive(Debug, Clone)]
pub enum EmbeddingBackend {
    LocalDense,
    Transformer(TransformerBackendConfig),
}

#[derive(Debug, Clone)]
pub struct TransformerBackendConfig {
    pub command: String,
    pub args: Vec<String>,
    pub model_name: String,
}

pub fn active_backend() -> EmbeddingBackend {
    let requested = std::env::var(ENV_BACKEND)
        .unwrap_or_else(|_| BACKEND_LOCAL_DENSE.to_string())
        .trim()
        .to_ascii_lowercase();

    if requested != BACKEND_TRANSFORMER {
        return EmbeddingBackend::LocalDense;
    }

    let command = std::env::var(ENV_TRANSFORMER_CMD)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let Some(command) = command else {
        return EmbeddingBackend::LocalDense;
    };

    let args = transformer_args_from_env();

    let model_name = std::env::var(ENV_TRANSFORMER_MODEL)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_TRANSFORMER_MODEL.to_string());

    EmbeddingBackend::Transformer(TransformerBackendConfig {
        command,
        args,
        model_name,
    })
}

pub fn semantic_model_name() -> String {
    match active_backend() {
        EmbeddingBackend::LocalDense => DEFAULT_LOCAL_MODEL.to_string(),
        EmbeddingBackend::Transformer(cfg) => cfg.model_name,
    }
}

pub fn transformer_embedding(config: &TransformerBackendConfig, text: &str) -> Result<Vec<f32>> {
    let mut child = Command::new(&config.command)
        .args(&config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn transformer embedding command `{}`",
                config.command
            )
        })?;

    if let Some(stdin) = child.stdin.as_mut() {
        let payload = serde_json::to_vec(&json!({ "text": text }))?;
        stdin
            .write_all(&payload)
            .context("failed to write transformer embedding request to stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("failed to wait transformer embedding command")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "transformer embedding command exited with {}: {}",
            output.status,
            stderr.trim()
        );
    }

    parse_embedding_response(&output.stdout)
}

fn parse_embedding_response(bytes: &[u8]) -> Result<Vec<f32>> {
    let value: Value =
        serde_json::from_slice(bytes).context("transformer embedding output is not valid JSON")?;

    if let Some(embedding) = value_to_embedding(&value)? {
        return Ok(embedding);
    }
    if let Some(embedding) = value
        .get("embedding")
        .map(value_to_embedding)
        .transpose()?
        .flatten()
    {
        return Ok(embedding);
    }
    if let Some(embedding) = value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("embedding"))
        .map(value_to_embedding)
        .transpose()?
        .flatten()
    {
        return Ok(embedding);
    }

    bail!("transformer embedding output does not contain `embedding` array");
}

fn value_to_embedding(value: &Value) -> Result<Option<Vec<f32>>> {
    let Some(arr) = value.as_array() else {
        return Ok(None);
    };
    if arr.is_empty() {
        bail!("embedding array must not be empty");
    }

    let mut out = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
        let Some(v) = item.as_f64() else {
            bail!("embedding[{idx}] must be a number");
        };
        if !v.is_finite() {
            bail!("embedding[{idx}] must be a finite number");
        }
        out.push(v as f32);
    }
    Ok(Some(out))
}

fn transformer_args_from_env() -> Vec<String> {
    if let Some(parsed) = std::env::var(ENV_TRANSFORMER_ARGS_JSON)
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
    {
        return parsed;
    }

    std::env::var(ENV_TRANSFORMER_ARGS)
        .ok()
        .map(|v| {
            v.split_whitespace()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{EmbeddingBackend, active_backend, parse_embedding_response};

    #[test]
    fn default_backend_is_local_dense() {
        // No env setup in test should stay backward compatible.
        assert!(matches!(active_backend(), EmbeddingBackend::LocalDense));
    }

    #[test]
    fn parse_embedding_response_rejects_non_numeric_elements() {
        let raw = br#"{"embedding":[0.1,"oops",0.3]}"#;
        let err = parse_embedding_response(raw).expect_err("mixed-type embedding must fail");
        assert!(
            err.to_string().contains("embedding[1] must be a number"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_embedding_response_accepts_valid_numeric_array() {
        let raw = br#"{"embedding":[0.1,0.2,0.3]}"#;
        let vec = parse_embedding_response(raw).expect("valid embedding should parse");
        assert_eq!(vec.len(), 3);
    }
}
