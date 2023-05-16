use rust_bert::pipelines::sentence_embeddings::{
    builder::SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};

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
}
