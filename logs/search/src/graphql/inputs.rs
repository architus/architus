use crate::graphql::{ElasticsearchParams, QueryInput};
use crate::rpc::logs::event::{AgentSpecialType, EntityType, EventOrigin, EventType};
use juniper::{graphql_value, FieldError, FieldResult};
use serde_json::json;
use std::str::FromStr;

/// Filter input object that allows for filtering the results of `allEvent`
#[derive(juniper::GraphQLInputObject)]
pub struct EventFilterInput {
    id: Option<IdFilterInput>,
    timestamp: Option<UIntStringFilterInput>,
    origin: Option<EnumFilterInput>,
    r#type: Option<EnumFilterInput>,
    guild_id: Option<UIntStringFilterInput>,
    reason: Option<TextOptionFilterInput>,
    audit_log_id: Option<UIntStringOptionFilterInput>,
    channel_id: Option<UIntStringOptionFilterInput>,
    agent_id: Option<UIntStringOptionFilterInput>,
    agent_type: Option<EnumFilterInput>,
    agent_special_type: Option<EnumFilterInput>,
    subject_id: Option<UIntStringOptionFilterInput>,
    subject_type: Option<EnumFilterInput>,
    auxiliary_id: Option<UIntStringOptionFilterInput>,
    auxiliary_type: Option<EnumFilterInput>,
    content: Option<TextOptionFilterInput>,
    // TODO implement content metadata filter
}

impl QueryInput for EventFilterInput {
    fn to_elasticsearch(&self, params: &mut ElasticsearchParams) -> FieldResult<()> {
        self.id.to_elasticsearch(params)?;
        self.timestamp
            .as_ref()
            .map(|filter| filter.apply(params, "inner.timestamp"))
            .transpose()?;
        self.origin
            .as_ref()
            .map(|filter| filter.apply::<EventOrigin>(params, "inner.origin"))
            .transpose()?;
        self.r#type
            .as_ref()
            .map(|filter| filter.apply::<EventType>(params, "inner.type"))
            .transpose()?;
        self.guild_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.guild_id"))
            .transpose()?;
        // TODO add reason filter application
        self.audit_log_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.audit_log_id"))
            .transpose()?;
        self.channel_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.channel_id"))
            .transpose()?;
        self.agent_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.agent_id"))
            .transpose()?;
        self.agent_type
            .as_ref()
            .map(|filter| filter.apply::<EntityType>(params, "inner.agent_type"))
            .transpose()?;
        self.agent_special_type
            .as_ref()
            .map(|filter| filter.apply::<AgentSpecialType>(params, "inner.agent_special_type"))
            .transpose()?;
        self.subject_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.subject_id"))
            .transpose()?;
        self.subject_type
            .as_ref()
            .map(|filter| filter.apply::<EntityType>(params, "inner.subject_type"))
            .transpose()?;
        self.auxiliary_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.auxiliary_id"))
            .transpose()?;
        self.auxiliary_type
            .as_ref()
            .map(|filter| filter.apply::<EntityType>(params, "inner.auxiliary_type"))
            .transpose()?;
        // TODO add content filter application
        // TODO add content metadata filter application

        Ok(())
    }
}

/// Sort input object that allows for sorting the results of `allEvent`
#[derive(juniper::GraphQLInputObject)]
pub struct EventSortInput {
    // TODO use real input fields
    id: Option<i32>,
}

impl QueryInput for EventSortInput {
    fn to_elasticsearch(&self, _params: &mut ElasticsearchParams) -> FieldResult<()> {
        // TODO implement
        Ok(())
    }
}

/// Parses a uint string into the actual u64 value, or returns a `FieldError`
fn parse_uint_string(s: impl AsRef<str>) -> FieldResult<u64> {
    let s = s.as_ref();
    s.parse::<u64>().map_err(|err| {
        let message = format!("Could not parse unsigned int from '{}'", s);
        let internal = format!("{:?}", err);
        FieldError::new(
            message,
            graphql_value!({
                "internal": internal,
            }),
        )
    })
}

/// Filter input object that allows for the filtering of an unsigned integer value
/// (represented in the schema as a String)
#[derive(juniper::GraphQLInputObject)]
pub struct UIntStringFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    gt: Option<String>,
    gte: Option<String>,
    lt: Option<String>,
    lte: Option<String>,
    #[graphql(name = "in")]
    in_set: Option<Vec<String>>,
    nin: Option<Vec<String>>,
}

