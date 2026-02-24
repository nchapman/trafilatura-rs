// Port of go-trafilatura/metadata-json.go

use serde_json::{Map, Value};

use crate::dom::Document;
use crate::options::Options;
use crate::result::Metadata;
use crate::utils::{str_or, str_word_count, trim, unescape_html, uniquify_lists};

/// Schema.org data extracted from a JSON-LD script block.
#[derive(Clone)]
pub struct SchemaData {
    pub types: Vec<String>,
    pub data: Map<String, Value>,
    pub importance: f64,
    /// Types of the parent schema node in the JSON-LD tree.
    pub parent_types: Option<Vec<String>>,
    /// Types of the grandparent schema node (parent's parent).
    pub grandparent_types: Option<Vec<String>>,
}

/// Port of `extractJsonLd`.
pub fn extract_json_ld(opts: &Options, doc: &Document, original_metadata: Metadata) -> Metadata {
    let mut metadata = Metadata::default();
    let (persons, organizations, articles) = decode_json_ld(doc, opts);

    // Extract metadata from each article, in importance order.
    for article in &articles {
        // Grab author from schema with @type "Person".
        if metadata.author.is_empty() {
            let mut author_names = String::new();
            let author_val = article.data.get("author").cloned().unwrap_or(Value::Null);
            for author in get_schema_names(&author_val, &["person"]) {
                let author = super::validate_metadata_name(&author);
                author_names = super::normalize_authors(&author_names, &author);
            }
            if !author_names.is_empty() {
                metadata.author = author_names;
            }
        }

        // Grab sitename from publisher.
        if metadata.sitename.is_empty() {
            let pub_val = article.data.get("publisher").cloned().unwrap_or(Value::Null);
            let names = get_schema_names(&pub_val, &[]);
            if !names.is_empty() {
                metadata.sitename = names[0].clone();
            }
        }

        // Grab categories.
        let categories = get_string_values(&article.data, "articleSection");
        if !categories.is_empty() {
            metadata.categories.extend(categories);
        }

        // Grab tags.
        let keywords_val = article.data.get("keywords").cloned().unwrap_or(Value::Null);
        let tags = get_schema_names(&keywords_val, &[]);
        if !tags.is_empty() {
            metadata.tags.extend(tags);
        }

        // Grab title.
        if metadata.title.is_empty() {
            metadata.title = get_single_string_value(&article.data, "name");
        }

        // If title is empty or a single word, try headline variants.
        if metadata.title.is_empty() || str_word_count(&metadata.title) == 1 {
            for (attr, _) in &article.data {
                if !attr.to_lowercase().contains("headline") {
                    continue;
                }
                let title = get_single_string_value(&article.data, attr);
                if !title.is_empty() && !title.contains("...") {
                    metadata.title = title;
                    break;
                }
            }
        }

        // If title found, use article type as page type.
        if metadata.page_type.is_empty()
            && !metadata.title.is_empty()
            && !article.types.is_empty()
        {
            metadata.page_type = article.types[0].clone();
        }
    }

    // If author still not found, look in person schemas.
    if metadata.author.is_empty() {
        let mut author_names = String::new();
        for person in &persons {
            let person_val = Value::Object(person.data.clone());
            for name in get_schema_names(&person_val, &[]) {
                let name = super::validate_metadata_name(&name);
                author_names = super::normalize_authors(&author_names, &name);
            }
        }
        if !author_names.is_empty() {
            metadata.author = author_names;
        }
    }

    // If sitename still not found, look in organization schemas.
    if metadata.sitename.is_empty() {
        let mut names = Vec::new();
        for org in &organizations {
            let org_val = Value::Object(org.data.clone());
            for name in get_schema_names(&org_val, &[]) {
                let name = super::validate_metadata_name(&name);
                if !name.is_empty() {
                    names.push(name);
                }
            }
        }
        if !names.is_empty() {
            metadata.sitename = names.join("; ");
        }
    }

    // If page type still not found, use first article type.
    if metadata.page_type.is_empty() {
        if let Some(first) = articles.first() {
            if !first.types.is_empty() {
                metadata.page_type = first.types[0].clone();
            }
        }
    }

    // Uniquify tags and categories.
    let tag_strs: Vec<&str> = metadata.tags.iter().map(|s| s.as_str()).collect();
    metadata.tags = uniquify_lists(&tag_strs);
    let cat_strs: Vec<&str> = metadata.categories.iter().map(|s| s.as_str()).collect();
    metadata.categories = uniquify_lists(&cat_strs);

    // Merge into original metadata.
    // JSON-LD overrides original for author (JSON-LD is more structured).
    let mut result = original_metadata;
    result.title = str_or(&[&metadata.title, &result.title]).to_string();
    result.page_type = str_or(&[&result.page_type, &metadata.page_type]).to_string();
    result.author = str_or(&[&metadata.author, &result.author]).to_string();

    if !metadata.categories.is_empty() {
        result.categories = metadata.categories;
    }
    if !metadata.tags.is_empty() {
        result.tags = metadata.tags;
    }

    // If new sitename is longer, prefer it.
    if metadata.sitename.chars().count() > result.sitename.chars().count() {
        result.sitename = metadata.sitename;
    }

    result
}

