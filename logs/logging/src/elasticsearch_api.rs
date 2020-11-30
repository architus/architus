/// Contains bindings for the elasticsearch search API,
/// used to make working with responses more ergonomic
/// Source: https://www.elastic.co/guide/en/elasticsearch/reference/7.10/search-search.html#search-api-response-body
pub mod search {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Response<T> {
        pub took: i64,
        pub timed_out: bool,
        #[serde(rename = "_shards")]
        pub shards: Shards,
        pub hits: Hits<T>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Shards {
        pub total: i64,
        pub successful: i64,
        pub skipped: i64,
        pub failed: i64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Hits<T> {
        pub total: HitsTotal,
        pub max_score: f64,
        pub hits: Vec<HitObject<T>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HitsTotal {
        pub value: i64,
        pub relation: HitsTotalRelation,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum HitsTotalRelation {
        #[serde(rename = "eq")]
        Accurate,
        #[serde(rename = "gte")]
        LowerBound,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HitObject<T> {
        #[serde(rename = "_index")]
        pub index: String,
        #[serde(rename = "_type")]
        pub hit_type: String,
        #[serde(rename = "_id")]
        pub id: String,
        #[serde(rename = "_score")]
        pub score: f64,
        #[serde(rename = "_source")]
        pub source: T,
    }
}
