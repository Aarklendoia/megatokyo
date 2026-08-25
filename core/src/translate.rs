//! DeepL translation, cached per `(rant_number, lang)` in [`crate::store`] —
//! per the plan's decision 3, a rant is only ever sent to DeepL once per
//! target language, on the first request for it; every later request for
//! the same pair is served straight from SQLite.
//!
//! No API key ships with this crate (unlike the original .NET version, whose
//! `appsettings.json` embedded a live Bing Translator key): the daemon reads
//! one from its own config, provided by whoever runs it.

use serde::Deserialize;

use crate::store::{Store, StoreError};

#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("DeepL request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("DeepL returned no translation")]
    Empty,
}

#[derive(Debug, thiserror::Error)]
pub enum GetTranslatedRantError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Translate(#[from] TranslateError),
}

pub struct Translator {
    client: reqwest::Client,
    api_key: String,
    /// Always `None` outside tests: [`Translator::endpoint`] then derives
    /// the real DeepL host from the key. Tests use
    /// [`Translator::with_endpoint`] to point requests at a local mock
    /// server instead, since DeepL itself obviously isn't reachable in CI.
    endpoint_override: Option<String>,
}

impl Translator {
    pub fn new(api_key: String) -> Self {
        Self::with_client(reqwest::Client::new(), api_key)
    }

