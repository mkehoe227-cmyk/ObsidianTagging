use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;

const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";
const MAX_TOKENS: usize = 256;

pub async fn embed_text(text: &str) -> Result<Vec<f32>> {
    let device = Device::Cpu;

    let api = Api::new().context("Failed to create hf-hub API")?;
    let repo = api.repo(Repo::new(MODEL_ID.to_string(), RepoType::Model));
    let config_path = repo.get("config.json").await.context("download config.json")?;
    let tokenizer_path = repo.get("tokenizer.json").await.context("download tokenizer.json")?;
    let weights_path = repo
        .get("model.safetensors")
        .await
        .context("download model.safetensors")?;

    let tokenizer = Tokenizer::from_file(tokenizer_path)
        .map_err(|e| anyhow::anyhow!("tokenizer load error: {}", e))?;

    let config: BertConfig =
        serde_json::from_reader(std::fs::File::open(config_path)?)?;

    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)?
    };
    let model = BertModel::load(vb, &config)?;

    let encoding = tokenizer
        .encode(text, true)
        .map_err(|e| anyhow::anyhow!("tokenize error: {}", e))?;

    let len = MAX_TOKENS.min(encoding.get_ids().len());
    let ids: Vec<u32> = encoding.get_ids()[..len].to_vec();
    let mask: Vec<u32> = encoding.get_attention_mask()[..len].to_vec();
    let type_ids: Vec<u32> = vec![0u32; len];

    let input_ids = Tensor::new(ids.as_slice(), &device)?.unsqueeze(0)?;
    let attention_mask = Tensor::new(mask.as_slice(), &device)?.unsqueeze(0)?;
    let token_type_ids = Tensor::new(type_ids.as_slice(), &device)?.unsqueeze(0)?;

    // forward returns [batch=1, seq_len, hidden=384]
    let hidden = model.forward(&input_ids, &token_type_ids, Some(&attention_mask))?;

    // mean pooling: sum(hidden * mask) / sum(mask)
    let mask_f32 = attention_mask.to_dtype(DType::F32)?.unsqueeze(2)?;
    let sum = hidden.broadcast_mul(&mask_f32)?.sum(1)?;
    let count = mask_f32.sum(1)?;
    // mean: [1, 384] → squeeze to [384]
    let mean = sum.broadcast_div(&count)?.squeeze(0)?;

    // L2 normalize: scale by 1/||v||
    let norm_sq: f32 = mean.sqr()?.sum_all()?.to_scalar::<f32>()?;
    let normalized = mean.affine((1.0 / norm_sq.sqrt()) as f64, 0.0)?;

    Ok(normalized.to_vec1::<f32>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embed_produces_384_dim_vector() {
        let v = embed_text("The Zettelkasten method is a note-taking system.")
            .await
            .unwrap();
        assert_eq!(v.len(), 384, "MiniLM-L6-v2 must produce 384-dim embeddings");
    }

    #[tokio::test]
    async fn test_embed_is_normalized() {
        let v = embed_text("Rust ownership model prevents memory bugs.")
            .await
            .unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "not normalized, norm={}", norm);
    }

    #[tokio::test]
    async fn test_similar_texts_closer_than_dissimilar() {
        let a = embed_text("knowledge management and note taking systems")
            .await
            .unwrap();
        let b = embed_text("Zettelkasten is a personal knowledge management method")
            .await
            .unwrap();
        let c = embed_text("Rust borrow checker prevents data races at compile time")
            .await
            .unwrap();
        let sim_ab: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let sim_ac: f32 = a.iter().zip(c.iter()).map(|(x, y)| x * y).sum();
        assert!(
            sim_ab > sim_ac,
            "similar texts must score higher: ab={:.3} ac={:.3}",
            sim_ab,
            sim_ac
        );
    }
}
