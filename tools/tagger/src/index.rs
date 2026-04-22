use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

const DIMS: usize = 384;
const BYTES_PER_VEC: usize = DIMS * 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: usize,
    pub path: String,
    pub title: String,
    pub path_hash: String,
}

pub fn load_manifest(index_dir: &str) -> Result<Vec<Entry>> {
    let manifest_path = Path::new(index_dir).join("manifest.json");
    if !manifest_path.exists() {
        return Ok(vec![]);
    }
    let f = File::open(&manifest_path)?;
    Ok(serde_json::from_reader(BufReader::new(f))?)
}

fn save_manifest(index_dir: &str, manifest: &[Entry]) -> Result<()> {
    let manifest_path = Path::new(index_dir).join("manifest.json");
    let f = File::create(&manifest_path)?;
    serde_json::to_writer_pretty(BufWriter::new(f), manifest)?;
    Ok(())
}

fn path_hash(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hex::encode(&hasher.finalize()[..8])
}

fn read_vector(index_dir: &str, row: usize) -> Result<Vec<f32>> {
    let vectors_path = Path::new(index_dir).join("vectors.bin");
    let mut f = File::open(&vectors_path)
        .with_context(|| format!("Cannot open vectors.bin in {}", index_dir))?;
    f.seek(SeekFrom::Start((row * BYTES_PER_VEC) as u64))?;
    let mut buf = vec![0u8; BYTES_PER_VEC];
    f.read_exact(&mut buf)?;
    Ok(buf
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect())
}

fn write_vector(index_dir: &str, row: usize, vector: &[f32]) -> Result<()> {
    let vectors_path = Path::new(index_dir).join("vectors.bin");
    let mut f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&vectors_path)?;
    f.seek(SeekFrom::Start((row * BYTES_PER_VEC) as u64))?;
    let bytes: Vec<u8> = vector.iter().flat_map(|v| v.to_le_bytes()).collect();
    f.write_all(&bytes)?;
    Ok(())
}

pub fn upsert_and_search(
    index_dir: &str,
    path: &str,
    title: &str,
    embedding: &[f32],
    top_k: usize,
) -> Result<Vec<String>> {
    fs::create_dir_all(index_dir)
        .with_context(|| format!("Cannot create index dir: {}", index_dir))?;

    let mut manifest = load_manifest(index_dir)?;
    let hash = path_hash(path);

    let row = if let Some(entry) = manifest.iter_mut().find(|e| e.path_hash == hash) {
        let r = entry.id;
        entry.title = title.to_string();
        entry.path = path.to_string();
        r
    } else {
        let new_row = manifest.len();
        manifest.push(Entry {
            id: new_row,
            path: path.to_string(),
            title: title.to_string(),
            path_hash: hash.clone(),
        });
        new_row
    };

    write_vector(index_dir, row, embedding)?;
    save_manifest(index_dir, &manifest)?;

    let n = manifest.len();
    if n <= 1 {
        return Ok(vec![]);
    }

    let mut scores: Vec<(f32, usize)> = (0..n)
        .filter(|&r| r != row)
        .map(|r| {
            let v = read_vector(index_dir, r).unwrap_or_else(|_| vec![0.0; DIMS]);
            let score: f32 = embedding.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
            (score, r)
        })
        .collect();

    scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    let k = top_k.min(scores.len());
    Ok(scores[..k]
        .iter()
        .map(|(_, r)| manifest[*r].title.clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn dummy_vec(seed: f32) -> Vec<f32> {
        let mut v: Vec<f32> = (0..384).map(|i| (i as f32 * seed).sin()).collect();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter_mut().for_each(|x| *x /= norm);
        v
    }

    #[test]
    fn test_upsert_new_entry_appends() {
        let dir = TempDir::new().unwrap();
        let index_dir = dir.path().to_str().unwrap();
        upsert_and_search(index_dir, "notes/a.md", "Note A", &dummy_vec(1.0), 5).unwrap();
        let manifest = load_manifest(index_dir).unwrap();
        assert_eq!(manifest.len(), 1);
        assert_eq!(manifest[0].title, "Note A");
    }

    #[test]
    fn test_upsert_existing_does_not_duplicate() {
        let dir = TempDir::new().unwrap();
        let index_dir = dir.path().to_str().unwrap();
        upsert_and_search(index_dir, "notes/a.md", "Note A v1", &dummy_vec(1.0), 5).unwrap();
        upsert_and_search(index_dir, "notes/a.md", "Note A v2", &dummy_vec(2.0), 5).unwrap();
        let manifest = load_manifest(index_dir).unwrap();
        assert_eq!(manifest.len(), 1, "upsert must not duplicate");
        assert_eq!(manifest[0].title, "Note A v2", "title must update");
    }

    #[test]
    fn test_cosine_search_returns_most_similar() {
        let dir = TempDir::new().unwrap();
        let index_dir = dir.path().to_str().unwrap();
        upsert_and_search(index_dir, "notes/b.md", "Similar Note", &dummy_vec(1.001), 5).unwrap();
        upsert_and_search(index_dir, "notes/c.md", "Dissimilar Note", &dummy_vec(99.0), 5)
            .unwrap();
        let results =
            upsert_and_search(index_dir, "notes/a.md", "Query Note", &dummy_vec(1.0), 2).unwrap();
        assert!(!results.contains(&"Query Note".to_string()));
        assert_eq!(results[0], "Similar Note");
    }

    #[test]
    fn test_top_k_respected() {
        let dir = TempDir::new().unwrap();
        let index_dir = dir.path().to_str().unwrap();
        for i in 0..20usize {
            upsert_and_search(
                index_dir,
                &format!("notes/n{}.md", i),
                &format!("Note {}", i),
                &dummy_vec(i as f32 + 0.1),
                1,
            )
            .unwrap();
        }
        let results =
            upsert_and_search(index_dir, "notes/q.md", "Query", &dummy_vec(0.5), 5).unwrap();
        assert_eq!(results.len(), 5);
    }
}