    /// Same as [`Translator::new`], but reuses a caller-supplied client
    /// instead of building a fresh one — lets a caller that translates
    /// repeatedly (e.g. the daemon's `/rant` route, which builds a
    /// `Translator` fresh per request so a `deepl_api_key` change takes
    /// effect immediately) keep one connection pool/TLS context across
    /// calls instead of paying a new handshake to DeepL every time.
    pub fn with_client(client: reqwest::Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            endpoint_override: None,
        }
    }

    #[cfg(test)]
    fn with_endpoint(api_key: String, endpoint: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            endpoint_override: Some(endpoint),
        }
    }

    /// DeepL free-tier keys are suffixed `:fx` and must hit a different host
    /// than paid/pro keys — this is DeepL's own convention, not something
    /// megatokyo chooses.
    fn endpoint(&self) -> String {
        if let Some(endpoint) = &self.endpoint_override {
            return endpoint.clone();
        }
        if self.api_key.ends_with(":fx") {
            "https://api-free.deepl.com/v2/translate".to_string()
        } else {
            "https://api.deepl.com/v2/translate".to_string()
        }
    }

    /// Translates `html` (a rant's stored content) into `target_lang` (a
    /// DeepL language code, e.g. `"FR"`), preserving markup via DeepL's own
    /// `tag_handling=html` — rants are stored as HTML (see
    /// `scraper::rants`), and this must round-trip that markup rather than
    /// flattening it to plain text.
    pub async fn translate_html(
        &self,
        html: &str,
        target_lang: &str,
    ) -> Result<String, TranslateError> {
        #[derive(Deserialize)]
        struct DeeplResponse {
            translations: Vec<DeeplTranslation>,
        }
        #[derive(Deserialize)]
        struct DeeplTranslation {
            text: String,
        }

        let response: DeeplResponse = self
            .client
            .post(self.endpoint())
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .form(&[
                ("text", html),
                ("target_lang", target_lang),
                ("tag_handling", "html"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        response
            .translations
            .into_iter()
            .next()
            .map(|t| t.text)
            .ok_or(TranslateError::Empty)
    }
}

/// The daemon's `GET /rant?number=N&lang=..` route (see the plan): returns
/// the rant's original content when `lang` is empty/`en`, otherwise the
/// cached translation if one exists, otherwise translates via DeepL and
/// caches the result before returning it. `Ok(None)` means the rant itself
/// doesn't exist; translation failures propagate as `Err` rather than
/// silently falling back to the original text, so a caller can tell "no
/// rant" apart from "translation is temporarily broken".
pub async fn get_translated_rant(
    store: &Store,
    translator: &Translator,
    number: i32,
    lang: &str,
) -> Result<Option<String>, GetTranslatedRantError> {
    let Some(rant) = store.rant_by_number(number)? else {
        return Ok(None);
    };
    if lang.is_empty() || lang.eq_ignore_ascii_case("en") {
        return Ok(Some(rant.content));
    }
    if let Some(cached) = store.get_translation(number, lang)? {
        return Ok(Some(cached));
    }
    let translated = translator.translate_html(&rant.content, lang).await?;
    store.save_translation(number, lang, &translated)?;
    Ok(Some(translated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Rant;
    use wiremock::matchers::{header, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_rant() -> Rant {
        Rant {
            number: 1106,
            author: "Piro".to_string(),
            title: "Clearing of the Air".to_string(),
            url: "https://megatokyo.com/rantimgs/1106.png".to_string(),
            publish_date: "2023-09-27T00:00:00Z".to_string(),
            content: "<p>hello</p>".to_string(),
        }
    }

    #[test]
    fn free_tier_keys_use_the_free_endpoint() {
        let translator = Translator::new("abc123:fx".to_string());
        assert_eq!(
            translator.endpoint(),
            "https://api-free.deepl.com/v2/translate"
        );
    }

    #[test]
    fn pro_keys_use_the_pro_endpoint() {
        let translator = Translator::new("abc123".to_string());
        assert_eq!(translator.endpoint(), "https://api.deepl.com/v2/translate");
    }

    #[tokio::test]
    async fn translate_html_posts_the_text_and_returns_the_first_translation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "translations": [{"detected_source_language": "EN", "text": "<p>bonjour</p>"}]
            })))
            .mount(&server)
            .await;

        let translator = Translator::with_endpoint("test-key".to_string(), server.uri());
        let translated = translator
            .translate_html("<p>hello</p>", "FR")
            .await
            .unwrap();
        assert_eq!(translated, "<p>bonjour</p>");
    }

    /// DeepL's API rejects requests that send the key as an `auth_key` form
    /// field — verified live against a real key (403 "Missing Authorization
    /// header"): it must go in an `Authorization: DeepL-Auth-Key <key>`
    /// header instead (see DeepL's own quickstart docs). This was the
    /// actual bug behind translation silently failing for every user with a
    /// real key, caught only by testing against the live API — this test
    /// pins the fix down so it can't regress back to the form-field form.
    #[tokio::test]
    async fn translate_html_sends_the_key_as_an_authorization_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("Authorization", "DeepL-Auth-Key test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "translations": [{"detected_source_language": "EN", "text": "<p>bonjour</p>"}]
            })))
            .mount(&server)
            .await;

        let translator = Translator::with_endpoint("test-key".to_string(), server.uri());
        let translated = translator
            .translate_html("<p>hello</p>", "FR")
            .await
            .unwrap();
        assert_eq!(translated, "<p>bonjour</p>");
    }

    #[tokio::test]
    async fn translate_html_errors_on_a_non_success_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let translator = Translator::with_endpoint("bad-key".to_string(), server.uri());
        assert!(translator
            .translate_html("<p>hello</p>", "FR")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn get_translated_rant_returns_original_for_english_or_empty_lang() {
        let store = Store::open_in_memory().unwrap();
        store.upsert_rant(&sample_rant()).unwrap();
        let translator = Translator::new("unused".to_string());

        for lang in ["", "en", "EN"] {
            let content = get_translated_rant(&store, &translator, 1106, lang)
                .await
                .unwrap();
            assert_eq!(content, Some("<p>hello</p>".to_string()));
        }
    }

    #[tokio::test]
    async fn get_translated_rant_returns_none_for_an_unknown_rant() {
        let store = Store::open_in_memory().unwrap();
        let translator = Translator::new("unused".to_string());
        let content = get_translated_rant(&store, &translator, 9999, "fr")
            .await
            .unwrap();
        assert_eq!(content, None);
    }

    #[tokio::test]
    async fn get_translated_rant_serves_a_cached_translation_without_calling_deepl() {
        let store = Store::open_in_memory().unwrap();
        store.upsert_rant(&sample_rant()).unwrap();
        store
            .save_translation(1106, "fr", "<p>bonjour (cache)</p>")
            .unwrap();
        // A translator pointed at an unresolvable host: if the cache lookup
        // didn't short-circuit before reaching the network, this call would
        // error out instead of returning the cached value.
        let translator = Translator::new("unused".to_string());
        let content = get_translated_rant(&store, &translator, 1106, "fr")
            .await
            .unwrap();
        assert_eq!(content, Some("<p>bonjour (cache)</p>".to_string()));
    }
}
