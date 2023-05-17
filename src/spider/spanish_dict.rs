use ego_tree::NodeRef;
use html5ever::tree_builder::QuirksMode;
use log::{debug, info};
use once_cell::sync::Lazy;
use rust_bert::pipelines::keywords_extraction::KeywordExtractionModel;
use scraper::{node::Node, ElementRef, Html, Selector};
use tokio::sync::{Mutex, OnceCell};
use tokio::task;
use url::form_urlencoded;

use super::{SpiderError, CLIENT};

/// Represents an example of a word in a dictionary
#[derive(Debug)]
pub enum DictionaryExample {
    Example {
        example: String,
    },
    ExampleAndTranslation {
        example: String,
        translation: String,
    },
}

/// Represents a definition of a word in a dictionary
#[derive(Debug)]
pub enum DictionaryDefinition {
    Definition {
        definition: String,
    },
    DefinitionAndGroup {
        group: String,
        definition: String,
    },
    DefinitionAndGroupWithExample {
        group: String,
        definition: String,
        examples: Vec<DictionaryExample>,
    },
}

/// Represents a word in a dictionary
#[derive(Debug)]
pub struct DictionaryEntry {
    pub word: String,
    pub definitions: Vec<DictionaryDefinition>,
}

const LANG_EN: &str = "en";
const LANG_ES: &str = "es";

static KEYWORD_MODEL: OnceCell<Mutex<KeywordExtractionModel>> = OnceCell::const_new();

/**
Perform a search of a word in SpanishDict.com
 */
pub async fn search_vocab(word: &str) -> Result<DictionaryEntry, Box<dyn std::error::Error>> {
    let _lock = KEYWORD_MODEL
        .get_or_init(|| async {
            task::spawn_blocking(move || {
                info!(target: "spanish_dict", "loading keyword model");
                let model = KeywordExtractionModel::new(Default::default())
                    .expect("should be able to load keyword model");
                Mutex::new(model)
            })
            .await
            .expect("should be able to get model")
        })
        .await
        .lock();

    let model = _lock.await;
    for _ in 0..2 {
        match search_vocab_inner(word).await {
            Ok(entry) => {
                if entry.definitions.is_empty() {
                    info!(target: "spanish_dict", "failed to find any definitions for word: {}", word);
                } else {
                    return Ok(entry);
                }
            }
            Err(e) => {
                info!(target: "spanish_dict", "failed to search for word: {} because\n{}", word, e);
            }
        }
    }
    for _ in 0..2 {
        let prediction = model.predict(&[word])?;
        match prediction.get(0) {
            Some(keyword) => {
                let keyword = match keyword.get(0) {
                    Some(keyword) => &keyword.text,
                    None => {
                        info!(target: "spanish_dict", "failed to find any keywords for word: {}", word);
                        continue;
                    }
                };
                info!(target: "spanish_dict", "found keyword: {}", keyword);
                match search_vocab_inner(keyword).await {
                    Ok(entry) => {
                        if entry.definitions.is_empty() {
                            info!(target: "spanish_dict", "failed to find any definitions for word: {}", keyword);
                        } else {
                            return Ok(entry);
                        }
                    }
                    Err(e) => {
                        info!(target: "spanish_dict", "failed to search for word: {} because\n{}", keyword, e);
                    }
                }
            }
            None => {
                info!(target: "spanish_dict", "failed to find any keywords for word: {}", word);
            }
        }
    }

    Err(Box::new(SpiderError::new(&format!(
        "failed to search for word: {}",
        word
    ))))
}

