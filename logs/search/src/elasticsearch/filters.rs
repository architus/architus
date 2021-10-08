use crate::elasticsearch::SearchParams;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FilterApplyError {
    #[error("serializing the given value to JSON failed for filter type {filter_type} on field {field_name}")]
    ValueSerializationError {
        #[source]
        inner: serde_json::Error,
        filter_type: &'static str,
        field_name: String,
    },
}

pub enum TermFilter {
    EqualTo,
    NotEqualTo,
}

impl TermFilter {
    pub fn apply<T>(
        &self,
        value_option: Option<T>,
        params: &mut SearchParams,
        field_name: impl AsRef<str>,
    ) -> Result<(), FilterApplyError>
    where
        T: Serialize,
    {
        let field_name = field_name.as_ref();
        if let Some(value) = value_option {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-term-query.html
            let owned_field_name = String::from(field_name);
            let value_json = serde_json::to_value(value).map_err(|err| {
                FilterApplyError::ValueSerializationError {
                    inner: err,
                    filter_type: "TermFilter",
                    field_name: String::from(field_name),
                }
            })?;
            let filter = serde_json::json!({
                "term": {
                    owned_field_name: {
                        "value": value_json,
                    }
                }
            });

            match self {
                Self::EqualTo => params.filters.push(filter),
                Self::NotEqualTo => params.anti_filters.push(filter),
            }
        }

        Ok(())
    }
}

pub enum RangeFilter {
    GreaterThan,
    GreaterThanOrEqualTo,
    LessThan,
    LessThanOrEqualTo,
}

impl RangeFilter {
    pub fn apply<T>(
        &self,
        value_option: Option<T>,
        params: &mut SearchParams,
        field_name: impl AsRef<str>,
    ) -> Result<(), FilterApplyError>
    where
        T: Serialize,
    {
        let field_name = field_name.as_ref();
        if let Some(value) = value_option {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            let owned_field_name = String::from(field_name);
            let value_json = serde_json::to_value(value).map_err(|err| {
                FilterApplyError::ValueSerializationError {
                    inner: err,
                    filter_type: "RangeFilter",
                    field_name: String::from(field_name),
                }
            })?;
            let inner_filter = match self {
                Self::GreaterThan => serde_json::json!({ "gt": value_json }),
                Self::GreaterThanOrEqualTo => serde_json::json!({ "gte": value_json }),
                Self::LessThan => serde_json::json!({ "lt": value_json }),
                Self::LessThanOrEqualTo => serde_json::json!({ "lte": value_json }),
            };

            params.filters.push(serde_json::json!({
                "range": {
                    owned_field_name: inner_filter,
                }
            }));
        }

        Ok(())
    }
}

pub enum SetFilter {
    In,
    NotIn,
}

impl SetFilter {
    pub fn apply<T>(
        &self,
        values_option: Option<Vec<T>>,
        params: &mut SearchParams,
        field_name: impl AsRef<str>,
    ) -> Result<(), FilterApplyError>
    where
        T: Serialize,
    {
        let field_name = field_name.as_ref();
        if let Some(values) = values_option {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-terms-query.html
            let owned_field_name = String::from(field_name);
            let values_json = serde_json::to_value(values).map_err(|err| {
                FilterApplyError::ValueSerializationError {
                    inner: err,
                    filter_type: "SetFilter",
                    field_name: String::from(field_name),
                }
            })?;
            let filter = serde_json::json!({
                "terms": {
                    owned_field_name: values_json,
                }
            });

            match self {
                Self::In => params.filters.push(filter),
                Self::NotIn => params.anti_filters.push(filter),
            }
        }

        Ok(())
    }
}

pub enum EmptyFilter {
    Empty,
    NotEmpty,
}

impl EmptyFilter {
    pub fn apply(
        &self,
        should_apply: bool,
        params: &mut SearchParams,
        field_name: impl AsRef<str>,
    ) {
        let field_name = field_name.as_ref();
        if should_apply {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-exists-query.html
            let owned_field_name = String::from(field_name);
            let filter = serde_json::json!({
                "exists": {
                    "field": owned_field_name,
                }
            });

            match self {
                Self::Empty => params.anti_filters.push(filter),
                Self::NotEmpty => params.filters.push(filter),
            }
        }
    }
}

pub enum IdFilter {
    EqualTo,
    NotEqualTo,
}

impl IdFilter {
    pub fn apply(&self, value_option: Option<String>, params: &mut SearchParams) {
        if let Some(value) = value_option {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-ids-query.html
            let filter = serde_json::json!({
                "ids": {
                    "values": [value],
                }
            });

            match self {
                Self::EqualTo => params.filters.push(filter),
                Self::NotEqualTo => params.anti_filters.push(filter),
            }
        }
    }
}

pub enum IdSetFilter {
    In,
    NotIn,
}

impl IdSetFilter {
    pub fn apply(&self, values_option: Option<Vec<String>>, params: &mut SearchParams) {
        if let Some(values) = values_option {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-ids-query.html
            let filter = serde_json::json!({
                "ids": {
                    "values": values,
                }
            });

            match self {
                Self::In => params.filters.push(filter),
                Self::NotIn => params.anti_filters.push(filter),
            }
        }
    }
}

pub struct WildcardFilter;

impl WildcardFilter {
    pub fn apply(
        &self,
        value_option: Option<String>,
        params: &mut SearchParams,
        field_name: impl AsRef<str>,
    ) {
        let field_name = field_name.as_ref();
        if let Some(value) = value_option {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-wildcard-query.html
            let owned_field_name = String::from(field_name);
            params.filters.push(serde_json::json!({
                "wildcard": {
                    owned_field_name: {
                        "value": value,
                    },
                }
            }));
        }
    }
}

pub enum ExactSetFilter {
    ContainsAll,
}

impl ExactSetFilter {
    pub fn apply<T>(
        &self,
        values_option: Option<Vec<T>>,
        params: &mut SearchParams,
        field_name: impl AsRef<str>,
    ) -> Result<(), FilterApplyError>
    where
        T: Serialize,
    {
        let field_name = field_name.as_ref();
        if let Some(values) = values_option {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-bool-query.html
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-term-query.html
            let term_filters = values
                .into_iter()
                .map(|v| {
                    let owned_field_name = String::from(field_name);
                    let v_json = serde_json::to_value(v).map_err(|err| {
                        FilterApplyError::ValueSerializationError {
                            inner: err,
                            filter_type: "ExactSetFilter",
                            field_name: String::from(field_name),
                        }
                    })?;
                    Ok(serde_json::json!({"term": {owned_field_name: v_json}}))
                })
                .collect::<Result<Vec<_>, _>>()?;

            params.filters.push(serde_json::json!({
                "bool": {
                    "filter": term_filters,
                },
            }));
        }

        Ok(())
    }
}

pub enum TextFilter {
    Query,
    Match,
}

impl TextFilter {
    pub fn apply(
        &self,
        value_option: Option<String>,
        params: &mut SearchParams,
        field_name: impl AsRef<str>,
    ) {
        let field_name = field_name.as_ref();
        if let Some(value) = value_option {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-match-query.html
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-simple-query-string-query.html
            let owned_field_name = String::from(field_name);
            let filter = match self {
                Self::Query => serde_json::json!({
                    "simple_query_string": {
                        "query": value,
                        "fields": [owned_field_name],
                    }
                }),
                Self::Match => serde_json::json!({
                    "match": {
                        owned_field_name: {
                            "query": value,
                        }
                    }
                }),
            };

            params.filters.push(filter);
        }
    }
}
