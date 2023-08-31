use anyhow::Result;
use data_encoding::HEXLOWER;
use lindera::tokenizer::{DictionaryConfig, Tokenizer};
use lindera::{
    mode::Mode,
    tokenizer::{Tokenizer, TokenizerConfig, UserDictionaryConfig},
    DictionaryKind,
};
use once_cell::sync::{Lazy, OnceCell};
use reqwest::Url;
use sha2::{Digest, Sha256};
use std::collections::hash_map;
use std::{collections::HashMap, hash::Hash};
use tokio::sync::RwLock;

static DEFAULT_TOKENIZER: Lazy<Tokenizer> = Lazy::new(|| {
    let dictionary = DictionaryConfig {
        kind: Some(DictionaryKind::IPADIC),
        path: None,
    };
    let config = TokenizerConfig {
        dictionary,
        user_dictionary: None,
        mode: Mode::Normal,
        with_details: false,
    };
    Tokenizer::new(config).unwrap()
});

static TOKENIZERS: OnceCell<RwLock<HashMap<Url, TokenizerInfo>>> =
    OnceCell::<RwLock<HashMap<Url, TokenizerInfo>>>::new();
struct TokenizerInfo {
    file_hash: String,
    tokenizer: Tokenizer,
}

pub async fn get_tokenizer(user_dictionary_url: Option<Url>) -> Result<Tokenizer> {
    let Some(user_dictionary_url)=user_dictionary_url else {
        return Ok((*DEFAULT_TOKENIZER).clone());
    };
    let rw_tokenizers =
        TOKENIZERS.get_or_init(|| RwLock::new(HashMap::<Url, TokenizerInfo>::new()));
    let user_dict = reqwest::get(user_dictionary_url.clone())
        .await?
        .text()
        .await?;
    let user_dict_hash = compute_user_dictionary_hash(&user_dict);
    let element = {
        let tokenizers = rw_tokenizers.read().await;
        tokenizers.get(&user_dictionary_url)
    };
    if let Some(tokenizer_info) = element {
        if tokenizer_info.file_hash == user_dict_hash {
            Ok(tokenizer_info.tokenizer.clone())
        } else {
            let dictionary = DictionaryConfig {
                kind: Some(DictionaryKind::IPADIC),
                path: None,
            };
            let user_dictionary = Some(UserDictionaryConfig {
                kind: Some(DictionaryKind::IPADIC),
                path: PathBuf::from("./assets/userdic.csv"),
            });
            let config = TokenizerConfig {
                dictionary,
                user_dictionary: user_dictionary,
                mode: Mode::Normal,
                with_details: false,
            };
            let tokenizer = Tokenizer::new(config)?;
            let tokenizers = rw_tokenizers.write().await;
            tokenizers[user_dictionary_url] = TokenizerInfo {
                file_hash: user_dict_hash,
                tokenizer,
            };
        }
    } else {
    }

    Err(anyhow::anyhow!(""))
}

fn compute_user_dictionary_hash(user_dictionary: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(user_dictionary.as_bytes());
    let digest = hasher.finalize();
    HEXLOWER.encode(digest.as_ref())
}
