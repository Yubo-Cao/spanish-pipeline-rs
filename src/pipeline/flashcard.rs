use std::path::PathBuf;

use serde::{
    de::Error,
    ser::{Serialize, SerializeSeq, Serializer},
    Deserialize, Deserializer,
};

/// Represents the flashcard output of a pipeline stage.
#[derive(Debug, Clone)]
pub struct Flashcard {
    pub word: String,
    pub definition: String,
}

impl Flashcard {
    /// Load the flashcards from a file.
    /// The file should be yaml format with a list of list of strings,
    /// first string is the word, second string is the definition.
    pub fn load(path: &PathBuf) -> Result<Vec<Flashcard>, &'static str> {
        let yaml = std::fs::read_to_string(path).map_err(|_| "Failed to read file")?;
        let flashcards: Vec<Flashcard> =
            serde_yaml::from_str(&yaml).map_err(|_| "Failed to parse yaml")?;
        Ok(flashcards)
    }
}

impl Serialize for Flashcard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.word)?;
        seq.serialize_element(&self.definition)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Flashcard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seq: Vec<String> = Vec::deserialize(deserializer)?;
        if seq.len() != 2 {
            return Err(D::Error::invalid_length(
                seq.len(),
                &"expected a sequence with two elements",
            ));
        }
        let word = seq[0].clone();
        let definition = seq[1].clone();
        Ok(Flashcard { word, definition })
    }
}