/// Port of `decodeJsonLd`.
fn decode_json_ld(
    doc: &Document,
    _opts: &Options,
) -> (Vec<SchemaData>, Vec<SchemaData>, Vec<SchemaData>) {
    let mut persons: Vec<SchemaData> = Vec::new();
    let mut organizations: Vec<SchemaData> = Vec::new();
    let mut articles: Vec<SchemaData> = Vec::new();

    // Find all script nodes with JSON+LD schema.
    let mut script_ids =
        doc.query_selector_all(doc.root(), r#"script[type="application/ld+json"]"#);
    script_ids.extend(
        doc.query_selector_all(doc.root(), r#"script[type="application/settings+json"]"#),
    );

    for script_id in script_ids {
        let json_text = doc.text_content(script_id);
        let json_text = unescape_html(json_text.trim());
        if json_text.is_empty() {
            continue;
        }

        // Try as JSON array first, then as single object.
        let data_list: Vec<Map<String, Value>> =
            if let Ok(list) = serde_json::from_str::<Vec<Map<String, Value>>>(&json_text) {
                list
            } else if let Ok(obj) = serde_json::from_str::<Map<String, Value>>(&json_text) {
                vec![obj]
            } else {
                tracing::warn!("error in JSON metadata extraction");
                continue;
            };

        for data in data_list {
            find_important_objects(&data, None, &mut persons, &mut organizations, &mut articles);
        }
    }

    // Sort by importance (higher importance first).
    organizations.sort_by(|a, b| {
        b.importance
            .partial_cmp(&a.importance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    articles.sort_by(|a, b| {
        b.importance
            .partial_cmp(&a.importance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Prefer persons and organizations nested inside articles.
    let article_persons: Vec<SchemaData> = persons
        .iter()
        .filter(|p| schema_in_article(p, "person"))
        .cloned()
        .collect();
    if !article_persons.is_empty() {
        persons = article_persons;
    }

    let article_orgs: Vec<SchemaData> = organizations
        .iter()
        .filter(|o| schema_in_article(o, "organization"))
        .cloned()
        .collect();
    if !article_orgs.is_empty() {
        organizations = article_orgs;
    }

    (persons, organizations, articles)
}

/// Port of `findImportantObjects` (recursive).
///
/// `parent_ctx` = (parent_types, grandparent_types).
fn find_important_objects(
    obj: &Map<String, Value>,
    parent_ctx: Option<(&[String], Option<&[String]>)>,
    persons: &mut Vec<SchemaData>,
    organizations: &mut Vec<SchemaData>,
    articles: &mut Vec<SchemaData>,
) {
    let schema_types = get_schema_types(obj, false);

    let mut is_person = false;
    let mut is_website = false;
    let mut is_organization = false;
    let mut is_article = false;
    let mut is_posting = false;
    let mut is_report = false;
    let mut is_blog = false;
    let mut is_page = false;
    let mut is_listing = false;

    for st in &schema_types {
        let st = st.to_lowercase();
        is_person = is_person || st == "person";
        is_website = is_website || st == "website";
        is_organization = is_organization || st.contains("organization");
        is_article = is_article || st.contains("article");
        is_posting = is_posting || st.contains("posting");
        is_report = is_report || st == "report";
        is_blog = is_blog || st == "blog";
        is_page = is_page || st.contains("page");
        is_listing = is_listing || st.contains("listing");
    }

    let (parent_types, grandparent_types) = match parent_ctx {
        Some((pt, gpt)) => (Some(pt.to_vec()), gpt.map(|g| g.to_vec())),
        None => (None, None),
    };

    let schema_data = SchemaData {
        types: schema_types.clone(),
        data: obj.clone(),
        importance: 0.0,
        parent_types,
        grandparent_types,
    };

    if is_person {
        persons.push(schema_data.clone());
    }

    if is_website || is_organization {
        let mut sd = schema_data.clone();
        sd.importance = if is_organization { 2.0 } else { 1.0 };
        organizations.push(sd);
    }

    if is_article || is_posting || is_report || is_blog || is_page || is_listing {
        let mut sd = schema_data.clone();
        sd.importance = if is_article || is_posting || is_report {
            3.0
        } else if is_blog {
            2.0
        } else {
            1.0 // page or listing
        };
        articles.push(sd);
    }

    // Recurse into sub-objects.
    let child_parent_ctx: (&[String], Option<&[String]>) = (
        &schema_types,
        parent_ctx.map(|(pt, _)| pt),
    );
    for (_, value) in obj {
        match value {
            Value::Object(child_obj) => {
                find_important_objects(
                    child_obj,
                    Some(child_parent_ctx),
                    persons,
                    organizations,
                    articles,
                );
            }
            Value::Array(arr) => {
                for item in arr {
                    if let Value::Object(child_obj) = item {
                        find_important_objects(
                            child_obj,
                            Some(child_parent_ctx),
                            persons,
                            organizations,
                            articles,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

/// Port of `getSchemaNames`.
pub fn get_schema_names(v: &Value, expected_types: &[&str]) -> Vec<String> {
    use crate::utils::regex_patterns::{JSON_SYMBOL, NAME_JSON};

    match v {
        Value::String(s) => {
            // Handle embedded JSON-like fragments in name strings.
            let value = if JSON_SYMBOL.is_match(s) {
                if let Some(caps) = NAME_JSON.captures(s) {
                    caps.get(1).map(|m| m.as_str()).unwrap_or(s).to_string()
                } else {
                    s.clone()
                }
            } else {
                s.clone()
            };
            let value = trim(&value);
            if value.is_empty() { vec![] } else { vec![value] }
        }

        Value::Object(obj) => {
            let schema_types = get_schema_types(obj, true);

            // If expected types specified, schema must match one of them.
            if !expected_types.is_empty() {
                if schema_types.is_empty() {
                    return vec![];
                }
                let allowed = schema_types.iter().any(|t| expected_types.contains(&t.as_str()));
                if !allowed {
                    return vec![];
                }
            }

            // Try "name" property.
            let mut names = get_string_values(obj, "name");

            // If Person with no name, try name combination.
            if names.is_empty() && schema_types.iter().any(|t| t == "person") {
                let given = get_single_string_value(obj, "givenName");
                let additional = get_single_string_value(obj, "additionalName");
                let family = get_single_string_value(obj, "familyName");
                let full = trim(&format!("{given} {additional} {family}"));
                if !full.is_empty() {
                    names = vec![full];
                }
            }

            if names.is_empty() {
                names = get_string_values(obj, "legalName");
            }
            if names.is_empty() {
                names = get_string_values(obj, "alternateName");
            }

            if !names.is_empty() {
                return names;
            }

            // Recurse if "name" is an object or array.
            match obj.get("name") {
                Some(child @ Value::Object(_)) | Some(child @ Value::Array(_)) => {
                    get_schema_names(child, expected_types)
                }
                _ => vec![],
            }
        }

        Value::Array(arr) => {
            let mut names = Vec::new();
            for item in arr {
                names.extend(get_schema_names(item, expected_types));
            }
            names
        }

        _ => vec![],
    }
}

/// Port of `getSchemaTypes`.
pub fn get_schema_types(obj: &Map<String, Value>, to_lower: bool) -> Vec<String> {
    let mut types = get_string_values(obj, "@type");
    if to_lower {
        for t in &mut types {
            *t = t.to_lowercase();
        }
    }
    types
}

/// Port of `getStringValues`.
pub fn get_string_values(obj: &Map<String, Value>, key: &str) -> Vec<String> {
    match obj.get(key) {
        Some(Value::String(s)) => {
            let s = trim(s);
            if s.is_empty() { vec![] } else { vec![s] }
        }
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|item| {
                if let Value::String(s) = item {
                    let s = trim(s);
                    if !s.is_empty() { Some(s) } else { None }
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    }
}

/// Port of `getSingleStringValue`.
pub fn get_single_string_value(obj: &Map<String, Value>, key: &str) -> String {
    get_string_values(obj, key).into_iter().next().unwrap_or_default()
}

/// Port of `schemaInArticle`.
///
/// Returns true if this schema should be used (either top-level or nested inside an article).
fn schema_in_article(data: &SchemaData, wanted_type: &str) -> bool {
    let Some(parent_types) = &data.parent_types else {
        return true; // No parent = top-level = always include.
    };

    let parent_is_person = parent_types.iter().any(|t| t.to_lowercase() == "person");
    let parent_is_organization = parent_types.iter().any(|t| {
        let t = t.to_lowercase();
        t == "website" || t.contains("organization")
    });

    let types_to_check =
        if (wanted_type == "person" && parent_is_person)
            || (wanted_type == "organization" && parent_is_organization)
        {
            match &data.grandparent_types {
                None => return true,
                Some(gpt) => gpt,
            }
        } else {
            parent_types
        };

    is_article_type_list(types_to_check)
}

fn is_article_type_list(types: &[String]) -> bool {
    types.iter().any(|t| {
        let t = t.to_lowercase();
        t.contains("article")
            || t.contains("posting")
            || t == "report"
            || t == "blog"
            || t.contains("page")
            || t.contains("listing")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Document;
    use crate::options::Options;

    #[test]
    fn test_get_string_values_string() {
        let mut obj = Map::new();
        obj.insert("name".to_string(), Value::String("Alice".to_string()));
        assert_eq!(get_string_values(&obj, "name"), vec!["Alice".to_string()]);
    }

    #[test]
    fn test_get_string_values_array() {
        let mut obj = Map::new();
        obj.insert(
            "name".to_string(),
            Value::Array(vec![
                Value::String("Alice".to_string()),
                Value::String("Bob".to_string()),
            ]),
        );
        let vals = get_string_values(&obj, "name");
        assert_eq!(vals, vec!["Alice", "Bob"]);
    }

    #[test]
    fn test_get_schema_names_string() {
        let v = Value::String("Jane Doe".to_string());
        assert_eq!(get_schema_names(&v, &[]), vec!["Jane Doe".to_string()]);
    }

    #[test]
    fn test_get_schema_names_object_name() {
        let json = r#"{"@type": "Person", "name": "John Smith"}"#;
        let obj: Map<String, Value> = serde_json::from_str(json).unwrap();
        let v = Value::Object(obj);
        assert_eq!(get_schema_names(&v, &[]), vec!["John Smith".to_string()]);
    }

    #[test]
    fn test_get_schema_names_person_type_filter() {
        let json = r#"{"@type": "Organization", "name": "Acme Corp"}"#;
        let obj: Map<String, Value> = serde_json::from_str(json).unwrap();
        let v = Value::Object(obj);
        // expected_types "person" should reject an Organization.
        assert!(get_schema_names(&v, &["person"]).is_empty());
    }

    #[test]
    fn test_get_schema_names_person_given_family() {
        let json = r#"{"@type": "Person", "givenName": "Jane", "familyName": "Doe"}"#;
        let obj: Map<String, Value> = serde_json::from_str(json).unwrap();
        let v = Value::Object(obj);
        let names = get_schema_names(&v, &[]);
        assert_eq!(names, vec!["Jane Doe".to_string()]);
    }

    #[test]
    fn test_extract_json_ld_article_author() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@type":"Article","name":"Test Article","author":{"@type":"Person","name":"Jane Writer"}}
            </script>
        </head><body></body></html>"#;
        let doc = Document::parse(html);
        let result = extract_json_ld(&Options::default(), &doc, Metadata::default());
        assert_eq!(result.author, "Jane Writer");
        assert_eq!(result.title, "Test Article");
    }

    #[test]
    fn test_extract_json_ld_sitename_from_publisher() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@type":"Article","name":"Story","publisher":{"@type":"Organization","name":"The Times"}}
            </script>
        </head><body></body></html>"#;
        let doc = Document::parse(html);
        let result = extract_json_ld(&Options::default(), &doc, Metadata::default());
        assert_eq!(result.sitename, "The Times");
    }

    #[test]
    fn test_extract_json_ld_array() {
        // JSON-LD as array of objects.
        let html = r#"<html><head>
            <script type="application/ld+json">
            [{"@type":"Article","name":"Array Article","author":{"@type":"Person","name":"Array Author"}}]
            </script>
        </head><body></body></html>"#;
        let doc = Document::parse(html);
        let result = extract_json_ld(&Options::default(), &doc, Metadata::default());
        assert_eq!(result.title, "Array Article");
        assert_eq!(result.author, "Array Author");
    }

    #[test]
    fn test_extract_json_ld_categories() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@type":"Article","name":"Cat Article","articleSection":["Tech","News"]}
            </script>
        </head><body></body></html>"#;
        let doc = Document::parse(html);
        let result = extract_json_ld(&Options::default(), &doc, Metadata::default());
        assert!(result.categories.contains(&"Tech".to_string()));
        assert!(result.categories.contains(&"News".to_string()));
    }

    #[test]
    fn test_schema_in_article_no_parent() {
        let sd = SchemaData {
            types: vec!["Person".to_string()],
            data: Map::new(),
            importance: 0.0,
            parent_types: None,
            grandparent_types: None,
        };
        assert!(schema_in_article(&sd, "person"));
    }

    #[test]
    fn test_schema_in_article_person_in_article() {
        // Person with parent Article should be included.
        let sd = SchemaData {
            types: vec!["Person".to_string()],
            data: Map::new(),
            importance: 0.0,
            parent_types: Some(vec!["Article".to_string()]),
            grandparent_types: None,
        };
        assert!(schema_in_article(&sd, "person"));
    }

    #[test]
    fn test_schema_in_article_person_in_non_article() {
        // Person with parent Website should be excluded.
        let sd = SchemaData {
            types: vec!["Person".to_string()],
            data: Map::new(),
            importance: 0.0,
            parent_types: Some(vec!["WebSite".to_string()]),
            grandparent_types: None,
        };
        assert!(!schema_in_article(&sd, "person"));
    }

    #[test]
    fn test_schema_in_article_person_nested_in_person_top_level() {
        // Person whose parent Person is itself top-level (no grandparent) → include.
        // Corresponds to Go: data.Parent.Parent == nil → return true.
        let sd = SchemaData {
            types: vec!["Person".to_string()],
            data: Map::new(),
            importance: 0.0,
            parent_types: Some(vec!["Person".to_string()]),
            grandparent_types: None, // parent Person has no grandparent
        };
        assert!(schema_in_article(&sd, "person"));
    }

    #[test]
    fn test_schema_in_article_person_nested_in_person_in_article() {
        // Person → Person → Article: grandparent is Article → include.
        let sd = SchemaData {
            types: vec!["Person".to_string()],
            data: Map::new(),
            importance: 0.0,
            parent_types: Some(vec!["Person".to_string()]),
            grandparent_types: Some(vec!["Article".to_string()]),
        };
        assert!(schema_in_article(&sd, "person"));
    }

    #[test]
    fn test_schema_in_article_person_nested_in_person_not_in_article() {
        // Person → Person → WebSite: grandparent is not Article → exclude.
        let sd = SchemaData {
            types: vec!["Person".to_string()],
            data: Map::new(),
            importance: 0.0,
            parent_types: Some(vec!["Person".to_string()]),
            grandparent_types: Some(vec!["WebSite".to_string()]),
        };
        assert!(!schema_in_article(&sd, "person"));
    }

    #[test]
    fn test_extract_json_ld_nested_person_author() {
        // Person nested inside another Person at top level should still be extracted.
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@type":"Person","author":{"@type":"Person","name":"Nested Author"}}
            </script>
        </head><body></body></html>"#;
        let doc = Document::parse(html);
        let result = extract_json_ld(&Options::default(), &doc, Metadata::default());
        assert_eq!(result.author, "Nested Author");
    }
}