impl UIntStringFilterInput {
    fn apply(
        &self,
        params: &mut ElasticsearchParams,
        field_name: impl AsRef<str>,
    ) -> FieldResult<()> {
        let field_name = field_name.as_ref();
        if let Some(eq) = self.eq.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-term-query.html
            params.add_filter(json!({
                "term": {
                    String::from(field_name): {
                        "value": eq,
                    }
                }
            }));
        }
        if let Some(ne) = self.ne.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-term-query.html
            params.add_anti_filter(json!({
                "term": {
                    String::from(field_name): {
                        "value": ne,
                    }
                }
            }));
        }
        if let Some(gt) = self.gt.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    String::from(field_name): {
                        "gt": gt,
                    }
                }
            }));
        }
        if let Some(gte) = self.gte.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    String::from(field_name): {
                        "gte": gte,
                    }
                }
            }));
        }
        if let Some(lt) = self.lt.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    String::from(field_name): {
                        "lt": lt,
                    }
                }
            }));
        }
        if let Some(lte) = self.lte.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    String::from(field_name): {
                        "lte": lte,
                    }
                }
            }));
        }
        if let Some(in_set) = self.in_set.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-terms-query.html
            let in_set = in_set
                .iter()
                .map(parse_uint_string)
                .collect::<FieldResult<Vec<_>>>()?;
            params.add_filter(json!({
                "terms": {
                    String::from(field_name): in_set,
                }
            }));
        }
        if let Some(nin) = self.nin.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-terms-query.html
            let nin = nin
                .iter()
                .map(parse_uint_string)
                .collect::<FieldResult<Vec<_>>>()?;
            params.add_anti_filter(json!({
                "terms": {
                    String::from(field_name): nin,
                }
            }));
        }

        Ok(())
    }
}

/// Filter input object that allows for the filtering of a document ID
/// (represented in the schema as a String)
#[derive(juniper::GraphQLInputObject)]
pub struct IdFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    gt: Option<String>,
    gte: Option<String>,
    lt: Option<String>,
    lte: Option<String>,
    #[graphql(name = "in")]
    in_set: Option<Vec<String>>,
    nin: Option<Vec<String>>,
}

impl QueryInput for IdFilterInput {
    fn to_elasticsearch(&self, params: &mut ElasticsearchParams) -> FieldResult<()> {
        if let Some(eq) = self.eq.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-ids-query.html
            params.add_filter(json!({
                "term": {
                    "ids": {
                        "values": [eq],
                    }
                }
            }));
        }
        if let Some(ne) = self.ne.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-ids-query.html
            params.add_anti_filter(json!({
                "term": {
                    "ids": {
                        "values": [ne],
                    }
                }
            }));
        }
        if let Some(gt) = self.gt.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    "id": {
                        "gt": gt,
                    }
                }
            }));
        }
        if let Some(gte) = self.gte.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    "id": {
                        "gte": gte,
                    }
                }
            }));
        }
        if let Some(lt) = self.lt.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    "id": {
                        "lt": lt,
                    }
                }
            }));
        }
        if let Some(lte) = self.lte.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    "id": {
                        "lte": lte,
                    }
                }
            }));
        }
        if let Some(in_set) = self.in_set.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-ids-query.html
            params.add_filter(json!({
                "term": {
                    "ids": {
                        "values": in_set,
                    }
                }
            }));
        }
        if let Some(nin) = self.nin.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-ids-query.html
            params.add_anti_filter(json!({
                "term": {
                    "ids": {
                        "values": nin,
                    }
                }
            }));
        }

        Ok(())
    }
}

/// Represents a trait over the numeric cast for field-less enum discriminants
trait FilterableEnum {
    fn discriminant(&self) -> i32;
}

impl FilterableEnum for EventType {
    fn discriminant(&self) -> i32 {
        *self as i32
    }
}

impl FilterableEnum for EventOrigin {
    fn discriminant(&self) -> i32 {
        *self as i32
    }
}

impl FilterableEnum for EntityType {
    fn discriminant(&self) -> i32 {
        *self as i32
    }
}

impl FilterableEnum for AgentSpecialType {
    fn discriminant(&self) -> i32 {
        *self as i32
    }
}

/// Filter input object that allows for the filtering of an enum
/// (represented in the schema as a String)
#[derive(juniper::GraphQLInputObject)]
pub struct EnumFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    #[graphql(name = "in")]
    in_set: Option<Vec<String>>,
    nin: Option<Vec<String>>,
}

impl EnumFilterInput {
    fn try_convert<A, T>(string: A) -> FieldResult<i32>
    where
        A: AsRef<str>,
        T: FilterableEnum + FromStr,
        <T as FromStr>::Err: std::fmt::Debug,
    {
        let string = string.as_ref();
        let value: Result<T, _> = FromStr::from_str(string);
        match value {
            Ok(value) => Ok(value.discriminant()),
            Err(err) => {
                let message = format!(
                    "Could not convert enum value '{}' to desired type '{}'",
                    string,
                    std::any::type_name::<T>()
                );
                let internal = format!("{:?}", err);
                Err(FieldError::new(
                    message,
                    graphql_value!({
                        "internal": internal,
                    }),
                ))
            }
        }
    }

