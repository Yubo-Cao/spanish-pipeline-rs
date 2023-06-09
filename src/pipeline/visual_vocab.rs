use std::io::Cursor;

use async_trait::async_trait;
use clap::Parser;
use docx_rs::*;
use image::{DynamicImage, GenericImageView};
use log::{debug, error, info};
use rand::random;
use rust_bert::pipelines::sentence_embeddings::{
    builder::SentenceEmbeddingsBuilder, SentenceEmbeddingsModel,
    SentenceEmbeddingsModelType,
};
use tokio::{
    sync::{Mutex, OnceCell},
    task,
};

use super::{Flashcard, Pipeline, PipelineError, PipelineIO};
use crate::{
    error::CliError,
    spider::{
        google_image::image_search_max,
        spanish_dict::{search_vocab, DictionaryDefinition, DictionaryExample},
    },
};

/// A pipeline for making visual vocab
#[derive(Debug, Parser)]
pub struct VisualVocabPipeline {
    /// The number of rows
    #[clap(short, long, default_value = "3")]
    row: u32,
    /// The number of columns
    #[clap(short, long, default_value = "6")]
    col: u32,
    /// The name of the output file
    #[clap(short, long, default_value = "visual_vocab.docx")]
    filename: String,
    /// The name of the student
    name: String,
    /// The period of the student
    period: String,
}

/// A representation of the results created by VisualVocabPipeline
#[derive(Debug, Clone)]
pub struct VisualFlashCard {
    pub word: String,
    pub definition: String,
    pub image: DynamicImage,
    pub example: String,
}

impl std::fmt::Display for VisualFlashCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} - {} ({}, {} bytes)",
            self.word,
            self.definition,
            self.example,
            self.image.dimensions().0 * self.image.dimensions().1 * 3
        )
    }
}

impl VisualFlashCard {
    /// Return a table with the visual flashcards
    /// ```md
    /// |-------------------------|-------------------------|-------------------------|
    /// | Vocabulario: word       | Vocabulario: word       | Vocabulario: word       |
    /// |-------------------------|-------------------------|-------------------------|
    /// | Frase Completa: example | Frase Completa: example | Frase Completa: example |
    /// |-------------------------|-------------------------|-------------------------|
    /// | Foto / Media: at image     | Foto / Media: image     | Foto / Media: image     |
    /// |-------------------------|-------------------------|-------------------------|
    /// ```
    ///
    /// Size should be specified as (width, height) in emu
    async fn to_table(
        vocabs: Vec<VisualFlashCard>,
        size: (u32, u32),
    ) -> Result<Table, Box<dyn std::error::Error>> {
        info!(target: "visual_vocab", "Creating table for {} vocabs with size {:?}", vocabs.len(), size);
        let mut images = Vec::new();

        for vocab in &vocabs {
            let (t_w_emu, t_h_emu) =
                (size.0 / vocabs.len() as u32, size.1 - super::docx::cm(0.5));
            let (w_emu, h_emu) = Pic::new(&vocab.get_image_buf()?).size;
            let ratio = f32::min(
                t_w_emu as f32 / w_emu as f32,
                t_h_emu as f32 / h_emu as f32,
            );
            let (f_w_emu, f_h_emu) =
                ((w_emu as f32 * ratio) as u32, (h_emu as f32 * ratio) as u32);
            let (w_px, h_px) = vocab.image.dimensions();
            let (f_w_px, f_h_px) =
                ((h_px as f32 * ratio) as u32, (w_px as f32 * ratio) as u32);

            info!(target: "visual_vocab", "Resizing image from {}x{} to {}x{}", w_px, h_px, f_w_px, f_h_px);
            let mut buffer = Cursor::new(Vec::new());
            let resized = vocab.image.resize_exact(
                f_w_px,
                f_h_px,
                image::imageops::FilterType::Lanczos3,
            );
            resized.write_to(&mut buffer, image::ImageOutputFormat::Png)?;

            info!(target: "visual_vocab", "Adding image ({}, {})", f_w_emu, f_h_emu);
            images.push(TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_image(
                    Pic::new(&buffer.into_inner()).size(f_w_emu, f_h_emu),
                )),
            ))
        }

        let cellify = |x: String| {
            let mut cell = TableCell::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(x)),
            );
            cell.property = cell.property.width(
                size.0 as usize / vocabs.len() / 12_700 * 12,
                WidthType::Dxa,
            );
            cell
        };

        Ok(Table::new(vec![
            TableRow::new(
                vocabs
                    .iter()
                    .map(|x| cellify(format!("Vocabulario: {}", x.word)))
                    .collect(),
            ),
            TableRow::new(
                vocabs
                    .iter()
                    .map(|x| cellify(format!("Frase Completa: {}", x.example)))
                    .collect(),
            ),
            TableRow::new(images),
        ]))
    }

    fn default() -> Self {
        Self {
            word: String::new(),
            definition: String::new(),
            image: DynamicImage::new_rgb8(1, 1),
            example: String::new(),
        }
    }

    fn get_image_buf(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut buf = Cursor::new(Vec::new());
        self.image
            .write_to(&mut buf, image::ImageOutputFormat::Png)?;
        Ok(buf.into_inner())
    }
}

