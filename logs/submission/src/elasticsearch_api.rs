//! Contains bindings for the elasticsearch API,
//! used to make working with responses more ergonomic

pub(crate) mod bulk {
    // Source: `https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html#bulk-api-response-body`
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Response {
        pub took: i64,
        pub errors: bool,
        pub items: Vec<ResultItem>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ResultItem {
        pub create: Option<ResultItemAction>,
        pub delete: Option<ResultItemAction>,
        pub index: Option<ResultItemAction>,
        pub update: Option<ResultItemAction>,
    }

    #[serde_with::serde_as]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ResultItemAction {
        #[serde(rename = "_index")]
        pub index: String,
        #[serde(rename = "_type")]
        pub r#type: String,
        #[serde(rename = "_id")]
        pub id: String,
        #[serde(rename = "_version")]
        pub version: Option<i64>,
        pub result: Option<String>,
        #[serde(rename = "_shards")]
        pub shards: Option<Shards>,
        #[serde(rename = "_seq_no")]
        pub seq_no: Option<i64>,
        #[serde(rename = "_primary_term")]
        pub primary_term: Option<i64>,
        #[serde(with = "serde_status_code")]
        pub status: hyper::StatusCode,
        pub error: Option<Error>,
    }

    mod serde_status_code {
        use serde::Deserialize;

        pub fn deserialize<'de, D>(deserializer: D) -> Result<hyper::http::StatusCode, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            hyper::StatusCode::from_u16(u16::deserialize(deserializer)?)
                .map_err(serde::de::Error::custom)
        }

        pub fn serialize<S>(
            status_code: &hyper::StatusCode,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_u16(status_code.as_u16())
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Shards {
        pub total: i64,
        pub successful: i64,
        pub failed: i64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Error {
        pub r#type: String,
        pub reason: String,
        pub index_uuid: String,
        pub shard: String,
        pub index: String,
    }

    /// From https://github.com/elastic/elasticsearch/blob/7.13/docs/reference/docs/bulk.asciidoc#api-examples-title
    #[test]
    fn response_deserializes_example_1() {
        serde_json::from_value::<Response>(serde_json::json!({
           "took": 30,
           "errors": false,
           "items": [
              {
                 "index": {
                    "_index": "test",
                    "_type": "_doc",
                    "_id": "1",
                    "_version": 1,
                    "result": "created",
                    "_shards": {
                       "total": 2,
                       "successful": 1,
                       "failed": 0
                    },
                    "status": 201,
                    "_seq_no" : 0,
                    "_primary_term": 1
                 }
              },
              {
                 "delete": {
                    "_index": "test",
                    "_type": "_doc",
                    "_id": "2",
                    "_version": 1,
                    "result": "not_found",
                    "_shards": {
                       "total": 2,
                       "successful": 1,
                       "failed": 0
                    },
                    "status": 404,
                    "_seq_no" : 1,
                    "_primary_term" : 2
                 }
              },
              {
                 "create": {
                    "_index": "test",
                    "_type": "_doc",
                    "_id": "3",
                    "_version": 1,
                    "result": "created",
                    "_shards": {
                       "total": 2,
                       "successful": 1,
                       "failed": 0
                    },
                    "status": 201,
                    "_seq_no" : 2,
                    "_primary_term" : 3
                 }
              },
              {
                 "update": {
                    "_index": "test",
                    "_type": "_doc",
                    "_id": "1",
                    "_version": 2,
                    "result": "updated",
                    "_shards": {
                        "total": 2,
                        "successful": 1,
                        "failed": 0
                    },
                    "status": 200,
                    "_seq_no" : 3,
                    "_primary_term" : 4
                 }
              }
           ]
        }))
        .unwrap();
    }

    /// From https://github.com/elastic/elasticsearch/blob/7.13/docs/reference/docs/bulk.asciidoc#example-with-failed-actions
    #[test]
    fn response_deserializes_example_2() {
        serde_json::from_value::<Response>(serde_json::json!({
          "took": 486,
          "errors": true,
          "items": [
            {
              "update": {
                "_index": "index1",
                "_type" : "_doc",
                "_id": "5",
                "status": 404,
                "error": {
                  "type": "document_missing_exception",
                  "reason": "[_doc][5]: document missing",
                  "index_uuid": "aAsFqTI0Tc2W0LCWgPNrOA",
                  "shard": "0",
                  "index": "index1"
                }
              }
            },
            {
              "update": {
                "_index": "index1",
                "_type" : "_doc",
                "_id": "6",
                "status": 404,
                "error": {
                  "type": "document_missing_exception",
                  "reason": "[_doc][6]: document missing",
                  "index_uuid": "aAsFqTI0Tc2W0LCWgPNrOA",
                  "shard": "0",
                  "index": "index1"
                }
              }
            },
            {
              "create": {
                "_index": "index1",
                "_type" : "_doc",
                "_id": "7",
                "_version": 1,
                "result": "created",
                "_shards": {
                  "total": 2,
                  "successful": 1,
                  "failed": 0
                },
                "_seq_no": 0,
                "_primary_term": 1,
                "status": 201
              }
            }
          ]
        }))
        .unwrap();
    }
}