    fn apply<T>(&self, params: &mut ElasticsearchParams, field_name: &str) -> FieldResult<()>
    where
        T: FilterableEnum + FromStr,
        <T as FromStr>::Err: std::fmt::Debug,
    {
        if let Some(eq) = self
            .eq
            .as_ref()
            .map(Self::try_convert::<_, T>)
            .transpose()?
        {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-term-query.html
            params.add_filter(json!({
                "term": {
                    String::from(field_name): {
                        "value": eq,
                    }
                }
            }));
        }
        if let Some(ne) = self
            .ne
            .as_ref()
            .map(Self::try_convert::<_, T>)
            .transpose()?
        {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-term-query.html
            params.add_anti_filter(json!({
                "term": {
                    String::from(field_name): {
                        "value": ne,
                    }
                }
            }));
        }
        if let Some(in_set) = self.in_set.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-terms-query.html
            let in_set = in_set
                .iter()
                .map(Self::try_convert::<_, T>)
                .collect::<FieldResult<Vec<_>>>()?;
            params.add_filter(json!({
                "terms": {
                    String::from(field_name): in_set,
                }
            }));
        }
        if let Some(nin) = self.nin.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-terms-query.html
            let nin = nin
                .iter()
                .map(Self::try_convert::<_, T>)
                .collect::<FieldResult<Vec<_>>>()?;
            params.add_anti_filter(json!({
                "terms": {
                    String::from(field_name): nin,
                }
            }));
        }

        Ok(())
    }
}

/// Filter input object that allows for the filtering of an optional unsigned integer value
/// (represented in the schema as a String)
#[derive(juniper::GraphQLInputObject)]
pub struct UIntStringOptionFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    gt: Option<String>,
    gte: Option<String>,
    lt: Option<String>,
    lte: Option<String>,
    #[graphql(name = "in")]
    in_set: Option<Vec<String>>,
    nin: Option<Vec<String>>,
    present: Option<bool>,
    absent: Option<bool>,
}

impl UIntStringOptionFilterInput {
    fn apply(
        &self,
        params: &mut ElasticsearchParams,
        field_name: impl AsRef<str>,
    ) -> FieldResult<()> {
        let field_name = field_name.as_ref();
        if let Some(eq) = self.eq.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-term-query.html
            params.add_filter(json!({
                "term": {
                    String::from(field_name): {
                        "value": eq,
                    }
                }
            }));
        }
        if let Some(ne) = self.ne.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-term-query.html
            params.add_anti_filter(json!({
                "term": {
                    String::from(field_name): {
                        "value": ne,
                    }
                }
            }));
        }
        if let Some(gt) = self.gt.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    String::from(field_name): {
                        "gt": gt,
                    }
                }
            }));
        }
        if let Some(gte) = self.gte.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    String::from(field_name): {
                        "gte": gte,
                    }
                }
            }));
        }
        if let Some(lt) = self.lt.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    String::from(field_name): {
                        "lt": lt,
                    }
                }
            }));
        }
        if let Some(lte) = self.lte.as_ref().map(parse_uint_string).transpose()? {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-range-query.html
            params.add_filter(json!({
                "range": {
                    String::from(field_name): {
                        "lte": lte,
                    }
                }
            }));
        }
        if let Some(in_set) = self.in_set.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-terms-query.html
            let in_set = in_set
                .iter()
                .map(parse_uint_string)
                .collect::<FieldResult<Vec<_>>>()?;
            params.add_filter(json!({
                "terms": {
                    String::from(field_name): in_set,
                }
            }));
        }
        if let Some(nin) = self.nin.as_ref() {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-terms-query.html
            let nin = nin
                .iter()
                .map(parse_uint_string)
                .collect::<FieldResult<Vec<_>>>()?;
            params.add_anti_filter(json!({
                "terms": {
                    String::from(field_name): nin,
                }
            }));
        }
        if let Some(present) = self.present {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-exists-query.html
            if present {
                params.add_filter(json!({
                    "exists": {
                        "field": String::from(field_name),
                    }
                }));
            }
        }
        if let Some(absent) = self.absent {
            // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-exists-query.html
            if absent {
                params.add_anti_filter(json!({
                    "exists": {
                        "field": String::from(field_name),
                    }
                }));
            }
        }

        Ok(())
    }
}

/// Filter input object that allows for the filtering of an optional text value
#[derive(juniper::GraphQLInputObject)]
pub struct TextOptionFilterInput {
    present: Option<bool>,
    absent: Option<bool>,
}

// TODO implement
