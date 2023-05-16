use std::io::Cursor;

use async_trait::async_trait;
use docx_rs::*;
use once_cell::sync::Lazy;
use rand::random;
use rust_bert::pipelines::sentence_embeddings::{
    builder::SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};

use super::{get_or_default, Flashcard, PipelineInput, PipelineOutput, PipelineStage};
use crate::spider::{
    google_image::image_search_max,
    spanish_dict::{search_vocab, DictionaryDefinition, DictionaryExample},
};
use log::info;

struct VisualVocabPipeline {
    vocab: Vec<Flashcard>,
}

struct VisualFlashCard {
    word: String,
    definition: String,
    image: Vec<u8>,
    example: String,
}

impl VisualFlashCard {
    /// Return a table with the visual flashcard
    /// |-------------------------|
    /// | Vocabulario: word       |
    /// |-------------------------|
    /// | Frase Completa: example |
    /// |-------------------------|
    /// | Foto / Media: image     |
    /// |-------------------------|
    pub fn to_table(&self) -> Table {
        Table::new(vec![
            TableRow::new(vec![TableCell::new()
                .add_paragraph(Paragraph::new().add_run(
                    Run::new().add_text(&format!("Vocabulario: {}", self.word)),
                ))]),
            TableRow::new(vec![TableCell::new()
                .add_paragraph(Paragraph::new().add_run(
                    Run::new().add_text(&format!("Frase Completa: {}", self.example)),
                ))]),
            TableRow::new(vec![TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_image(Pic::new(&self.image))),
            )]),
        ])
    }

    /// Return a table with the visual flashcards
    /// |-------------------------|-------------------------|-------------------------|
    /// | Vocabulario: word       | Vocabulario: word       | Vocabulario: word       |
    /// |-------------------------|-------------------------|-------------------------|
    /// | Frase Completa: example | Frase Completa: example | Frase Completa: example |
    /// |-------------------------|-------------------------|-------------------------|
    /// | Foto / Media: image     | Foto / Media: image     | Foto / Media: image     |
    /// |-------------------------|-------------------------|-------------------------|
    pub fn to_tables(vocabs: &[VisualFlashCard]) -> Table {
        Table::new(vec![
            TableRow::new(
                vocabs
                    .iter()
                    .map(|x| {
                        TableCell::new().add_paragraph(
                            Paragraph::new()
                                .add_run(Run::new().add_text(&format!("Vocabulario: {}", x.word))),
                        )
                    })
                    .collect(),
            ),
            TableRow::new(
                vocabs
                    .iter()
                    .map(|x| {
                        TableCell::new().add_paragraph(Paragraph::new().add_run(
                            Run::new().add_text(&format!("Frase Completa: {}", x.example)),
                        ))
                    })
                    .collect(),
            ),
            TableRow::new(
                vocabs
                    .iter()
                    .map(|x| {
                        TableCell::new().add_paragraph(
                            Paragraph::new().add_run(Run::new().add_image(Pic::new(&x.image))),
                        )
                    })
                    .collect(),
            ),
        ])
    }
}

const IMAGE_RANDOM_POOL_SIZE: u32 = 10;

#[async_trait]
impl PipelineStage for VisualVocabPipeline {
    async fn process(&self, input: PipelineInput) -> Result<Vec<PipelineOutput>, &'static str> {
        // parse input
        let row: usize = get_or_default(&input, "row", 6);
        let col: usize = get_or_default(&input, "col", 3);
        let name: String = get_or_default(&input, "name", "Estudiante".to_string());
        let period: String = get_or_default(&input, "period", "Segunda".to_string());
        let filename: String = get_or_default(&input, "filename", "visual_vocab.docx".to_string());

        // pick random words
        let mut words = self.vocab.clone();
        let mut result: Vec<Flashcard> = vec![];
        for _ in 0..row * col {
            let word = words.remove(random::<usize>() % words.len());
            result.push(word);
        }

        // create visual flashcards
        info!(target: "visual_vocab", "Creating visual flashcards");
        let vocabs = create_visual_vocabs(words.as_slice())
            .await
            .expect("should have created visual flashcards");

        // create document
        info!(target: "visual_vocab", "Creating document");
        let mut docx = Docx::new();
        docx = docx
            .header(
                Header::new().add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text(&format!("Nombre: {}", name))
                            .add_tab()
                            .add_text(&format!("Hora: {}", period)),
                    ),
                ),
            ).add_paragraph(
                Paragraph::new().add_run(Run::new()
                    .add_text("Escoge 18 palabras del vocabulario de esta unidad.")
                    .add_break(BreakType::TextWrapping)
                    .add_text("Escribe la palabra de vocabulario y una frase completa con la palabra. Dibuja una foto que representa la palabra."))
            );
        for i in 1..row {
            let table = VisualFlashCard::to_tables(&vocabs[(i - 1) * col..i * col]);
            docx = docx.add_table(table);
        }

        // save document
        let mut buffer = Cursor::new(Vec::new());
        docx.build()
            .pack(&mut buffer)
            .expect("should have built document");
        Ok(vec![PipelineOutput::Document {
            name: filename,
            content: buffer.into_inner(),
        }])
    }
}

/// Create visual flashcards
async fn create_visual_vocabs(vocabs: &[Flashcard]) -> Result<Vec<VisualFlashCard>, &'static str> {
    let mut result: Vec<VisualFlashCard> = vec![];
    let mut tasks = vec![];
    for vocab in vocabs.iter() {
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
    Ok(result)
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
    contents: &[String],
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

fn cos_similarity(a: &[f32], b: &[f32]) -> f32 {
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

/// Convert cm to English metric unit
fn cm(cm: f32) -> usize {
    (cm * 360_000.0) as usize
}

/// Convert point to English metric unit
fn pixel(point: f32) -> usize {
    (point * 9525.0) as usize
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
        let results = deep_search(query, contents.as_ref(), 0, 0.0);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 0);
    }
}