const IMAGE_RANDOM_POOL_SIZE: u32 = 10;

#[async_trait]
impl Pipeline for VisualVocabPipeline {
    async fn run(
        &self,
        input: Option<PipelineIO>,
    ) -> Result<PipelineIO, Box<dyn std::error::Error>> {
        let VisualVocabPipeline {
            row,
            col,
            name,
            period,
            filename,
        } = self;

        let col = *col;
        let row = *row;
        let flashcard = match input {
            Some(PipelineIO::Flashcard(vocab)) => vocab,
            _ => return Err(CliError::new("No flashcard input").into()),
        };

        // pick random words
        let mut words = flashcard.clone();
        let mut result: Vec<Flashcard> = vec![];
        for _ in 0..row * col {
            let word = words.remove(random::<usize>() % words.len());
            result.push(word);
        }
        info!(target: "visual_vocab", "Picked {} words", result.len());

        // create visual flashcards
        info!(target: "visual_vocab", "Creating visual flashcards");
        let vocabs = create_visual_vocabs(result.as_slice())
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

        // a4paper
        let paper_width = super::docx::cm(21.0);
        let paper_height = super::docx::cm(29.7);

        // create tables
        let handles =
            vocabs.chunks(col as usize).enumerate().map(|(i, vocabs)| {
                info!(target: "visual_vocab", "Creating row {}", i);
                let vocabs = vocabs.to_owned();
                tokio::spawn(async move {
                    VisualFlashCard::to_table(
                        vocabs,
                        (paper_width, paper_height / 3),
                    )
                    .await
                    .map_err(|err| {
                        format!("Error creating visual flashcard: {}", err)
                    })
                })
            });
        let mut tables = futures::future::join_all(handles).await;
        for table in tables.drain(..) {
            info!(target: "visual_vocab", "Adding table");
            let table = table??;
            docx = docx.add_table(table).add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("")),
            );
        }

        // save document
        let mut buffer = Cursor::new(Vec::new());
        docx.build()
            .pack(&mut buffer)
            .expect("should have built document");
        Ok(PipelineIO::Document {
            name: filename.to_string(),
            content: buffer.into_inner(),
        })
    }

    fn name(&self) -> &'static str {
        "visual_vocab"
    }
}

/// Create visual flashcards
async fn create_visual_vocabs(
    vocabs: &[Flashcard],
) -> Result<Vec<VisualFlashCard>, PipelineError> {
    info!(target: "visual_vocab", "Creating visual {} flashcards", vocabs.len());

    let tasks = vocabs.iter().map(|vocab| {
        let vocab = vocab.clone();
        tokio::spawn(async move {
            match create_visual_vocab(&vocab).await {
                Ok(vocab) => vocab,
                Err(err) => {
                    error!(target: "visual_vocab", "Error creating visual flashcard: {}", err);
                    VisualFlashCard::default()
                }
            }
        })
    });
    let result = futures::future::join_all(tasks)
        .await
        .into_iter()
        .filter_map(|res| res.ok())
        .collect();
    Ok(result)
}

