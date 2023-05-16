use async_trait::async_trait;
use once_cell::sync::Lazy;
use rand::random;
use rust_bert::pipelines::sentence_embeddings::{
    builder::SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};

use super::{Flashcard, PipelineInput, PipelineOutput, PipelineStage};
use crate::spider::{
    google_image::{image_search_max},
    spanish_dict::{search_vocab, DictionaryDefinition, DictionaryExample},
};

struct VisualVocab {
    vocab: Vec<Flashcard>,
}

struct VisualFlashCard {
    word: String,
    definition: String,
    image: Vec<u8>,
    example: String,
}

const IMAGE_RANDOM_POOL_SIZE: u32 = 10;

#[async_trait]
impl PipelineStage for VisualVocab {
    async fn process(&self, _input: PipelineInput) -> Result<Vec<PipelineOutput>, &'static str> {
        let mut result: Vec<VisualFlashCard> = vec![];
        let mut tasks = vec![];
        for vocab in self.vocab.iter() {
            let vocab = vocab.clone();
            let task = tokio::spawn(async move {
                create_visual_vocab(&vocab)
                    .await
                    .expect("should have created a visual flashcard")
            });
            tasks.push(task);
        }
        for task in tasks {
            result.push(task.await.expect("should have awaited task"));
        }
        Ok(vec![])
    }
}

/// Create a visual flashcard
async fn create_visual_vocab(vocab: &Flashcard) -> Result<VisualFlashCard, &'static str> {
    let mut images = image_search_max(&vocab.word, IMAGE_RANDOM_POOL_SIZE)
        .await
        .expect("should have images");

    let definition = search_vocab(&vocab.word)
        .await
        .expect("should have found a definition");

    let mut image: Vec<u8> = vec![];
    let mut flag = false;
    while !images.is_empty() && !flag {
        if let Ok(img) = images
            .remove(random::<usize>() % images.len())
            .get_bytes()
            .await
        {
            image = img;
            flag = true;
        }
    }
    if flag {
        return Err("should have found an image");
    }

    let examples: Vec<(_, _)> = definition
        .definitions
        .iter()
        .filter(|x| {
            matches!(
                x,
                DictionaryDefinition::DefinitionAndGroupWithExample { .. }
            )
        })
        .flat_map(|x| {
            if let DictionaryDefinition::DefinitionAndGroupWithExample {
                group,
                definition,
                examples,
            } = x
            {
                return examples
                    .iter()
                    .map(|x| {
                        let def = format!("{} ({})", definition, group);
                        let example = match x {
                            DictionaryExample::Example { example } => example,
                            DictionaryExample::ExampleAndTranslation {
                                example,
                                translation: _,
                            } => example,
                        };
                        (def, example)
                    })
                    .collect::<Vec<(_, _)>>();
            }
            vec![]
        })
        .collect();

    let definition = examples.iter().map(|x| x.0.to_owned()).collect::<Vec<_>>();
    let rank = deep_search(&vocab.word, &definition, 1, 0.0);
    let example = examples[rank[0].0].1.to_owned();

    Ok(VisualFlashCard {
        word: vocab.word.to_owned(),
        definition: vocab.definition.to_owned(),
        image,
        example,
    })
}

/// Search for a query in a list of strings
/// - `query` is the string to search for
/// - `contents` is the list of strings to search in
/// - `limit` is the maximum number of results to return. If 0, return all results
/// - `threshold` is the minimum similarity score to return a result
/// Return a list ranked by relevance of the results
pub fn deep_search(
    query: &str,
    contents: &Vec<String>,
    limit: usize,
    threshold: f32,
) -> Vec<(usize, f32)> {
    let model = Lazy::new(|| {
        SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL12V2)
            .create_model()
            .expect("should have created a model")
    });
    let query_embedding = model.encode(&[query]).expect("should have encoded query")[0].to_owned();
    let content_embedding = model
        .encode(contents)
        .expect("should have encoded contents");
    let similarities = content_embedding
        .iter()
        .map(|x| cos_similarity(&query_embedding, x))
        .collect::<Vec<f32>>();
    let mut results = similarities
        .iter()
        .enumerate()
        .filter_map(|x| {
            if *x.1 > threshold {
                Some((x.0, *x.1))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    if limit == 0 {
        results
    } else {
        results[0..limit].to_vec()
    }
}

fn cos_similarity(a: &Vec<f32>, b: &Vec<f32>) -> f32 {
    let mut dot_product = 0.0;
    let mut a_norm = 0.0;
    let mut b_norm = 0.0;
    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        a_norm += a[i] * a[i];
        b_norm += b[i] * b[i];
    }
    dot_product / (a_norm * b_norm).sqrt()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rust_bert() {
        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL12V2)
            .create_model()
            .expect("should have created a model");
        let sentences = ["this is an example sentence", "each sentence is converted"];
        let output = model
            .encode(&sentences)
            .expect("should have encoded sentences");
        println!("{:?}", output);
    }

    #[test]
    fn test_deep_search() {
        let query = "this is an example sentence";
        let contents = [
            "this example sentence is the first sentence".to_string(),
            "each sentence is converted".to_string(),
            "this is a different sentence".to_string(),
        ];
        let results = deep_search(query, &contents.to_vec(), 0, 0.0);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 0);
    }
}