async fn search_vocab_inner(word: &str) -> Result<DictionaryEntry, &'static str> {
    let encoded = form_urlencoded::Serializer::new(String::new())
        .append_key_only(word)
        .finish();
    let url = format!("https://www.spanishdict.com/translate/{encoded}");
    debug!(target: "spanish_dict", "url: {}", url);
    let html = CLIENT
        .get(&url)
        .send()
        .await
        .expect("should be able to send request")
        .text()
        .await
        .expect("should be able to get text");
    let dom = Html::parse_document(&html);
    let selector =
        Lazy::new(|| Selector::parse("#main-container-video div[id^=dictionary]").unwrap());
    let mut definitions: Vec<_> = vec![];

    for dictionary in dom.select(&selector) {
        let id = dictionary.value().attr("id").unwrap();
        match id {
            "dictionary-neodict-es" => {
                let selector = Lazy::new(|| {
                    Selector::parse(&format!("div[lang] div[lang^={}]", LANG_EN)).unwrap()
                });
                for group in dictionary.select(&selector) {
                    for definition in group.next_sibling().unwrap().children() {
                        let dom = as_dom(definition);

                        let definition_text = get_text_from_selector(&dom, "a", LANG_EN);
                        let example_text = get_text_from_selector(&dom, "span", LANG_ES);
                        let translation_text = get_text_from_selector(&dom, "span", LANG_EN);

                        let selector = Lazy::new(|| Selector::parse("span:last-child").unwrap());
                        let group_text = textify(&group.select(&selector).next().unwrap());

                        definitions.push(DictionaryDefinition::DefinitionAndGroupWithExample {
                            group: group_text,
                            definition: definition_text,
                            examples: vec![DictionaryExample::ExampleAndTranslation {
                                example: example_text,
                                translation: translation_text,
                            }],
                        });
                    }
                }
            }
            "dictionary-neoharrap-es" => {
                let selector = Lazy::new(|| {
                    Selector::parse("#dictionary-neoharrap-es > div > div > div:nth-child(2) > div")
                        .unwrap()
                });

                for group in ElementRef::wrap(dictionary.parent().unwrap())
                    .unwrap()
                    .select(&selector)
                {
                    let intermediate = group
                        .children()
                        .nth(1)
                        .unwrap()
                        .first_child()
                        .unwrap()
                        .children()
                        .filter_map(|e| {
                            if let Some(e) = ElementRef::wrap(e) {
                                return Some(e);
                            }
                            None
                        })
                        .collect::<Vec<_>>();

                    let definition = if intermediate.len() == 3 {
                        textify(&intermediate[1])
                    } else {
                        "".to_string()
                    };

                    let group = textify(
                        &ElementRef::wrap(group.first_child().unwrap().children().nth(2).unwrap())
                            .unwrap(),
                    );

                    let example = intermediate[intermediate.len() - 1];
                    if example.children().next().is_none() {
                        continue;
                    }

                    let result = example
                        .children()
                        .filter_map(|e| {
                            if let Some(e) = ElementRef::wrap(e) {
                                let collect = e
                                    .children()
                                    .filter_map(|e| {
                                        if let Some(e) = ElementRef::wrap(e) {
                                            return Some(e);
                                        }
                                        None
                                    })
                                    .collect::<Vec<_>>();

                                if collect.len() == 3 {
                                    let example = textify(&collect[0]);
                                    let translation = textify(&collect[2]);
                                    return Some(DictionaryExample::ExampleAndTranslation {
                                        example,
                                        translation,
                                    });
                                }
                            }
                            None
                        })
                        .collect::<Vec<_>>();

                    if result.is_empty() {
                        definitions
                            .push(DictionaryDefinition::DefinitionAndGroup { group, definition });
                    } else {
                        definitions.push(DictionaryDefinition::DefinitionAndGroupWithExample {
                            group,
                            definition,
                            examples: result,
                        });
                    }
                }
            }
            &_ => {
                debug!(target: "spanish_dict", "unknown dictionary: {}", id);
            }
        }
    }
    Ok(DictionaryEntry {
        word: word.to_string(),
        definitions,
    })
}

/// Wrap a NodeRef into scraper HTML to enable CSS selectors
fn as_dom(definition: NodeRef<Node>) -> Html {
    let dom = Html {
        errors: vec![],
        tree: definition.tree().to_owned(),
        quirks_mode: QuirksMode::NoQuirks,
    };
    dom
}

/// Textify a ElementRef
fn textify(element: &ElementRef) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .trim_end_matches(')')
        .trim_start_matches('(')
        .to_string()
}

/// Get the text from a selector, given a language & tag of the selector
fn get_text_from_selector(dom: &Html, selector_str: &str, lang: &str) -> String {
    let selector =
        Lazy::new(|| Selector::parse(&format!("{}[lang={}]", selector_str, lang)).unwrap());
    textify(&dom.select(&selector).next().unwrap())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn search_light() {
        let result = search_vocab_inner("luz").await.unwrap();
        assert_eq!(result.word, "luz");
        assert!(!result.definitions.is_empty());
        dbg!(result);
    }
}