/// Create a visual flashcard
async fn create_visual_vocab(
    vocab: &Flashcard,
) -> Result<VisualFlashCard, PipelineError> {
    info!(target: "visual_vocab", "Creating visual flashcard for {}", vocab);

    let mut images = image_search_max(&vocab.word, IMAGE_RANDOM_POOL_SIZE)
        .await
        .map_err(|e| {
            PipelineError::new(&format!("Error getting images: {}", e))
        })?;

    let definition = search_vocab(&vocab.word).await.map_err(|e| {
        PipelineError::new(&format!("Error searching for definition: {}", e))
    })?;

    let image = loop {
        let img = images.remove(random::<usize>() % images.len());
        match img.full.get_image().await {
            Ok(img) => {
                break Some(img);
            }
            Err(err) => {
                error!(target: "visual_vocab", "Error getting image bytes: {}", err);
            }
        }
    };
    let image = match image {
        Some(img) => img,
        None => {
            return Err(PipelineError::new("No image found"));
        }
    };
    info!(target: "visual_vocab", "Got image for {}", vocab);

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

    let definition =
        examples.iter().map(|x| x.0.to_owned()).collect::<Vec<_>>();
    let rank = deep_search(&vocab.word, &definition, 1, 0.0).await;
    let example = examples[rank[0].0].1.to_owned();

    let visual_flash_card = VisualFlashCard {
        word: vocab.word.to_owned(),
        definition: vocab.definition.to_owned(),
        image,
        example,
    };
    info!(target: "visual_vocab", "Created visual flashcard {}", visual_flash_card);
    Ok(visual_flash_card)
}

static SENTENCE_EMBEDDER: OnceCell<Mutex<SentenceEmbeddingsModel>> =
    OnceCell::const_new();

/// Search for a query in a list of strings
/// - `query` is the string to search for
/// - `contents` is the list of strings to search in
/// - `limit` is the maximum number of results to return. If 0, return all results
/// - `threshold` is the minimum similarity score to return a result
/// Return a list ranked by relevance of the results
async fn deep_search(
    query: &str,
    contents: &[String],
    limit: usize,
    threshold: f32,
) -> Vec<(usize, f32)> {
    debug!(target: "deep_search", "Searching for {} in {} contents", query, contents.len());
    if contents.is_empty() {
        info!(target: "deep_search", "No contents to search for {}", query);
        return vec![];
    }

    let model = SENTENCE_EMBEDDER
        .get_or_init(|| async {
            task::spawn_blocking(move || {
                info!(target: "deep_search", "Loading sentence embedder model");
                Mutex::new(
                    SentenceEmbeddingsBuilder::remote(
                        SentenceEmbeddingsModelType::AllMiniLmL12V2,
                    )
                    .create_model()
                    .expect("should have created a model"),
                )
            })
            .await
            .expect("should have awaited task")
        })
        .await
        .lock()
        .await;
    let query_embedding =
        model.encode(&[query]).expect("should have encoded query")[0]
            .to_owned();
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rust_bert() {
        let model = SentenceEmbeddingsBuilder::remote(
            SentenceEmbeddingsModelType::AllMiniLmL12V2,
        )
        .create_model()
        .expect("should have created a model");
        let sentences =
            ["this is an example sentence", "each sentence is converted"];
        let output = model
            .encode(&sentences)
            .expect("should have encoded sentences");
        println!("{:?}", output);
    }

    #[tokio::test]
    async fn test_deep_search() {
        let query = "this is an example sentence";
        let contents = [
            "this example sentence is the first sentence".to_string(),
            "each sentence is converted".to_string(),
            "this is a different sentence".to_string(),
        ];
        let mut tasks = vec![];
        for _ in 0..8 {
            let contents = contents.clone();
            let task = tokio::spawn(async move {
                let results =
                    deep_search(query, contents.as_ref(), 0, 0.0).await;
                assert_eq!(results.len(), 3);
                assert_eq!(results[0].0, 0);
                results
            });
            tasks.push(task);
        }
        let mut results = vec![];
        for task in tasks {
            results.push(task.await.expect("should have awaited task"));
        }
        assert_eq!(results.len(), 8);
        for i in 0..8 {
            assert_eq!(results[i], results[0]);
        }
    }
}
